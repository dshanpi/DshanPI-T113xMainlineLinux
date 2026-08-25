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

## Build

Ubuntu/Debian host prerequisites include Git, Make, GCC, Python 3, `cpio`,
`zstd`, `rsync`, development libraries required by Buildroot, and enough disk
space for a complete cross-build.

```sh
git clone https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
make all
```

`make all` performs the following operations:

1. clones the pinned Buildroot tree;
2. installs the DshanPi defconfig and board support;
3. downloads checksum-verified Linux 6.18.8 and U-Boot 2026.07 sources;
4. builds the toolchain, U-Boot, kernel, DTB and UBIFS root filesystem;
5. builds the self-contained mainline Linux RAM installer;
6. emits the bounded FEL artifact bundle under `out/t113s3pro-mainline-fel`.

The exact source versions and archive hashes are recorded in
[`manifests/sources.lock`](manifests/sources.lock).

## Flash

Build OpenixCLI from the companion repository, connect the board in FEL mode,
and use the stable location reported by `openixcli scan`:

```sh
./scripts/flash-mainline-fel.sh \
  ./out/t113s3pro-mainline-fel \
  libusb:3:2
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

## Documentation

- [Hardware facts](docs/hardware.md)
- [Porting order](docs/porting-order.md)
- [Boot and installation chain](docs/boot-chain.md)
- [SPI-NAND layout](docs/nand-layout.md)
- [Reproducibility](docs/reproducibility.md)
- [Development journal](docs/development-journal.md)
- [Verification status](docs/verification-status.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Frozen FES experiments](docs/frozen-fes-experiments.md)
- [Logs and evidence](logs/README.md)
- [Latest line-by-line local validation](logs/local-validation-20260825.txt)
