use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{bail, Context as _};
use libefex::Context;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

// Keep the helper's independent trust boundary no broader than LYNX's
// validated mainline plan. The CLI remains safe even when invoked directly.
const MAX_ARTIFACTS: usize = 8;
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 768 * 1024 * 1024;
const MAX_PLAN_FILE_BYTES: u64 = 256 * 1024;
// Keep each FEL command below the 64 KiB protocol maximum and below the
// transfer size that legacy Windows Allwinner drivers have been observed to
// leave pending indefinitely. Each completed chunk becomes a cancellation and
// progress boundary visible to LYNX.
const FEL_CHUNK_SIZE: usize = 16 * 1024;
const R528_DEVICE_ID: u32 = 0x0018_5900;
const R528_SPL_ADDRESS: u32 = 0x0002_0000;
const R528_SCRATCH_ADDRESS: u32 = 0x0002_1000;
const R528_THUNK_ADDRESS: u32 = 0x0003_a200;
const R528_SPL_MAX_BYTES: usize = 160 * 1024;
const R528_SWAP_SOURCE: u32 = 0x0002_1000;
const R528_SWAP_BACKUP: u32 = 0x0003_8000;
const R528_SWAP_BYTES: usize = 0x1000;
const SPL_RETURN_MARKER: &[u8; 8] = b"eGON.FEL";

// Audited FEL-to-SPL return thunk machine words. The thunk preserves the
// BootROM stack and CPSR, rejects unexpected
// cache state, verifies the SPL checksum, swaps the SoC-specific SRAM window,
// and restores every item before returning to FEL. `r528_spl_thunk` appends
// the parameters required by the R528 SRAM layout.
const FEL_TO_SPL_THUNK_CODE: &[u32] = &[
    0xea000015,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xe1a00000,
    0xe28f40e8,
    0xe4940004,
    0xe4941004,
    0xe4946004,
    0xe3560000,
    0x012fff1e,
    0xe5902000,
    0xe5913000,
    0xe2566004,
    0xe4812004,
    0xe4803004,
    0x1afffff9,
    0xeafffff3,
    0xe59f80b0,
    0xe24f0044,
    0xe520d004,
    0xe1a0d000,
    0xe10f2000,
    0xe92d4004,
    0xe38220c0,
    0xe121f002,
    0xee112f10,
    0xe3120004,
    0x03120a01,
    0x1a000013,
    0xebffffe5,
    0xe59f706c,
    0xe1a00008,
    0xe5905010,
    0xe4902004,
    0xe2555004,
    0xe0877002,
    0x1afffffb,
    0xe598200c,
    0xe0577082,
    0x1a00000b,
    0xe59f2048,
    0xe5882008,
    0xee072f9a,
    0xee102f10,
    0xe202280f,
    0xe3520806,
    0xce072f95,
    0xe12fff38,
    0xea000004,
    0xe59f2028,
    0xe5882008,
    0xea000002,
    0xe59f2020,
    0xe5882008,
    0xebffffcc,
    0xe8bd4004,
    0xe121f002,
    0xe59dd000,
    0xe12fff1e,
    0x5f0a6c39,
    0x4c45462e,
    0x3f3f3f2e,
    0x4441422e,
];

// R528 requires the instruction cache to be disabled before replacing the
// BootROM SRAM code window. This is an independently assembled, fixed ARMv7
// sequence: clear SCTLR.I, invalidate I-cache, ISB, return.
const R528_DISABLE_ICACHE: &[u32] = &[
    0xee110f10, 0xe3c00a01, 0xee010f10, 0xee070f15, 0xf57ff06f, 0xe12fff1e,
];
static NEXT_SNAPSHOT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MainlinePlan {
    artifacts: Vec<MainlineArtifact>,
    entry_address: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MainlineArtifact {
    role: MainlineRole,
    file_path: PathBuf,
    load_address: u32,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MainlineRole {
    Spl,
    Bootloader,
    Kernel,
    DeviceTree,
    Initramfs,
    TrustedFirmware,
}

fn emit(jsonl: bool, value: serde_json::Value) {
    if jsonl {
        println!("{value}");
    } else if let Some(phase) = value.get("phase").and_then(serde_json::Value::as_str) {
        println!("{phase}");
    }
    let _ = std::io::stdout().flush();
}

fn cancellation_requested() -> bool {
    std::env::var_os("LYNX_FLASH_CANCEL_FILE")
        .map(PathBuf::from)
        .is_some_and(|path| path.is_file())
}

fn ensure_not_cancelled() -> anyhow::Result<()> {
    if cancellation_requested() {
        bail!("TASK_CANCELLED:mainline FEL boot cancelled by operator");
    }
    Ok(())
}

#[derive(Debug)]
struct ValidatedArtifact {
    snapshot_path: PathBuf,
    snapshot: Option<File>,
    size: u64,
    spl_length: Option<usize>,
}

impl Drop for ValidatedArtifact {
    fn drop(&mut self) {
        drop(self.snapshot.take());
        let _ = std::fs::remove_file(&self.snapshot_path);
    }
}

fn create_snapshot() -> anyhow::Result<(PathBuf, File)> {
    for _ in 0..16 {
        let id = NEXT_SNAPSHOT_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "openix-mainline-{}-{timestamp:032x}-{id:016x}.bin",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("MAINLINE_SNAPSHOT_CREATE"),
        }
    }
    bail!("MAINLINE_SNAPSHOT_CREATE:unique path exhausted")
}

fn read_u32_le(bytes: &[u8], offset: usize) -> anyhow::Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .context("MAINLINE_SPL_HEADER_TRUNCATED")?;
    Ok(u32::from_le_bytes(raw.try_into().expect("four-byte slice")))
}

