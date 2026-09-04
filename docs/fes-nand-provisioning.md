# FES NAND component provisioning

## Status

`hardware-verified` for the exact v5 bundle, DshanPi T113S3 Pro and Winbond
W25N02KV tested on 2026-08-26. FES verified every persistent component and a
separate one-second power-off cold boot reached the mainline login prompt. This
does not qualify other boards, NAND parts or manufacturing bad-block samples.

## Boundary

The final NAND content is entirely mainline: mainline SPL, mainline U-Boot,
Linux FIT and Buildroot/UBI root filesystem. A board-matched Tina/IMAGEWTY
loader is used only in RAM to initialize DDR and expose the FES service.

OpenixCLI exposes this as `flash-nand-components`; it does not reuse
`boot-mainline`, does not allow a whole-disk raw image, accepts only
`partition_erase` or `full_erase`, pins the physical USB endpoint and disables
automatic FES retry.

## Bundle contract

`scripts/prepare-fes-bundle.py` creates a closed directory containing:

```text
manifest.json
bootstrap-loader.img
mainline-nand-components.img
boot0-mainline.bin
boot1-mainline.img
boot.itb
rootfs.ubifs
SHA256SUMS
```

Every file is SHA-256 pinned. OpenixCLI also extracts every named component
from the IMAGEWTY component package and compares it with the manifest hash
before opening USB. This prevents a correct loose file being paired with a
stale container.

The bootstrap loader builder, profile and required input bins are included under
`tools/allwinner-loader/`. Build it reproducibly with `make -C
tools/allwinner-loader check dist`; its canonical output is copied into the FES
bundle as `bootstrap-loader.img`.

## Required layout agreement

The mainline physical view remains:

| Region | Offset | Size |
|---|---:|---:|
| SPL/Boot0 reservation | `0x00000000` | 1 MiB |
| U-Boot/Boot1 reservation | `0x00100000` | 3 MiB |
| secure storage reservation | `0x00400000` | 1 MiB |
| `sys` | `0x00500000` | 251 MiB |

FES creates `boot`, `rootfs`, and autoresize `UDISK` UBI volumes inside `sys`;
U-Boot reads the kernel FIT from `boot` and Linux mounts `rootfs`. The
5 MiB boundary is derived from the Tina SPI-NAND source: 8 Boot0 blocks,
24 Boot1 blocks and 8 secure-storage blocks at 128 KiB each. The remaining
hardware result proves the complete FES write and cold-boot chain for the
hash-pinned v5 bundle.

The bundle manifest also pins the exact FES MBR sector layout: `boot` starts
at sector 504 with length 16632, `rootfs` starts at 17136 with length 81900,
and autoresize `UDISK` starts at 99036. OpenixCLI compares every name, start,
and length with the MBR embedded in IMAGEWTY before opening USB. This prevents
an older but syntactically valid component image from being written by mistake.
The same preflight checks the expected file signatures for eGON Boot0, legacy
U-Boot Boot1, FIT, and UBIFS before it accepts their hashes.

T113 FES command `0x020e` returns the current UBI user logical area, not raw
SPI-NAND chip capacity. It can legally return zero before a usable UBI layout
exists. The manifest therefore declares `fes-logical-or-unavailable`: a
non-zero result must fit the raw-capacity upper bound and contain every fixed
FES volume; zero is reported as unavailable and never presented as a detected
256 MiB capacity. Storage type, board-specific loader, component identities,
and exact MBR remain mandatory gates.

The RAM-only IMAGEWTY loader must embed the board-matched FES U-Boot build that
accepts upstream legacy U-Boot and mainline eGON SPL components. The manifest
pins the capability markers `mainline u-boot size` and
`mainline eGON SPL size`; the bundle builder and OpenixCLI both reject an older
loader before USB is opened even if that loader is otherwise valid IMAGEWTY.

FES flash access has two distinct phases. The host uses `flash_set_on` before
the logical-capacity probe and keeps it active through MBR and ordinary
partition transfer. Once those partitions are verified, it must issue
`flash_set_off` before sending Boot1 and Boot0 through their dedicated FES
component commands. Keeping partition access active causes the Boot1 command to
fail with a USB protocol error.

## Safe workflow

Host-only validation opens no USB:

```sh
OPENIXCLI_BIN=$PWD/tools/OpenixCLI/target/release/openixcli \
FES_BUNDLE=$PWD/out/t113s3pro-mainline-fes make fes-preflight
```

The destructive hardware operation requires an explicit endpoint:

```sh
DEVICE_LOCATION=libusb:3:2 BUS=3 PORT=2 \
OPENIXCLI_BIN=$PWD/tools/OpenixCLI/target/release/openixcli \
./scripts/flash-fes-nand.sh ./out/t113s3pro-mainline-fes
```

The command deliberately leaves the board in FES. A separate controlled power
cycle and UART3 capture must prove `Trying to boot from sunxi SPI`, successful
UBI/UBIFS mount and `t113s3pro-mainline login:`.

That acceptance gate passed on 2026-08-26. Exact component hashes are in
`manifests/hardware-verified-fes-v5-20260826.sha256`; protocol outcomes are in
`logs/fes-hardware-validation-20260826.jsonl`; UART evidence is in
`logs/fes-v5-cold-boot-20260826.log`.

## Forbidden shortcuts

- Do not use SPI-NAND with the raw whole-disk route.
- Do not reuse historical component images or recovery loaders.
- Do not fall back to the RAM installer inside the same task.
- Do not automatically retry after erase begins.
- Do not mix UART output into the FES protocol log.
- Do not label protocol acknowledgement as readback or cold-boot verification.
