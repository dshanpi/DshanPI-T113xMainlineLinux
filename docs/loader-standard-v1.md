# Allwinner RAM Loader Standard v1

## 1. Scope

This specification defines a registry and deterministic packaging contract for Allwinner IMAGEWTY v3 containers used only to bootstrap FEL devices into RAM-resident FES/USB Product mode. It does not define a final firmware image or an on-media boot layout.

## 2. Canonical name

```text
<chip>-<memory-type>-<storage-type>-<product-name>-loader.bin
```

Each field is lowercase kebab-case (`[a-z0-9]+(?:-[a-z0-9]+)*`). The manifest stores the four fields independently and the builder derives the filename. A manifest-provided `output_name` must equal the derived name.

Recommended tokens:

- chip: public chip name without punctuation, such as `t113s3`, `t113s4`, `r528`, `h133`, `d1`, or `v853`;
- memory type: `ddr2`, `ddr3`, `ddr4`, `lpddr2`, `lpddr3`, or `lpddr4`;
- storage type: `nand`, `spinand`, `spinor`, `emmc`, `sd`, or `ufs`;
- product name: stable board/product slug, not a mutable marketing version.

Memory size and clock, storage geometry, SoC family, and FEL/FES Device IDs belong in the manifest, not in the canonical filename.

## 3. Manifest

Every profile contains `loader.json` with:

- `schema_version = 1`;
- canonical identity: `chip`, `memory_type`, `storage_type`, `product_name`, and `output_name`;
- hardware identity: SoC family, Device IDs, DRAM size/clock, and storage capacity where known;
- `purpose = "ram-fes-bootstrap"` and `flash_payload = false`;
- `expected_output_sha256` pinning the complete canonical binary;
- source provenance and `hardware_validation` status;
- exactly six ordered entries, each with role, IMAGEWTY main/sub type, path, source SHA-256, packed SHA-256, and runtime disposition.

Allowed hardware validation values are `software-only`, `ram-bootstrap-passed`, and `hil-passed`. Only `hil-passed` asserts successful media write and verification.

## 4. Required IMAGEWTY entries

| Order | Role | Main type | Subtype | Runtime use |
|---:|---|---|---|---|
| 1 | `mbr-placeholder` | `12345678` | `1234567890___MBR` | container compatibility only; never written |
| 2 | `sys-config` | `COMMON` | `SYS_CONFIG_BIN00` | DDR/board configuration in RAM |
| 3 | `board-config` | `COMMON` | `BOARD_CONFIG_BIN` | board configuration in RAM |
| 4 | `dtb-config` | `COMMON` | `DTB_CONFIG000000` | vendor U-Boot device tree in RAM |
| 5 | `fes1` | `FES` | `FES_1-0000000000` | DDR/FEL-to-FES bootstrap in RAM |
| 6 | `uboot` | `12345678` | `UBOOT_0000000000` | USB Product/FES service in RAM |

Boot0, Boot1, partition payloads, root filesystems, mainline system images, and Phoenix/PID images are forbidden in a v1 loader.

## 5. Container requirements

- Unencrypted IMAGEWTY v3 (`header_version = 0x0300`).
- One 1024-byte image header and one 1024-byte header per entry.
- Payload offsets aligned to 1024 bytes.
- Stored and original lengths equal the exact source length; loader entries are not compressed.
- Output produced by the pinned compatible vendor `dragon` backend.
- Post-build parser verification must match every main type, subtype, length, packed SHA-256, image size, and canonical filename. `dragon` may update a format checksum inside selected payloads, so both pre-pack `sha256` and post-pack `packed_sha256` are mandatory.
- The complete output must match `expected_output_sha256`; a changed output requires review and a new release.

## 6. Consumer safety contract

A consumer must verify both the published SHA-256 and manifest before opening hardware. It must use the loader only for FEL-to-FES RAM bootstrap. It must wait for and validate FES re-enumeration, storage type, and capacity before any erase. It must write only the separately supplied final-system image.

No implementation may silently fall back to Boot0, SyterKit, Phoenix, an entire Tina firmware image, or any vendor partition flashing path.
