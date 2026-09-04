//! Flash module
//!
//! Provides flash functionality for writing firmware to Allwinner devices
//! Supports both FEL mode (USB boot) and FES mode (U-Boot)

#![allow(dead_code)]

pub mod fel_handler;
pub mod fes_handler;

pub use fel_handler::FelHandler;
pub use fes_handler::FesHandler;

use crate::firmware::OpenixPacker;
use crate::process::{FlashStages, StageType};
use crate::utils::{FlashError, FlashResult, Logger};

/// Flash mode options
///
/// # Variants
/// * `Partition` - Flash only specified partitions
/// * `KeepData` - Keep existing data
/// * `PartitionErase` - Erase partitions before flashing
/// * `FullErase` - Erase all data before flashing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlashMode {
    Partition,
    KeepData,
    PartitionErase,
    FullErase,
}

impl FlashMode {
    /// Get erase flag for this mode
    pub fn erase_flag(&self) -> u32 {
        match self {
            FlashMode::Partition => 0x0,
            FlashMode::KeepData => 0x0,
            FlashMode::PartitionErase => 0x1,
            FlashMode::FullErase => 0x12,
        }
    }
}

/// Flash options configuration
///
/// # Fields
/// * `bus` - USB bus number (optional)
/// * `port` - USB port number (optional)
/// * `verify` - Enable verification after write
/// * `mode` - Flash mode
/// * `partitions` - Specific partitions to flash (optional)
/// * `post_action` - Action after flashing
#[derive(Debug, Clone)]
pub struct FlashOptions {
    pub bus: Option<u8>,
    pub port: Option<u8>,
    pub verify: bool,
    pub mode: FlashMode,
    pub partitions: Option<Vec<String>>,
    pub post_action: String,
    pub reconnect_timeout_sec: u64,
    pub reconnect_interval_ms: u64,
    /// Present only for the explicit NAND component route.
    pub nand_constraints: Option<NandConstraints>,
}

#[derive(Debug, Clone)]
pub struct NandConstraints {
    pub expected_capacity_bytes: u64,
    pub minimum_logical_sectors: u64,
    pub allow_unavailable_capacity: bool,
    pub expected_partitions: Vec<String>,
    pub expected_ubifs_partition: Option<String>,
    pub exact_bus: u8,
    pub exact_port: u8,
    pub disable_fes_retry: bool,
}

/// Main flash controller
///
/// Coordinates the flashing process including FEL initialization,
/// FES handling, and partition flashing
pub struct Flasher {
    packer: OpenixPacker,
    ram_only_bootstrap: Option<OpenixPacker>,
    options: FlashOptions,
    logger: Logger,
}

impl Flasher {
    /// Create a new flasher instance
    pub fn new(packer: OpenixPacker, options: FlashOptions, logger: Logger) -> Self {
        Self {
            packer,
            ram_only_bootstrap: None,
            options,
            logger,
        }
    }

    /// Use a separate board-matched IMAGEWTY package only for FEL DRAM/FES
    /// bootstrap. Persistent components continue to come from `self.packer`.
    pub fn with_ram_only_bootstrap(mut self, bootstrap: OpenixPacker) -> Self {
        self.ram_only_bootstrap = Some(bootstrap);
        self
    }