fn validate_mainline_spl(bytes: &[u8]) -> anyhow::Result<usize> {
    if bytes.len() < 0x20 || bytes.get(4..12) != Some(b"eGON.BT0") {
        bail!("MAINLINE_SPL_EGON_REQUIRED");
    }
    if bytes.get(0x14..0x17) != Some(b"SPL") {
        bail!("MAINLINE_SPL_SIGNATURE_REQUIRED:vendor Boot0 is not accepted");
    }
    let version = bytes[0x17];
    if !(1..=31).contains(&version) {
        bail!("MAINLINE_SPL_VERSION_UNSUPPORTED:{version}");
    }
    let length =
        usize::try_from(read_u32_le(bytes, 0x10)?).context("MAINLINE_SPL_LENGTH_OVERFLOW")?;
    if !(0x20..=R528_SPL_MAX_BYTES).contains(&length) || !length.is_multiple_of(4) {
        bail!("MAINLINE_SPL_LENGTH_INVALID:{length}");
    }
    if length != bytes.len() {
        bail!(
            "MAINLINE_SPL_EXACT_FILE_REQUIRED:header={length}:file={}",
            bytes.len()
        );
    }

    let stored_checksum = read_u32_le(bytes, 0x0c)?;
    let expected_sum = stored_checksum.wrapping_mul(2).wrapping_sub(0x5f0a_6c39);
    let actual_sum = bytes[..length].chunks_exact(4).fold(0_u32, |sum, word| {
        sum.wrapping_add(u32::from_le_bytes(
            word.try_into().expect("four-byte SPL word"),
        ))
    });
    if actual_sum != expected_sum {
        bail!("MAINLINE_SPL_CHECKSUM_INVALID");
    }
    Ok(length)
}

fn validate_artifact(artifact: &MainlineArtifact) -> anyhow::Result<ValidatedArtifact> {
    if !artifact.file_path.is_absolute()
        || artifact.load_address < 0x1_000
        || !artifact.load_address.is_multiple_of(4)
    {
        bail!("MAINLINE_PLAN_INVALID:artifact path/address");
    }
    if artifact.sha256.len() != 64 || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("MAINLINE_PLAN_INVALID:sha256");
    }
    let metadata = artifact
        .file_path
        .metadata()
        .context("MAINLINE_ARTIFACT_METADATA")?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ARTIFACT_BYTES {
        bail!("MAINLINE_PLAN_INVALID:artifact size");
    }
    let mut source = File::open(&artifact.file_path).context("MAINLINE_ARTIFACT_OPEN")?;
    let (snapshot_path, mut snapshot) = create_snapshot()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; FEL_CHUNK_SIZE];
    let mut size = 0_u64;
    loop {
        let read = source.read(&mut buffer).context("MAINLINE_ARTIFACT_READ")?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .context("MAINLINE_ARTIFACT_SIZE_OVERFLOW")?;
        if size > MAX_ARTIFACT_BYTES {
            bail!("MAINLINE_PLAN_INVALID:artifact size");
        }
        hasher.update(&buffer[..read]);
        snapshot
            .write_all(&buffer[..read])
            .context("MAINLINE_SNAPSHOT_WRITE")?;
    }
    if size == 0 {
        bail!("MAINLINE_PLAN_INVALID:artifact size");
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        bail!("MAINLINE_ARTIFACT_HASH_MISMATCH");
    }
    let end = u64::from(artifact.load_address)
        .checked_add(size)
        .context("MAINLINE_ADDRESS_RANGE_OVERFLOW")?;
    if end > u64::from(u32::MAX) + 1 {
        bail!("MAINLINE_ADDRESS_RANGE_OVERFLOW");
    }
    snapshot.flush().context("MAINLINE_SNAPSHOT_FLUSH")?;
    snapshot
        .seek(SeekFrom::Start(0))
        .context("MAINLINE_SNAPSHOT_REWIND")?;
    let spl_length = if matches!(artifact.role, MainlineRole::Spl) {
        let mut bytes = Vec::with_capacity(size as usize);
        snapshot
            .read_to_end(&mut bytes)
            .context("MAINLINE_SPL_READ")?;
        snapshot
            .seek(SeekFrom::Start(0))
            .context("MAINLINE_SNAPSHOT_REWIND")?;
        Some(validate_mainline_spl(&bytes)?)
    } else {
        None
    };
    Ok(ValidatedArtifact {
        snapshot_path,
        snapshot: Some(snapshot),
        size,
        spl_length,
    })
}

