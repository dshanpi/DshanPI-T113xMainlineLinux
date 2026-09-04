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

## Scope of this version

This version supports one installation path:

```text
BootROM FEL
  -> mainline U-Boot SPL in SRAM
  -> return to BootROM FEL
  -> mainline U-Boot proper in DRAM
  -> mainline Linux RAM installer
  -> Linux MTD/UBI writes and verifies SPI-NAND
  -> cold boot into the mainline system
```

No Tina/IMAGEWTY loader or FES service is used by this supported path. Earlier
FES NAND-component experiments are preserved only as historical records in
[`docs/frozen-fes-experiments.md`](docs/frozen-fes-experiments.md); their code
is not part of this release.

The RAM installer uses the mainline Linux NAND core, MTD tools and UBI. It is a
validated development and recovery mechanism for the tested board. It must not
be generalized into a claim of production NAND provisioning: BootROM/SPL boot
redundancy, OOB/ECC compatibility and bad-block behavior require separate
qualification for each NAND device and manufacturing process.

The exact preserved bundle named in
[`manifests/verified-hardware-artifacts.sha256`](manifests/verified-hardware-artifacts.sha256)
has passed this complete gate twice. The later clean repository rebuild is
internally valid but failed to start its RAM installer on hardware and is
explicitly `failed-do-not-use`. See
[`docs/verification-status.md`](docs/verification-status.md) before selecting
any artifact. The UART3 patch-loss root cause has since been repaired and a new
source-recovery candidate passes all local gates, but that exact candidate is
now hardware-verified by a complete installer, warm reboot and two controlled
power-off cold boots. The exact hashes are in
`manifests/hardware-verified-source-rebuild-20260825.sha256`. This feature branch
remains a development/recovery release rather than a manufacturing NAND claim.

## Build

Ubuntu/Debian host prerequisites include Git, Make, GCC, Python 3, `cpio`,
`zstd`, `rsync`, development libraries required by Buildroot, and enough disk
space for a complete cross-build.

```sh
git clone -b feat/t113s3pro-mainline \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
make all
```

`make all` performs the following operations:

1. clones the pinned Buildroot tree;
2. installs the DshanPi defconfig and board support;
3. downloads checksum-verified Linux 6.18.8 and U-Boot 2026.07 sources;
4. builds the toolchain, U-Boot, kernel, DTB and UBIFS root filesystem;
5. builds the self-contained mainline Linux RAM installer;
6. emits the bounded FEL artifact bundle under `out/t113s3pro-mainline-fel`;
7. runs all repository and artifact validation gates.

The exact source versions and archive hashes are recorded in
[`manifests/sources.lock`](manifests/sources.lock).

## Flash

For a clean two-repository build, clone both feature branches beside each other:

```sh
git clone -b feat/mainline-fel-ram-installer \
  https://github.com/100askTeam/OpenixCLI.git
git clone -b feat/t113s3pro-mainline \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/build-everything.sh
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

- [中文：纯主线 FEL 镜像与烧录步骤](docs/images-and-fel-flashing.zh-CN.md)
- [2026-09-04 pure FEL release notes](docs/releases/t113s3pro-mainline-fel-20260904-rc1.md)
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
- [Logs and evidence](logs/README.md)
- [Latest line-by-line local validation](logs/local-validation-20260825.txt)
