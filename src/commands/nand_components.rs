//! Explicit FEL -> vendor FES -> NAND component provisioning route.
//!
//! This module intentionally does not share the mainline RAM installer worker.
//! The bootstrap IMAGEWTY package is used only to bring up DRAM/FES; the
//! component IMAGEWTY package is the only source of persistent media data.

use crate::commands::{FlashMode as CommandFlashMode, NandComponentArgs};
use crate::flash::{FlashMode, FlashOptions, Flasher, NandConstraints};
use crate::utils::Logger;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const ROUTE: &str = "fes_nand_components";

pub fn error_code(message: &str) -> &str {
    message
        .split(|character: char| {
            !(character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_')
        })
        .find(|token| {
            token.starts_with("NAND_")
                || token.starts_with("PARTITION_")
                || token.starts_with("BOOT_")
        })
        .unwrap_or("FES_NAND_FAILED")
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub route: String,
    pub board: String,
    pub soc: String,
    pub bootstrap: Artifact,
    pub firmware_package: Artifact,
    pub storage: StorageExpectation,
    pub layout: Layout,
    pub components: Vec<ComponentArtifact>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageExpectation {
    pub kind: String,
    pub capacity_bytes: u64,
    pub page_size: u32,
    pub erase_size: u32,
    pub capacity_probe_policy: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layout {
    pub version: String,
    /// Linux-visible fixed MTD regions used to check physical bounds.
    pub partitions: Vec<Partition>,
    /// Logical UBI volumes that the FES MBR must create inside `sys`.
    pub fes_partitions: Vec<FesPartition>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FesPartition {
    pub name: String,
    pub address_sectors: u64,
    pub size_sectors: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Partition {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentArtifact {
    pub role: String,
    pub content_type: String,
    pub partition: Option<String>,
    pub file: String,
    pub sha256: String,
    /// Exact IMAGEWTY filename carrying this component.
    pub package_file: String,
}

fn validate_component_content(path: &Path, component: &ComponentArtifact) -> anyhow::Result<()> {
    let data = fs::read(path)?;
    let valid = match component.content_type.as_str() {
        "egon-boot0" => data.get(4..12) == Some(b"eGON.BT0"),
        "legacy-uboot" => data.get(..4) == Some(&[0x27, 0x05, 0x19, 0x56]),
        "fit" => data.get(..4) == Some(&[0xd0, 0x0d, 0xfe, 0xed]),
        "ubifs" => data.get(..4) == Some(&[0x31, 0x18, 0x10, 0x06]),
        _ => bail!(
            "NAND_MANIFEST_INVALID_CONTENT_TYPE:{}:{}",
            component.role,
            component.content_type
        ),
    };
    if !valid {
        bail!(
            "NAND_COMPONENT_CONTENT_MISMATCH:{}:{}",
            component.role,
            component.content_type
        );
    }
    Ok(())
}

pub struct ValidatedManifest {
    manifest: Manifest,
    bootstrap_path: PathBuf,
    firmware_path: PathBuf,
}

fn validate_fes_layout(
    expected: &[FesPartition],
    actual: &[crate::config::mbr_parser::SunxiPartition],
) -> anyhow::Result<()> {
    let expected_names: HashSet<&str> = expected
        .iter()
        .map(|partition| partition.name.as_str())
        .collect();
    let actual_partitions: HashMap<&str, _> = actual
        .iter()
        .map(|partition| (partition.name.as_str(), partition))
        .collect();
    if actual_partitions.len() != actual.len() {
        bail!("NAND_COMPONENT_PACKAGE_DUPLICATE_PARTITION");
    }
    for expected_partition in expected {
        let actual_partition = actual_partitions
            .get(expected_partition.name.as_str())
            .with_context(|| {
                format!(
                    "NAND_COMPONENT_PACKAGE_PARTITION_MISSING:{}",
                    expected_partition.name
                )
            })?;
        if actual_partition.address() != expected_partition.address_sectors
            || actual_partition.length() != expected_partition.size_sectors
        {
            bail!(
                "NAND_COMPONENT_PACKAGE_PARTITION_LAYOUT_MISMATCH:{}:expected={}/{}:actual={}/{}",
                expected_partition.name,
                expected_partition.address_sectors,
                expected_partition.size_sectors,
                actual_partition.address(),
                actual_partition.length()
            );
        }
    }
    for actual_partition in actual {
        if !expected_names.contains(actual_partition.name.as_str()) {
            bail!(
                "NAND_COMPONENT_PACKAGE_UNEXPECTED_PARTITION:{}",
                actual_partition.name
            );
        }
    }
    Ok(())
}

fn emit(jsonl: bool, event: serde_json::Value, text: &str) {
    if jsonl {
        println!("{}", event);
    } else {
        println!("{text}");
    }
}

fn validate_hash(value: &str, label: &str) -> anyhow::Result<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("NAND_MANIFEST_INVALID_SHA256:{label}");
    }
    Ok(())
}

fn safe_relative(root: &Path, value: &str, label: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(value);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("NAND_MANIFEST_UNSAFE_PATH:{label}:{value}");
    }
    let joined = root.join(relative);
    let canonical = joined
        .canonicalize()
        .with_context(|| format!("NAND_ARTIFACT_NOT_FOUND:{label}:{}", joined.display()))?;
    let canonical_root = root
        .canonicalize()
        .context("NAND_MANIFEST_ROOT_NOT_FOUND")?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        bail!("NAND_MANIFEST_UNSAFE_PATH:{label}:{value}");
    }
    Ok(canonical)
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_artifact(root: &Path, artifact: &Artifact, label: &str) -> anyhow::Result<PathBuf> {
    validate_hash(&artifact.sha256, label)?;
    let path = safe_relative(root, &artifact.file, label)?;
    let actual = sha256_file(&path)?;
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        bail!(
            "NAND_ARTIFACT_SHA256_MISMATCH:{label}:expected={}:actual={actual}",
            artifact.sha256
        );
    }
    Ok(path)
}

pub fn validate_manifest(path: &Path) -> anyhow::Result<ValidatedManifest> {
    let manifest_path = path
        .canonicalize()
        .with_context(|| format!("NAND_MANIFEST_NOT_FOUND:{}", path.display()))?;
    let root = manifest_path
        .parent()
        .context("NAND_MANIFEST_HAS_NO_PARENT")?;
    let bytes = fs::read(&manifest_path)?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).context("NAND_MANIFEST_INVALID_JSON")?;

    if manifest.format_version != 1 || manifest.route != ROUTE {
        bail!("NAND_MANIFEST_UNSUPPORTED_FORMAT_OR_ROUTE");
    }
    if manifest.board.trim().is_empty() || manifest.soc.trim().is_empty() {
        bail!("NAND_MANIFEST_BOARD_AND_SOC_REQUIRED");
    }
    if manifest.storage.kind != "spi-nand" && manifest.storage.kind != "nand" {
        bail!("NAND_MANIFEST_STORAGE_MUST_BE_NAND");
    }
    if manifest.storage.capacity_probe_policy != "fes-logical-or-unavailable" {
        bail!("NAND_MANIFEST_CAPACITY_PROBE_POLICY_UNSUPPORTED");
    }
    if manifest.storage.capacity_bytes == 0
        || manifest.storage.page_size == 0
        || manifest.storage.erase_size == 0
        || !manifest
            .storage
            .erase_size
            .is_multiple_of(manifest.storage.page_size)
    {
        bail!("NAND_MANIFEST_INVALID_GEOMETRY");
    }
    if manifest.layout.version.trim().is_empty()
        || manifest.layout.partitions.is_empty()
        || manifest.layout.fes_partitions.is_empty()
    {
        bail!("NAND_MANIFEST_LAYOUT_REQUIRED");
    }

    let mut names = HashSet::new();
    let mut ranges = Vec::new();
    for partition in &manifest.layout.partitions {
        if partition.name.is_empty()
            || partition.size == 0
            || !partition
                .offset
                .is_multiple_of(manifest.storage.erase_size as u64)
            || !partition
                .size
                .is_multiple_of(manifest.storage.erase_size as u64)
            || partition.offset.saturating_add(partition.size) > manifest.storage.capacity_bytes
            || !names.insert(partition.name.clone())
        {
            bail!("NAND_MANIFEST_INVALID_PARTITION:{}", partition.name);
        }
        ranges.push((partition.offset, partition.offset + partition.size));
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        bail!("NAND_MANIFEST_OVERLAPPING_PARTITIONS");
    }
    if ranges.first().map(|range| range.0) != Some(0)
        || ranges.last().map(|range| range.1) != Some(manifest.storage.capacity_bytes)
        || ranges.windows(2).any(|pair| pair[0].1 != pair[1].0)
    {
        bail!("NAND_MANIFEST_PHYSICAL_LAYOUT_MUST_COVER_DEVICE");
    }

    let fes_names: HashSet<&str> = manifest
        .layout
        .fes_partitions
        .iter()
        .map(|partition| partition.name.as_str())
        .collect();
    if fes_names.len() != manifest.layout.fes_partitions.len()
        || manifest
            .layout
            .fes_partitions
            .iter()
            .any(|partition| partition.name.is_empty())
    {
        bail!("NAND_MANIFEST_INVALID_FES_PARTITIONS");
    }
    let mut roles = HashSet::new();
    for component in &manifest.components {
        validate_hash(&component.sha256, &component.role)?;
        if component.package_file.is_empty()
            || Path::new(&component.package_file)
                .file_name()
                .and_then(|v| v.to_str())
                != Some(component.package_file.as_str())
        {
            bail!("NAND_MANIFEST_INVALID_PACKAGE_FILE:{}", component.role);
        }
        if !roles.insert((component.role.clone(), component.partition.clone())) {
            bail!("NAND_MANIFEST_DUPLICATE_COMPONENT:{}", component.role);
        }
        match (component.role.as_str(), component.content_type.as_str()) {
            ("boot0", "egon-boot0") | ("boot1", "legacy-uboot")
                if component.partition.is_none() => {}
            ("partition", "fit" | "ubifs") => {
                let name = component
                    .partition
                    .as_ref()
                    .context("NAND_MANIFEST_PARTITION_COMPONENT_NAME_REQUIRED")?;
                if !fes_names.contains(name.as_str()) {
                    bail!("NAND_MANIFEST_COMPONENT_UNKNOWN_PARTITION:{name}");
                }
            }
            _ => bail!(
                "NAND_MANIFEST_INVALID_COMPONENT_ROLE_OR_TYPE:{}:{}",
                component.role,
                component.content_type
            ),
        }
        let component_path = safe_relative(root, &component.file, &component.role)?;
        validate_component_content(&component_path, component)?;
        let actual = sha256_file(&component_path)?;
        if !actual.eq_ignore_ascii_case(&component.sha256) {
            bail!("NAND_COMPONENT_SHA256_MISMATCH:{}", component.role);
        }
    }
    if !manifest.components.iter().any(|item| item.role == "boot0")
        || !manifest.components.iter().any(|item| item.role == "boot1")
    {
        bail!("NAND_MANIFEST_BOOT0_AND_BOOT1_REQUIRED");
    }
    if manifest
        .components
        .iter()
        .filter(|item| item.content_type == "ubifs")
        .count()
        > 1
    {
        bail!("NAND_MANIFEST_ONLY_ONE_UBIFS_COMPONENT_SUPPORTED");
    }

    let bootstrap_path = verify_artifact(root, &manifest.bootstrap, "bootstrap")?;
    let firmware_path = verify_artifact(root, &manifest.firmware_package, "firmwarePackage")?;
    Ok(ValidatedManifest {
        manifest,
        bootstrap_path,
        firmware_path,
    })
}

fn validate_invocation(args: &NandComponentArgs) -> anyhow::Result<()> {
    if !matches!(
        args.mode,
        CommandFlashMode::PartitionErase | CommandFlashMode::FullErase
    ) {
        bail!("NAND_ERASE_POLICY_REQUIRED:partition_erase_or_full_erase");
    }
    if !matches!(args.post_action.as_str(), "none" | "reboot" | "poweroff") {
        bail!("NAND_INVALID_POST_ACTION");
    }
    if !args.verify {
        bail!("NAND_VERIFY_REQUIRED");
    }
    let expected = format!("libusb:{}:{}", args.bus, args.port);
    if args.device_location != expected {
        bail!(
            "NAND_USB_BINDING_MISMATCH:location={}:bus={}:port={}",
            args.device_location,
            args.bus,
            args.port
        );
    }
    Ok(())
}

pub async fn execute(args: NandComponentArgs) -> anyhow::Result<()> {
    validate_invocation(&args)?;
    let validated = validate_manifest(&args.manifest_path)?;
    let mut component_packer = crate::firmware::OpenixPacker::new();
    component_packer
        .load(&validated.firmware_path)
        .context("NAND_COMPONENT_PACKAGE_INVALID")?;
    let mbr_data = component_packer
        .get_mbr()
        .context("NAND_COMPONENT_PACKAGE_MBR_REQUIRED")?;
    let mbr = crate::config::mbr_parser::SunxiMbr::parse(&mbr_data)
        .map_err(|error| anyhow::anyhow!("NAND_COMPONENT_PACKAGE_MBR_INVALID:{error}"))?;
    validate_fes_layout(&validated.manifest.layout.fes_partitions, &mbr.partitions)?;
    for component in &validated.manifest.components {
        let embedded = component_packer
            .get_file_data_by_filename(&component.package_file)
            .with_context(|| {
                format!(
                    "NAND_COMPONENT_NOT_EMBEDDED:{}:{}",
                    component.role, component.package_file
                )
            })?;
        let embedded_hash = format!("{:x}", Sha256::digest(&embedded));
        if !embedded_hash.eq_ignore_ascii_case(&component.sha256) {
            bail!(
                "NAND_EMBEDDED_COMPONENT_SHA256_MISMATCH:{}:expected={}:actual={embedded_hash}",
                component.role,
                component.sha256
            );
        }
    }
    let mut bootstrap_packer = crate::firmware::OpenixPacker::new();
    bootstrap_packer
        .load(&validated.bootstrap_path)
        .context("NAND_BOOTSTRAP_PACKAGE_INVALID")?;
    bootstrap_packer
        .get_fes()
        .context("NAND_BOOTSTRAP_FES_REQUIRED")?;
    bootstrap_packer
        .get_uboot()
        .context("NAND_BOOTSTRAP_UBOOT_REQUIRED")?;
    bootstrap_packer
        .get_sys_config_bin()
        .context("NAND_BOOTSTRAP_SYS_CONFIG_REQUIRED")?;
    bootstrap_packer
        .get_board_config()
        .context("NAND_BOOTSTRAP_BOARD_CONFIG_REQUIRED")?;
    emit(
        args.jsonl,
        json!({
            "event":"preflight_complete",
            "route":ROUTE,
            "board":validated.manifest.board,
            "soc":validated.manifest.soc,
            "storage":validated.manifest.storage.kind,
            "capacityBytes":validated.manifest.storage.capacity_bytes,
            "layoutVersion":validated.manifest.layout.version,
            "componentCount":validated.manifest.components.len(),
            "embeddedComponentsVerified":true,
            "bootstrapRolesVerified":true,
            "fesPartitions":validated.manifest.layout.fes_partitions
        }),
        "FES NAND component host preflight passed",
    );
    if args.preflight_only {
        emit(
            args.jsonl,
            json!({"event":"complete","route":ROUTE,"scope":"host_preflight","coldBootStatus":"not_observed"}),
            "Preflight-only request complete; USB was not opened",
        );
        return Ok(());
    }

    let logger = if args.jsonl {
        Logger::with_jsonl(args.verbose, ROUTE)
    } else {
        Logger::with_verbose(args.verbose)
    };
    let options = FlashOptions {
        bus: Some(args.bus),
        port: Some(args.port),
        verify: args.verify,
        mode: match args.mode {
            CommandFlashMode::PartitionErase => FlashMode::PartitionErase,
            CommandFlashMode::FullErase => FlashMode::FullErase,
            _ => unreachable!(),
        },
        partitions: None,
        post_action: args.post_action,
        reconnect_timeout_sec: args.reconnect_timeout_sec,
        reconnect_interval_ms: args.reconnect_interval_ms,
        nand_constraints: Some(NandConstraints {
            expected_capacity_bytes: validated.manifest.storage.capacity_bytes,
            minimum_logical_sectors: validated
                .manifest
                .layout
                .fes_partitions
                .iter()
                .map(|partition| partition.address_sectors + partition.size_sectors)
                .max()
                .unwrap_or(0),
            allow_unavailable_capacity: true,
            expected_partitions: validated
                .manifest
                .components
                .iter()
                .filter_map(|component| {
                    (component.role == "partition")
                        .then(|| component.partition.clone())
                        .flatten()
                })
                .collect(),
            expected_ubifs_partition: validated
                .manifest
                .components
                .iter()
                .find(|component| component.content_type == "ubifs")
                .and_then(|component| component.partition.clone()),
            exact_bus: args.bus,
            exact_port: args.port,
            disable_fes_retry: true,
        }),
    };
    let mut flasher =
        Flasher::new(component_packer, options, logger).with_ram_only_bootstrap(bootstrap_packer);
    flasher.execute().await.map_err(anyhow::Error::msg)?;
    emit(
        args.jsonl,
        json!({"event":"complete","route":ROUTE,"scope":"fes_media_provisioning","coldBootStatus":"not_observed"}),
        "FES media provisioning complete; cold boot has not yet been observed",
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn digest(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    #[test]
    fn extracts_stable_nested_protocol_error_code() {
        assert_eq!(
            error_code("Invalid firmware format: NAND_MBR_VERIFY_FAILED"),
            "NAND_MBR_VERIFY_FAILED"
        );
        assert_eq!(error_code("Failed to open device"), "FES_NAND_FAILED");
    }

    fn mbr_partition(
        name: &str,
        address: u64,
        length: u64,
    ) -> crate::config::mbr_parser::SunxiPartition {
        crate::config::mbr_parser::SunxiPartition {
            addrhi: (address >> 32) as u32,
            addrlo: address as u32,
            lenhi: (length >> 32) as u32,
            lenlo: length as u32,
            classname: "DISK".into(),
            name: name.into(),
            user_type: 0,
            keydata: 0,
            ro: 0,
        }
    }

    #[test]
    fn rejects_component_package_with_old_fes_layout() {
        let expected = [FesPartition {
            name: "boot".into(),
            address_sectors: 504,
            size_sectors: 16632,
        }];
        let old = [mbr_partition("boot", 504, 16128)];
        let error = validate_fes_layout(&expected, &old)
            .unwrap_err()
            .to_string();
        assert!(error.contains("NAND_COMPONENT_PACKAGE_PARTITION_LAYOUT_MISMATCH:boot"));
    }

    #[test]
    fn rejects_raw_style_modes_before_usb() {
        let args = NandComponentArgs {
            manifest_path: PathBuf::from("missing"),
            device_location: "libusb:3:2".into(),
            bus: 3,
            port: 2,
            mode: CommandFlashMode::Partition,
            verify: true,
            post_action: "none".into(),
            reconnect_timeout_sec: 90,
            reconnect_interval_ms: 500,
            preflight_only: true,
            verbose: false,
            jsonl: true,
        };
        assert!(validate_invocation(&args)
            .unwrap_err()
            .to_string()
            .contains("NAND_ERASE_POLICY_REQUIRED"));
    }

    #[test]
    fn validates_closed_board_package() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("openix-nand-manifest-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let boot0 = b"xxxxeGON.BT0boot0";
        let boot1 = b"\x27\x05\x19\x56boot1";
        let boot = b"\xd0\x0d\xfe\xedboot";
        let files = [
            ("bootstrap.img", b"IMAGEWTY-bootstrap".as_slice()),
            ("components.img", b"IMAGEWTY-components".as_slice()),
            ("boot0.bin", boot0.as_slice()),
            ("boot1.bin", boot1.as_slice()),
            ("boot.itb", boot.as_slice()),
        ];
        for (name, bytes) in files {
            fs::write(root.join(name), bytes).unwrap();
        }
        let manifest = json!({
            "formatVersion":1,
            "route":ROUTE,
            "board":"dshanpi-t113s3pro",
            "soc":"r528",
            "bootstrap":{"file":"bootstrap.img","sha256":digest(b"IMAGEWTY-bootstrap")},
            "firmwarePackage":{"file":"components.img","sha256":digest(b"IMAGEWTY-components")},
            "storage":{"kind":"spi-nand","capacityBytes":268435456u64,"pageSize":2048,"eraseSize":131072,"capacityProbePolicy":"fes-logical-or-unavailable"},
            "layout":{"version":"test-v1","partitions":[
                {"name":"spl","offset":0,"size":1048576},
                {"name":"uboot","offset":1048576,"size":3145728},
                {"name":"secure-storage","offset":4194304,"size":1048576},
                {"name":"sys","offset":5242880u64,"size":263192576u64}
            ],"fesPartitions":[{"name":"boot","addressSectors":504,"sizeSectors":16632}]},
            "components":[
                {"role":"boot0","contentType":"egon-boot0","partition":null,"file":"boot0.bin","packageFile":"boot0_nand.fex","sha256":digest(boot0)},
                {"role":"boot1","contentType":"legacy-uboot","partition":null,"file":"boot1.bin","packageFile":"u-boot.fex","sha256":digest(boot1)},
                {"role":"partition","contentType":"fit","partition":"boot","file":"boot.itb","packageFile":"boot.fex","sha256":digest(boot)}
            ]
        });
        let path = root.join("manifest.json");
        fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let validated = validate_manifest(&path).unwrap();
        assert_eq!(validated.manifest.board, "dshanpi-t113s3pro");
        fs::remove_dir_all(root).unwrap();
    }
}