fn validate_plan(plan: &MainlinePlan) -> anyhow::Result<Vec<ValidatedArtifact>> {
    if plan.artifacts.is_empty() || plan.artifacts.len() > MAX_ARTIFACTS {
        bail!("MAINLINE_PLAN_INVALID:artifact count");
    }
    if !plan
        .artifacts
        .iter()
        .any(|artifact| artifact.load_address == plan.entry_address)
    {
        bail!("MAINLINE_ENTRY_NOT_DECLARED");
    }
    let spl_artifacts = plan
        .artifacts
        .iter()
        .filter(|artifact| matches!(artifact.role, MainlineRole::Spl))
        .collect::<Vec<_>>();
    if spl_artifacts.len() != 1 {
        bail!("ALLWINNER_MAINLINE_SPL_REQUIRED:expected exactly one mainline U-Boot SPL");
    }
    if !matches!(plan.artifacts[0].role, MainlineRole::Spl) {
        bail!("MAINLINE_SPL_MUST_BE_FIRST");
    }
    if spl_artifacts[0].load_address != R528_SPL_ADDRESS {
        bail!("MAINLINE_SPL_ADDRESS_INVALID:expected 0x{R528_SPL_ADDRESS:08x}");
    }
    if plan.entry_address == R528_SPL_ADDRESS {
        bail!("MAINLINE_ENTRY_SPL_FORBIDDEN:entry must be U-Boot proper or installer");
    }
    let bootloaders = plan
        .artifacts
        .iter()
        .filter(|artifact| matches!(artifact.role, MainlineRole::Bootloader))
        .collect::<Vec<_>>();
    if bootloaders.len() != 1 {
        bail!("MAINLINE_BOOTLOADER_REQUIRED:expected exactly one U-Boot proper");
    }
    if bootloaders[0].load_address != plan.entry_address {
        bail!("MAINLINE_ENTRY_MUST_BE_BOOTLOADER");
    }
    let mut total = 0_u64;
    let mut validated: Vec<ValidatedArtifact> = Vec::with_capacity(plan.artifacts.len());
    for artifact in &plan.artifacts {
        let validated_artifact = validate_artifact(artifact)?;
        let size = validated_artifact.size;
        total = total
            .checked_add(size)
            .context("MAINLINE_TOTAL_SIZE_OVERFLOW")?;
        if total > MAX_TOTAL_BYTES {
            bail!("MAINLINE_PLAN_INVALID:total size");
        }
        let start = u64::from(artifact.load_address);
        let end = start + size;
        if plan
            .artifacts
            .iter()
            .zip(validated.iter())
            .any(|(existing, validated_existing)| {
                let existing_start = u64::from(existing.load_address);
                let existing_end = existing_start + validated_existing.size;
                start < existing_end && existing_start < end
            })
        {
            bail!("MAINLINE_ARTIFACT_ADDRESS_OVERLAP");
        }
        validated.push(validated_artifact);
    }
    Ok(validated)
}

trait FelTransport {
    fn device_id(&self) -> u32;
    fn write(&self, address: u32, bytes: &[u8]) -> anyhow::Result<()>;
    fn read(&self, address: u32, bytes: &mut [u8]) -> anyhow::Result<()>;
    fn execute(&self, address: u32) -> anyhow::Result<()>;
}

impl FelTransport for Context {
    fn device_id(&self) -> u32 {
        // libefex exposes the opened endpoint response through its stable C
        // context. Keep this read local until the public crate grows a
        // dedicated accessor.
        unsafe { (*self.as_ptr()).resp.id }
    }

    fn write(&self, address: u32, bytes: &[u8]) -> anyhow::Result<()> {
        self.fel_write(address, bytes).map_err(Into::into)
    }

    fn read(&self, address: u32, bytes: &mut [u8]) -> anyhow::Result<()> {
        self.fel_read(address, bytes).map_err(Into::into)
    }

    fn execute(&self, address: u32) -> anyhow::Result<()> {
        self.fel_exec(address).map_err(Into::into)
    }
}