    /// Execute the flash process
    ///
    /// This is the main entry point for the flashing process.
    /// It handles both FEL and FES mode devices.
    pub async fn execute(&mut self) -> FlashResult<()> {
        let fes_data = match self.ram_only_bootstrap.as_mut() {
            Some(bootstrap) => bootstrap.get_fes(),
            None => self.packer.get_fes(),
        }
        .map_err(|_| FlashError::FesNotFound)?;

        let mut ctx = if let (Some(bus), Some(port)) = (self.options.bus, self.options.port) {
            let mut ctx = libefex::Context::new();
            ctx.scan_usb_device_at(bus, port)
                .map_err(|e| FlashError::DeviceOpenFailed(e.to_string()))?;
            ctx
        } else {
            let devices = libefex::Context::scan_usb_devices()
                .map_err(|e| FlashError::DeviceOpenFailed(e.to_string()))?;

            if devices.is_empty() {
                return Err(FlashError::DeviceNotFound);
            }

            let mut ctx = libefex::Context::new();
            ctx.scan_usb_device_at(devices[0].bus, devices[0].port)
                .map_err(|e| FlashError::DeviceOpenFailed(e.to_string()))?;
            ctx
        };

        ctx.usb_init()
            .map_err(|e| FlashError::DeviceOpenFailed(e.to_string()))?;

        ctx.efex_init()
            .map_err(|e| FlashError::DeviceOpenFailed(e.to_string()))?;

        let mode = ctx.get_device_mode();
        self.logger.info(&format!("Device mode: {:?}", mode));

        let has_fel = mode == libefex::DeviceMode::Fel;

        if has_fel {
            let stages = FlashStages::for_fel_mode();
            self.logger.define_stages(stages.stages());
        } else {
            let stages = FlashStages::for_fes_mode();
            self.logger.define_stages(stages.stages());
        }

        self.logger.start_global_progress();

        self.logger.begin_stage(StageType::Init);
        self.logger
            .info(&format!("FES data loaded ({} bytes)", fes_data.len()));
        self.logger.complete_stage();

        if has_fel {
            self.logger.begin_stage(StageType::FelDram);
            let fel_handler = FelHandler::new(&self.logger);
            fel_handler.handle(&mut ctx, &fes_data).await?;
            self.logger.complete_stage();

            self.logger.begin_stage(StageType::FelUboot);

            let (uboot_data, dtb_data, sysconfig_data, board_config_data) = {
                let bootstrap = self.ram_only_bootstrap.as_mut().unwrap_or(&mut self.packer);
                (
                    bootstrap
                        .get_uboot()
                        .map_err(|_| FlashError::UbootNotFound)?,
                    bootstrap.get_dtb().ok(),
                    bootstrap
                        .get_sys_config_bin()
                        .map_err(|_| FlashError::SysConfigNotFound)?,
                    bootstrap.get_board_config().ok(),
                )
            };

            fel_handler
                .download_uboot(
                    &ctx,
                    &uboot_data,
                    dtb_data.as_deref(),
                    &sysconfig_data,
                    board_config_data.as_deref(),
                )
                .await?;

            self.logger
                .info(&format!("U-Boot downloaded ({} bytes)", uboot_data.len()));
            self.logger.complete_stage();

            self.logger.begin_stage(StageType::FelReconnect);

            ctx = self.reconnect_device().await?;

            self.logger.complete_stage();
        }

        let mut fes_handler = FesHandler::new(&mut self.logger);
        let fes_result = fes_handler
            .handle(&ctx, &mut self.packer, &self.options)
            .await;
        if let Err(first_err) = fes_result {
            let retry_disabled = self
                .options
                .nand_constraints
                .as_ref()
                .is_some_and(|value| value.disable_fes_retry);
            if has_fel && !retry_disabled && Self::is_retryable_fes_error(&first_err) {
                self.logger.warn(&format!(
                    "FES first handshake failed: {}. Reconnecting and retrying once...",
                    first_err
                ));
                self.logger.begin_stage(StageType::FelReconnect);
                ctx = self.reconnect_device().await?;
                self.logger.complete_stage();

                let mut retry_handler = FesHandler::new(&mut self.logger);
                retry_handler
                    .handle(&ctx, &mut self.packer, &self.options)
                    .await?;
            } else {
                return Err(first_err);
            }
        }

        self.logger.finish_progress();

        self.set_device_mode(&ctx).await?;

        self.logger
            .stage_complete(&format!("Device will {}", self.options.post_action));

        Ok(())
    }

    /// Reconnect to device after FEL mode operations
    async fn reconnect_device(&self) -> FlashResult<libefex::Context> {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(self.options.reconnect_timeout_sec);
        let mut attempts: u64 = 0;
        let interval = tokio::time::Duration::from_millis(self.options.reconnect_interval_ms);

        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(interval).await;
            attempts += 1;

            let devices = match libefex::Context::scan_usb_devices() {
                Ok(d) => d,
                Err(_) => {
                    self.logger
                        .debug(&format!("Reconnect attempt {} (scan failed)", attempts));
                    continue;
                }
            };

            for dev in devices {
                if let Some(binding) = &self.options.nand_constraints {
                    if dev.bus != binding.exact_bus || dev.port != binding.exact_port {
                        continue;
                    }
                }
                let mut new_ctx = libefex::Context::new();
                if new_ctx.scan_usb_device_at(dev.bus, dev.port).is_err() {
                    continue;
                }
                if new_ctx.usb_init().is_err() {
                    continue;
                }
                if new_ctx.efex_init().is_err() {
                    self.logger.debug(&format!(
                        "Reconnect attempt {}: efex init failed at bus {}, port {}",
                        attempts, dev.bus, dev.port
                    ));
                    continue;
                }

                let mode = new_ctx.get_device_mode();
                self.logger.debug(&format!(
                    "Reconnect attempt {}: bus {}, port {}, mode {:?}",
                    attempts, dev.bus, dev.port, mode
                ));
                if mode == libefex::DeviceMode::Srv {
                    return Ok(new_ctx);
                }
            }
        }

        Err(FlashError::ReconnectFailed)
    }

    /// Set device mode after flashing
    async fn set_device_mode(&self, ctx: &libefex::Context) -> FlashResult<()> {
        if self.options.post_action == "none" {
            self.logger.info("Post action: none; leaving device in FES");
            return Ok(());
        }
        let tool_mode = match self.options.post_action.as_str() {
            "reboot" => libefex::FesToolMode::Reboot,
            "poweroff" => libefex::FesToolMode::PowerOff,
            "shutdown" => libefex::FesToolMode::PowerOff,
            _ => libefex::FesToolMode::Reboot,
        };

        ctx.fes_tool_mode(libefex::FesToolMode::Normal, tool_mode)
            .map_err(|e| FlashError::UsbTransferError(e.to_string()))?;

        Ok(())
    }

    fn is_retryable_fes_error(err: &FlashError) -> bool {
        matches!(
            err,
            FlashError::UsbTransferError(_) | FlashError::DeviceOpenFailed(_)
        )
    }
}
