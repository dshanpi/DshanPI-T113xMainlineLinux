# DshanPI T113x Mainline Linux

Reproducible mainline Linux port for the DshanPi T113S3 Pro SPI-NAND board.

Validated stack:

- Allwinner T113-S3/R528, dual Cortex-A7, 128 MiB DDR3;
- Winbond W25N02KV 256 MiB SPI-NAND;
- UART3 on PB6/PB7 at 115200;
- U-Boot 2026.07;
- Linux 6.18.8;
- Buildroot at commit `86102dd8279ac6c4c0244f3e490af98dc7460d5e`;
- UBIFS root filesystem.

## Installation routes

The hardware-verified recovery/development path remains:

```text
BootROM FEL
  -> mainline U-Boot SPL in SRAM
  -> return to BootROM FEL
  -> mainline U-Boot proper in DRAM
  -> mainline Linux RAM installer
  -> Linux MTD/UBI writes and verifies SPI-NAND
  -> cold boot into the mainline system
```

The formal NAND provisioning route is implemented separately:

```text
BootROM FEL -> board-matched Tina loader in RAM -> FES
  -> Boot0(mainline SPL) -> Boot1(mainline U-Boot)
  -> boot.itb + sys.ubi -> FES verify -> power-off cold boot
```

The loader is transport/bootstrap only and is never selected as persistent
firmware. The exact v5 bundle completed FES verification and a separate
power-off cold boot on 2026-08-26; it is `hardware-verified` for the tested
board and NAND only. FES media completion alone must still never be presented
as cold-boot success. See
[`docs/fes-nand-provisioning.md`](docs/fes-nand-provisioning.md).

The RAM installer uses the mainline Linux NAND core, MTD tools and UBI. It is a
validated development and recovery mechanism for the tested board. It must not
be generalized into a claim of production NAND provisioning: BootROM/SPL boot
redundancy, OOB/ECC compatibility and bad-block behavior require separate
qualification for each NAND device and manufacturing process.

The earlier physical-layout bundle named in
[`manifests/verified-hardware-artifacts.sha256`](manifests/verified-hardware-artifacts.sha256)
has passed its complete gate twice. It remains historical evidence for the
board port, but does not qualify the newly aligned FES/UBI layout. The later clean repository rebuild is
internally valid but failed to start its RAM installer on hardware and is
explicitly `failed-do-not-use`. See
[`docs/verification-status.md`](docs/verification-status.md) before selecting
any artifact. The UART3 patch-loss root cause has since been repaired and a new
source-recovery candidate passes all local gates, but that exact candidate is
hardware-verified by a complete installer, warm reboot and two controlled
power-off cold boots for the previous layout. The exact hashes are in
`manifests/hardware-verified-source-rebuild-20260825.sha256`. This feature branch
remains preserved. The current FES-aligned source and v5 artifacts have now
passed their hardware gate, but this is not yet a general manufacturing NAND
qualification across bad-block populations.

## Build

Ubuntu/Debian host prerequisites include Git, Make, GCC, Python 3, `cpio`,
`zstd`, `rsync`, development libraries required by Buildroot, and enough disk
space for a complete cross-build.

```sh
git clone -b feat/fes-nand-components \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/one-click-build.sh
```

The one-click script performs the following operations:

1. clones the pinned OpenixCLI and Buildroot trees;
2. installs the DshanPi defconfig and board support;
3. downloads Linux 6.18.8 from kernel.org and U-Boot 2026.07 from DENX, then verifies their pinned SHA-256 values;
4. builds the toolchain, U-Boot, kernel, DTB and UBIFS root filesystem;
5. builds the self-contained mainline Linux RAM installer;
6. tests and builds the pinned OpenixCLI release binary;
7. emits the bounded FEL artifact bundle under `out/t113s3pro-mainline-fel`;
8. runs all repository, source-lock and artifact validation gates.

The exact source versions and archive hashes are recorded in
[`manifests/sources.lock`](manifests/sources.lock).

For the formal FES NAND package, pass authorized local Tina packaging tools and
the board-specific RAM loader:

```sh
TINA_SDK=/absolute/path/to/authorized/Tina-SDK \
FES_BOOTSTRAP_LOADER=/absolute/path/to/authorized/loader.img \
./scripts/one-click-build.sh
```

Those two inputs are not downloadable public dependencies in this repository and
are never committed because their redistribution permission is not established.
The script builds everything else without them and clearly reports only the FES
packaging stage as skipped. Building never writes a connected board.

## Flash

The one-repository workflow above obtains the pinned OpenixCLI automatically. An
existing exact OpenixCLI checkout can instead be selected explicitly:

```sh
OPENIXCLI_DIR=/absolute/path/to/OpenixCLI ./scripts/one-click-build.sh
```

Connect the board in FEL mode and keep the independent UART3 adapter available.
With exactly one Allwinner device attached, the workflow can select its physical
USB endpoint automatically and wait for board-side installer plus reboot evidence:

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

For FEL RAM handoff only, without UART acceptance monitoring:

```sh
./scripts/flash-mainline-fel.sh \
  ./out/t113s3pro-mainline-fel \
  auto
```

Do not disconnect USB, UART or power while the installer is active. A complete
success requires all of the following:

- every host artifact SHA-256 matches;
- SPL returns to FEL;
- the installer validates its RAM payload;
- SPL, U-Boot and boot FIT readback hashes pass;
- UBI attaches `sys` and UBIFS mounts `rootfs`;
- a real power cycle reaches `t113s3pro-mainline login:` without manual U-Boot
  commands.

`flash-and-monitor.sh` proves installer completion followed by a reboot to the
login prompt. It does not claim a power-off cold boot. Release qualification
still requires a controlled power-off interval and a fresh UART capture.

## Documentation

- [中文：镜像说明与 FEL 烧录步骤](docs/images-and-fel-flashing.zh-CN.md)
- [2026-09-04 FEL release candidate notes](docs/releases/t113s3pro-mainline-fel-20260904-rc1.md)
- [Hardware facts](docs/hardware.md)
- [Porting order](docs/porting-order.md)
- [Boot and installation chain](docs/boot-chain.md)
- [SPI-NAND layout](docs/nand-layout.md)
- [Reproducibility](docs/reproducibility.md)
- [Development journal](docs/development-journal.md)
- [Verification status](docs/verification-status.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Automated two-repository workflow](docs/automated-workflow.md)
- [Frozen FES experiments](docs/frozen-fes-experiments.md)
- [Active FES NAND provisioning design](docs/fes-nand-provisioning.md)
- [Logs and evidence](logs/README.md)
- [Latest line-by-line local validation](logs/local-validation-20260825.txt)