fn words_as_le_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn r528_spl_thunk() -> Vec<u8> {
    let mut words = FEL_TO_SPL_THUNK_CODE.to_vec();
    words.extend_from_slice(&[
        R528_SPL_ADDRESS,
        R528_SWAP_SOURCE,
        R528_SWAP_BACKUP,
        R528_SWAP_BYTES as u32,
        0,
        0,
        0,
    ]);
    words_as_le_bytes(&words)
}

fn write_and_verify<T: FelTransport>(
    transport: &T,
    address: u32,
    bytes: &[u8],
    stage: &str,
) -> anyhow::Result<()> {
    transport
        .write(address, bytes)
        .with_context(|| format!("MAINLINE_SPL_WRITE_FAILED:{stage}:address=0x{address:08x}"))?;
    let mut readback = vec![0_u8; bytes.len()];
    transport
        .read(address, &mut readback)
        .with_context(|| format!("MAINLINE_SPL_READBACK_FAILED:{stage}:address=0x{address:08x}"))?;
    if readback != bytes {
        bail!("MAINLINE_SPL_READBACK_MISMATCH:{stage}:address=0x{address:08x}");
    }
    Ok(())
}

fn execute_r528_mainline_spl<T: FelTransport>(transport: &T, spl: &[u8]) -> anyhow::Result<()> {
    if transport.device_id() != R528_DEVICE_ID {
        bail!(
            "ALLWINNER_MAINLINE_SOC_UNSUPPORTED:expected=0x{R528_DEVICE_ID:08x}:actual=0x{:08x}",
            transport.device_id()
        );
    }
    validate_mainline_spl(spl)?;

    // The first SRAM write disables I-cache so a stale BootROM instruction
    // line cannot execute after the thunk is installed.
    let disable_icache = words_as_le_bytes(R528_DISABLE_ICACHE);
    transport
        .write(R528_SCRATCH_ADDRESS, &disable_icache)
        .context("MAINLINE_SPL_ICACHE_SETUP_FAILED")?;
    transport
        .execute(R528_SCRATCH_ADDRESS)
        .context("MAINLINE_SPL_ICACHE_DISABLE_FAILED")?;

    let swap_offset = usize::try_from(R528_SWAP_SOURCE - R528_SPL_ADDRESS)
        .expect("R528 SRAM addresses are ordered");
    let swap_end = swap_offset + R528_SWAP_BYTES;
    if spl.len() < swap_end {
        bail!("MAINLINE_SPL_LENGTH_INVALID:missing R528 swap window");
    }
    write_and_verify(
        transport,
        R528_SPL_ADDRESS,
        &spl[..swap_offset],
        "sram-head",
    )?;
    write_and_verify(
        transport,
        R528_SWAP_BACKUP,
        &spl[swap_offset..swap_end],
        "sram-swap-backup",
    )?;
    write_and_verify(
        transport,
        R528_SWAP_SOURCE + R528_SWAP_BYTES as u32,
        &spl[swap_end..],
        "sram-tail",
    )?;

    let thunk = r528_spl_thunk();
    if thunk.len() > 0x200 {
        bail!("MAINLINE_SPL_THUNK_TOO_LARGE:{}", thunk.len());
    }
    write_and_verify(transport, R528_THUNK_ADDRESS, &thunk, "return-thunk")?;
    transport
        .execute(R528_THUNK_ADDRESS)
        .context("MAINLINE_SPL_EXECUTE_FAILED")?;

    Ok(())
}

fn verify_r528_spl_return<T: FelTransport>(transport: &T) -> anyhow::Result<()> {
    let mut marker = [0_u8; 8];
    transport
        .read(R528_SPL_ADDRESS + 4, &mut marker)
        .context("MAINLINE_SPL_RETURN_READ_FAILED")?;
    if &marker != SPL_RETURN_MARKER {
        bail!(
            "MAINLINE_SPL_DID_NOT_RETURN_TO_FEL:marker={}",
            marker
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
    }
    Ok(())
}

fn validate_libusb_location(device_location: &str, bus: u8, port: u8) -> anyhow::Result<()> {
    let expected = format!("libusb:{bus}:{port}");
    if !device_location.eq_ignore_ascii_case(&expected) {
        bail!("FLASH_TARGET_REENUMERATED:binding={device_location}:current={expected}");
    }
    Ok(())
}

fn validated_endpoint(devices: &[libefex::ScannedDevice], bus: u8, port: u8) -> anyhow::Result<()> {
    let matches = devices
        .iter()
        .filter(|device| device.bus == bus && device.port == port)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [device] if device.vid == 0x1f3a && device.pid == 0xefe8 => Ok(()),
        [_] => bail!("FLASH_TARGET_PROTOCOL_MISMATCH:expected Allwinner FEL 1f3a:efe8"),
        [_, _, ..] => bail!("FLASH_TARGET_LOCATION_AMBIGUOUS:bus/port is not unique"),
        [] => bail!("FLASH_TARGET_NOT_FOUND:bound physical location is offline"),
    }
}

