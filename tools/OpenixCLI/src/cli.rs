//! Command-line interface definitions
//!
//! Defines the CLI structure using clap for argument parsing

use clap::{Parser, Subcommand, ValueEnum};

/// Main CLI structure
///
/// # Fields
/// * `command` - The subcommand to execute (scan, flash, or tui). Defaults to TUI if none given.
/// * `verbose` - Enable verbose output
#[derive(Parser)]
#[command(name = "openixcli")]
#[command(about = "Firmware flashing CLI tool for Allwinner chips", long_about = None)]
#[command(version)]
pub struct Cli {
    /// The subcommand to execute (defaults to TUI if omitted)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    /// Machine-readable output format for automation.
    #[arg(long, global = true, value_enum, default_value = "text")]
    pub output: OutputFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Jsonl,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Scan for connected devices
    Scan {
        /// Get detailed device information (requires device initialization)
        #[arg(short = 'l', long, help = "Get detailed device information")]
        detailed: bool,
    },

    /// Flash firmware to device
    Flash {
        /// Path to firmware file
        #[arg(help = "Path to firmware file")]
        firmware: String,

        /// USB bus number
        #[arg(short, long, help = "USB bus number")]
        bus: Option<u8>,

        /// USB port number
        #[arg(short = 'P', long, help = "USB port number")]
        port: Option<u8>,

        /// Enable verification after write
        #[arg(
            short = 'V',
            long,
            default_value = "true",
            help = "Enable verification after write"
        )]
        verify: bool,

        /// Flash mode
        /// - partition: Flash only specified partitions
        /// - keep_data: Keep existing data
        /// - partition_erase: Erase partitions before flashing
        /// - full_erase: Erase all data before flashing
        #[arg(
            short,
            long,
            default_value = "full_erase",
            help = "Flash mode: partition, keep_data, partition_erase, full_erase"
        )]
        mode: String,

        /// Partitions to flash (comma-separated)
        #[arg(short = 'p', long, help = "Partitions to flash (comma-separated)")]
        partitions: Option<String>,

        /// Post-flash action
        /// - reboot: Reboot device after flashing
        /// - poweroff: Power off device after flashing
        /// - shutdown: Shutdown device after flashing
        #[arg(
            short = 'a',
            long,
            default_value = "reboot",
            help = "Post-flash action: reboot, poweroff, shutdown"
        )]
        post_action: String,

        /// Device reconnect timeout in seconds after U-Boot handoff
        #[arg(
            long,
            default_value_t = 90u64,
            help = "Reconnect timeout seconds after FEL->FES transition"
        )]
        reconnect_timeout_sec: u64,

        /// Poll interval in milliseconds while waiting device reconnect
        #[arg(
            long,
            default_value_t = 500u64,
            help = "Reconnect polling interval in milliseconds"
        )]
        reconnect_interval_ms: u64,
    },

    /// Execute a bounded, hash-verified mainline FEL RAM-boot plan.
    BootMainline {
        /// Absolute path to the validated JSON plan.
        #[arg(long)]
        plan: String,

        /// Stable libusb location in `libusb:BUS:PORT` form.
        #[arg(long)]
        device_location: String,

        /// Current USB bus resolved from the saved physical binding.
        #[arg(long)]
        bus: u8,

        /// Current USB port resolved from the saved physical binding.
        #[arg(long)]
        port: u8,
    },

    /// Provision NAND/SPI-NAND through a board-matched RAM-only FES loader.
    FlashNandComponents {
        /// Component manifest. Artifact paths are resolved relative to this file.
        #[arg(long)]
        manifest: String,

        /// Stable libusb location in `libusb:BUS:PORT` form.
        #[arg(long)]
        device_location: String,

        /// Current USB bus resolved from the saved physical binding.
        #[arg(long)]
        bus: u8,

        /// Current USB port resolved from the saved physical binding.
        #[arg(long)]
        port: u8,

        /// NAND erase policy. Only partition_erase and full_erase are accepted.
        #[arg(long, default_value = "full_erase")]
        mode: String,

        /// Require component verification through FES.
        #[arg(long, default_value = "true")]
        verify: bool,

        /// Leave the board in FES by default so cold boot is a separate gate.
        #[arg(long, default_value = "none")]
        post_action: String,

        #[arg(long, default_value_t = 90u64)]
        reconnect_timeout_sec: u64,

        #[arg(long, default_value_t = 500u64)]
        reconnect_interval_ms: u64,

        /// Validate the package and execution plan without opening USB.
        #[arg(long)]
        preflight_only: bool,
    },

    /// Launch interactive TUI mode
    Tui,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, OutputFormat};
    use clap::Parser;

    #[test]
    fn parses_scoped_mainline_worker_plan() {
        let cli = Cli::try_parse_from([
            "openixcli",
            "--output",
            "jsonl",
            "boot-mainline",
            "--plan",
            "/tmp/plan.json",
            "--device-location",
            "libusb:3:2",
            "--bus",
            "3",
            "--port",
            "2",
        ])
        .unwrap();
        assert_eq!(cli.output, OutputFormat::Jsonl);
        assert!(matches!(
            cli.command,
            Some(Commands::BootMainline {
                bus: 3,
                port: 2,
                ..
            })
        ));
    }

    #[test]
    fn text_output_remains_default() {
        let cli = Cli::try_parse_from(["openixcli", "scan"]).unwrap();
        assert_eq!(cli.output, OutputFormat::Text);
    }

    #[test]
    fn parses_explicit_nand_component_route() {
        let cli = Cli::try_parse_from([
            "openixcli",
            "--output",
            "jsonl",
            "flash-nand-components",
            "--manifest",
            "/tmp/fes/manifest.json",
            "--device-location",
            "libusb:3:2",
            "--bus",
            "3",
            "--port",
            "2",
            "--mode",
            "partition_erase",
            "--post-action",
            "none",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::FlashNandComponents {
                bus: 3,
                port: 2,
                preflight_only: false,
                ..
            })
        ));
    }
}