fn reconnected_endpoint(
    devices: &[libefex::ScannedDevice],
    bus: u8,
    port: u8,
) -> anyhow::Result<bool> {
    let matches = devices
        .iter()
        .filter(|device| {
            device.vid == 0x1f3a && device.pid == 0xefe8 && device.bus == bus && device.port == port
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [_] => Ok(true),
        [_, _, ..] => {
            bail!("MAINLINE_SPL_RETURN_LOCATION_AMBIGUOUS:physical location is not unique")
        }
        [] => Ok(false),
    }
}

fn reconnect_mainline_fel(
    bus: u8,
    port: u8,
    expected_device_id: u32,
    jsonl: bool,
) -> anyhow::Result<Context> {
    const MAX_ATTEMPTS: u32 = 40;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(250);
    let mut last_error = "device has not reappeared".to_owned();

    for attempt in 1..=MAX_ATTEMPTS {
        ensure_not_cancelled()?;
        std::thread::sleep(RETRY_DELAY);
        let devices = match Context::scan_usb_devices() {
            Ok(devices) => devices,
            Err(error) => {
                last_error = format!("scan failed: {error}");
                continue;
            }
        };
        if !reconnected_endpoint(&devices, bus, port)? {
            last_error = "bound physical location is offline".to_owned();
            continue;
        }

        let mut context = Context::new();
        let selected = context.scan_usb_device_at(bus, port);
        if let Err(error) = selected {
            last_error = format!("endpoint selection failed: {error}");
            continue;
        }
        if let Err(error) = context.usb_init().and_then(|_| context.efex_init()) {
            last_error = format!("endpoint open failed: {error}");
            continue;
        }
        if FelTransport::device_id(&context) != expected_device_id {
            last_error = format!(
                "device id mismatch: expected=0x{expected_device_id:08x},actual=0x{:08x}",
                FelTransport::device_id(&context)
            );
            continue;
        }
        emit(
            jsonl,
            json!({
                "event":"phase",
                "phase":"mainline_spl_return_device_reopened",
                "attempt":attempt,
            }),
        );
        return Ok(context);
    }

    bail!("MAINLINE_SPL_RETURN_RECONNECT_FAILED:{last_error}")
}

pub fn execute(
    plan_path: PathBuf,
    device_location: String,
    bus: u8,
    port: u8,
    jsonl: bool,
) -> anyhow::Result<()> {
    if !plan_path.is_absolute()
        || device_location.is_empty()
        || device_location.len() > 1024
        || port == 0
    {
        bail!("MAINLINE_PLAN_INVALID:worker scope");
    }
    let plan_metadata = plan_path.metadata().context("MAINLINE_PLAN_METADATA")?;
    if !plan_metadata.is_file()
        || plan_metadata.len() == 0
        || plan_metadata.len() > MAX_PLAN_FILE_BYTES
    {
        bail!("MAINLINE_PLAN_INVALID:plan size");
    }
    let plan: MainlinePlan =
        serde_json::from_reader(File::open(&plan_path).context("MAINLINE_PLAN_OPEN")?)
            .context("MAINLINE_PLAN_PARSE")?;
    let mut validated_artifacts = validate_plan(&plan)?;
    ensure_not_cancelled()?;

    validate_libusb_location(&device_location, bus, port)?;
    let devices = Context::scan_usb_devices().context("FLASH_TARGET_SCAN")?;
    validated_endpoint(&devices, bus, port)?;
    emit(jsonl, json!({"event":"phase","phase":"endpoint_verified"}));
    let mut context = Context::new();
    context
        .scan_usb_device_at(bus, port)
        .context("FLASH_TARGET_REENUMERATED:current endpoint disappeared before open")?;
    emit(jsonl, json!({"event":"phase","phase":"opening_device"}));
    context.usb_init()?;
    context.efex_init()?;
    emit(jsonl, json!({"event":"phase","phase":"device_opened"}));

    let total_bytes = validated_artifacts
        .iter()
        .map(|artifact| artifact.size)
        .sum::<u64>();
    let mut written_bytes = 0_u64;
    let spl_index = plan
        .artifacts
        .iter()
        .position(|artifact| matches!(artifact.role, MainlineRole::Spl))
        .context("ALLWINNER_MAINLINE_SPL_REQUIRED")?;
    let spl_size = validated_artifacts[spl_index]
        .spl_length
        .context("MAINLINE_SPL_NOT_VALIDATED")?;
    let mut spl = Vec::with_capacity(spl_size);
    validated_artifacts[spl_index]
        .snapshot
        .as_mut()
        .context("MAINLINE_SNAPSHOT_UNAVAILABLE")?
        .read_to_end(&mut spl)
        .context("MAINLINE_SPL_READ")?;
    emit(
        jsonl,
        json!({"event":"phase","phase":"bootstrapping_mainline_spl"}),
    );
    let expected_device_id = FelTransport::device_id(&context);
    execute_r528_mainline_spl(&context, &spl)?;
    // The audited SPL handoff normally returns through the same FEL session.
    // Some controllers nevertheless reset during SPL execution, so
    // retain the standard same-handle path and use physical-location reopen as
    // a bounded recovery only when that first marker read fails.
    std::thread::sleep(std::time::Duration::from_millis(250));
    let context = match verify_r528_spl_return(&context) {
        Ok(()) => {
            emit(
                jsonl,
                json!({"event":"phase","phase":"mainline_spl_return_same_session"}),
            );
            context
        }
        Err(initial_error) => {
            drop(context);
            emit(
                jsonl,
                json!({
                    "event":"phase",
                    "phase":"mainline_spl_return_reconnect_wait",
                    "initialError":initial_error.to_string(),
                }),
            );
            let reopened = reconnect_mainline_fel(bus, port, expected_device_id, jsonl)?;
            verify_r528_spl_return(&reopened).with_context(|| {
                format!("MAINLINE_SPL_RETURN_AFTER_RECONNECT:initial={initial_error}")
            })?;
            reopened
        }
    };
    written_bytes = written_bytes.saturating_add(spl.len() as u64);
    emit(
        jsonl,
        json!({
            "event":"progress",
            "phase":"mainline_spl_returned_to_fel",
            "writtenBytes":written_bytes,
            "totalBytes":total_bytes,
        }),
    );
    emit(
        jsonl,
        json!({
            "event":"progress",
            "phase":"writing_artifacts",
            "writtenBytes":written_bytes,
            "totalBytes":total_bytes,
        }),
    );
    for (artifact, validated) in plan.artifacts.iter().zip(validated_artifacts.iter_mut()) {
        if matches!(artifact.role, MainlineRole::Spl) {
            continue;
        }
        let role = format!("{:?}", artifact.role);
        let mut buffer = vec![0_u8; FEL_CHUNK_SIZE];
        let mut offset = 0_u32;
        loop {
            ensure_not_cancelled()?;
            let read = validated
                .snapshot
                .as_mut()
                .context("MAINLINE_SNAPSHOT_UNAVAILABLE")?
                .read(&mut buffer)
                .context("MAINLINE_SNAPSHOT_READ")?;
            if read == 0 {
                break;
            }
            let address = artifact
                .load_address
                .checked_add(offset)
                .context("MAINLINE_ADDRESS_RANGE_OVERFLOW")?;
            context
                .fel_write(address, &buffer[..read])
                .with_context(|| {
                    format!("ALLWINNER_FEL_WRITE_FAILED:role={role}:address=0x{address:08x}")
                })?;
            offset = offset
                .checked_add(u32::try_from(read).context("MAINLINE_ADDRESS_RANGE_OVERFLOW")?)
                .context("MAINLINE_ADDRESS_RANGE_OVERFLOW")?;
            written_bytes = written_bytes.saturating_add(read as u64);
            emit(
                jsonl,
                json!({
                    "event":"progress",
                    "phase":"writing_artifacts",
                    "writtenBytes":written_bytes,
                    "totalBytes":total_bytes,
                }),
            );
        }
        if u64::from(offset) != validated.size {
            bail!("MAINLINE_SNAPSHOT_SIZE_CHANGED");
        }
    }
    ensure_not_cancelled()?;
    emit(jsonl, json!({"event":"phase","phase":"executing_entry"}));
    context.fel_exec(plan.entry_address)?;
    emit(
        jsonl,
        json!({"event":"complete","phase":"complete","writtenBytes":written_bytes,"totalBytes":total_bytes}),
    );
    Ok(())
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    fn device(bus: u8, port: u8) -> libefex::ScannedDevice {
        libefex::ScannedDevice {
            bus,
            port,
            vid: 0x1f3a,
            pid: 0xefe8,
        }
    }

    #[test]
    fn worker_requires_location_to_match_current_bus_and_port() {
        assert!(validate_libusb_location("libusb:3:2", 3, 2).is_ok());
        assert!(validate_libusb_location("libusb:3:7", 3, 2)
            .unwrap_err()
            .to_string()
            .starts_with("FLASH_TARGET_REENUMERATED:"));
    }

    #[test]
    fn worker_opens_only_the_bound_fel_endpoint() {
        let devices = [device(3, 2), device(3, 7)];
        assert!(validated_endpoint(&devices, 3, 2).is_ok());
        assert!(validated_endpoint(&devices, 3, 9)
            .unwrap_err()
            .to_string()
            .starts_with("FLASH_TARGET_NOT_FOUND:"));
    }

    #[test]
    fn spl_return_reconnect_never_migrates_to_another_endpoint() {
        assert!(reconnected_endpoint(&[device(3, 2)], 3, 2).unwrap());
        assert!(!reconnected_endpoint(&[device(3, 7)], 3, 2).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap};

    fn test_artifact(address: u32, contents: &[u8]) -> (PathBuf, MainlineArtifact) {
        let path = std::env::temp_dir().join(format!(
            "openix-mainline-source-{}-{:016x}.bin",
            std::process::id(),
            NEXT_SNAPSHOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, contents).unwrap();
        let artifact = MainlineArtifact {
            role: MainlineRole::Bootloader,
            file_path: path.clone(),
            load_address: address,
            sha256: format!("{:x}", Sha256::digest(contents)),
        };
        (path, artifact)
    }

    fn test_spl() -> Vec<u8> {
        let mut spl = vec![0_u8; 0x6000];
        spl[4..12].copy_from_slice(b"eGON.BT0");
        spl[0x0c..0x10].copy_from_slice(&0x5f0a_6c39_u32.to_le_bytes());
        let spl_length = spl.len() as u32;
        spl[0x10..0x14].copy_from_slice(&spl_length.to_le_bytes());
        spl[0x14..0x17].copy_from_slice(b"SPL");
        spl[0x17] = 2;
        let checksum = spl.chunks_exact(4).fold(0_u32, |sum, word| {
            sum.wrapping_add(u32::from_le_bytes(word.try_into().unwrap()))
        });
        spl[0x0c..0x10].copy_from_slice(&checksum.to_le_bytes());
        spl
    }

    fn spl_artifact(contents: &[u8]) -> (PathBuf, MainlineArtifact) {
        let (path, mut artifact) = test_artifact(R528_SPL_ADDRESS, contents);
        artifact.role = MainlineRole::Spl;
        (path, artifact)
    }

    #[derive(Default)]
    struct FakeFel {
        device_id: u32,
        memory: RefCell<BTreeMap<u32, u8>>,
        executions: RefCell<Vec<u32>>,
    }

    impl FelTransport for FakeFel {
        fn device_id(&self) -> u32 {
            self.device_id
        }

        fn write(&self, address: u32, bytes: &[u8]) -> anyhow::Result<()> {
            for (offset, byte) in bytes.iter().copied().enumerate() {
                self.memory
                    .borrow_mut()
                    .insert(address + offset as u32, byte);
            }
            Ok(())
        }

        fn read(&self, address: u32, bytes: &mut [u8]) -> anyhow::Result<()> {
            for (offset, byte) in bytes.iter_mut().enumerate() {
                *byte = self
                    .memory
                    .borrow()
                    .get(&(address + offset as u32))
                    .copied()
                    .unwrap_or(0);
            }
            Ok(())
        }

        fn execute(&self, address: u32) -> anyhow::Result<()> {
            self.executions.borrow_mut().push(address);
            if address == R528_THUNK_ADDRESS {
                self.write(R528_SPL_ADDRESS + 4, SPL_RETURN_MARKER)?;
            }
            Ok(())
        }
    }

    #[test]
    fn entry_must_be_one_declared_load_address() {
        let plan = MainlinePlan {
            artifacts: Vec::new(),
            entry_address: 0x4000_0000,
        };
        assert!(validate_plan(&plan)
            .unwrap_err()
            .to_string()
            .contains("artifact count"));
    }

    #[test]
    fn chunk_addresses_add_each_offset_exactly_once() {
        let base = 0x4000_0000_u32;
        let first = base.checked_add(0).unwrap();
        let second = base.checked_add(FEL_CHUNK_SIZE as u32).unwrap();
        assert_eq!(FEL_CHUNK_SIZE, 16 * 1024);
        assert_eq!(first, base);
        assert_eq!(second, 0x4000_4000);
    }

    #[test]
    fn validated_snapshot_is_immutable_from_later_source_changes_and_is_cleaned_up() {
        let (source_path, artifact) = test_artifact(0x4000_0000, b"trusted-mainline");
        let mut validated = validate_artifact(&artifact).unwrap();
        let snapshot_path = validated.snapshot_path.clone();
        std::fs::write(&source_path, b"changed-after-validation").unwrap();

        let mut snapshot = Vec::new();
        validated
            .snapshot
            .as_mut()
            .unwrap()
            .read_to_end(&mut snapshot)
            .unwrap();
        assert_eq!(snapshot, b"trusted-mainline");
        drop(validated);
        assert!(!snapshot_path.exists());
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn helper_rejects_low_and_overlapping_load_addresses() {
        let (low_path, low) = test_artifact(0x100, b"low");
        assert!(validate_artifact(&low)
            .unwrap_err()
            .to_string()
            .contains("artifact path/address"));
        let _ = std::fs::remove_file(low_path);

        let (spl_path, spl) = spl_artifact(&test_spl());
        let (first_path, first) = test_artifact(0x4000_0000, b"12345678");
        let (second_path, mut second) = test_artifact(0x4000_0004, b"abcdefgh");
        second.role = MainlineRole::Kernel;
        let plan = MainlinePlan {
            artifacts: vec![spl, first, second],
            entry_address: 0x4000_0000,
        };
        assert!(validate_plan(&plan)
            .unwrap_err()
            .to_string()
            .contains("MAINLINE_ARTIFACT_ADDRESS_OVERLAP"));
        let _ = std::fs::remove_file(spl_path);
        let _ = std::fs::remove_file(first_path);
        let _ = std::fs::remove_file(second_path);
    }

    #[test]
    fn helper_rejects_dram_bootloader_without_verified_spl_before_device_open() {
        let (path, artifact) = test_artifact(0x4700_0000, b"u-boot-proper-without-egon-spl");
        let plan = MainlinePlan {
            artifacts: vec![artifact],
            entry_address: 0x4700_0000,
        };

        assert_eq!(
            validate_plan(&plan).unwrap_err().to_string(),
            "ALLWINNER_MAINLINE_SPL_REQUIRED:expected exactly one mainline U-Boot SPL"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mainline_spl_requires_exact_egon_spl_file_and_checksum() {
        let spl = test_spl();
        assert_eq!(validate_mainline_spl(&spl).unwrap(), 0x6000);

        let mut vendor_boot0 = spl.clone();
        vendor_boot0[0x14..0x17].copy_from_slice(b"BT0");
        assert!(validate_mainline_spl(&vendor_boot0)
            .unwrap_err()
            .to_string()
            .starts_with("MAINLINE_SPL_SIGNATURE_REQUIRED:"));

        let mut corrupt = spl.clone();
        corrupt[0x100] ^= 0x80;
        assert_eq!(
            validate_mainline_spl(&corrupt).unwrap_err().to_string(),
            "MAINLINE_SPL_CHECKSUM_INVALID"
        );

        let mut redundant = spl.clone();
        redundant.extend_from_slice(&spl);
        assert!(validate_mainline_spl(&redundant)
            .unwrap_err()
            .to_string()
            .starts_with("MAINLINE_SPL_EXACT_FILE_REQUIRED:"));
    }

    #[test]
    fn r528_bootstrap_disables_icache_swaps_sram_and_requires_fel_return() {
        let thunk = r528_spl_thunk();
        assert_eq!(thunk.len(), 304);
        assert_eq!(
            format!("{:x}", Sha256::digest(&thunk)),
            "af829be896d9348f5eae9a82c80726a2daf7f94610c32b7eced44898f2a1762e"
        );
        let transport = FakeFel {
            device_id: R528_DEVICE_ID,
            ..FakeFel::default()
        };
        execute_r528_mainline_spl(&transport, &test_spl()).unwrap();
        let returned_transport = FakeFel {
            device_id: R528_DEVICE_ID,
            memory: RefCell::new(transport.memory.borrow().clone()),
            ..FakeFel::default()
        };
        verify_r528_spl_return(&returned_transport).unwrap();
        assert_eq!(
            transport.executions.borrow().as_slice(),
            &[R528_SCRATCH_ADDRESS, R528_THUNK_ADDRESS]
        );
        assert_eq!(
            transport.memory.borrow().get(&R528_SWAP_BACKUP).copied(),
            Some(test_spl()[R528_SWAP_BYTES])
        );
    }

    #[test]
    fn unsupported_soc_is_rejected_before_any_sram_write() {
        let transport = FakeFel {
            device_id: 0x0018_2300,
            ..FakeFel::default()
        };
        assert!(execute_r528_mainline_spl(&transport, &test_spl())
            .unwrap_err()
            .to_string()
            .starts_with("ALLWINNER_MAINLINE_SOC_UNSUPPORTED:"));
        assert!(transport.memory.borrow().is_empty());
        assert!(transport.executions.borrow().is_empty());
    }

    #[test]
    fn validated_plan_requires_one_spl_and_one_bootloader() {
        let spl = test_spl();
        let (spl_path, spl_artifact) = spl_artifact(&spl);
        let (boot_path, bootloader) = test_artifact(0x4700_0000, b"mainline-u-boot-proper");
        let plan = MainlinePlan {
            artifacts: vec![spl_artifact, bootloader],
            entry_address: 0x4700_0000,
        };
        let validated = validate_plan(&plan).unwrap();
        assert_eq!(validated[0].spl_length, Some(0x6000));
        let _ = std::fs::remove_file(spl_path);
        let _ = std::fs::remove_file(boot_path);
    }
}
