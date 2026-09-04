# Automated two-repository workflow

## Contract

The supported source workflow pins exact commits, not moving branch tips:

- OpenixCLI `de80fb95aabd3bd4f2afe1e355f9bc2f5bb94bca`;
- this repository's tested `feat/fes-nand-components` revision.

External upstream sources are not vendored. The board repository downloads and
verifies the following immutable inputs:

| Input | Version/revision | Official source |
| --- | --- | --- |
| Buildroot | `86102dd8279ac6c4c0244f3e490af98dc7460d5e` | `https://gitlab.com/buildroot.org/buildroot.git` |
| Linux | `6.18.8` | `https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.18.8.tar.xz` |
| U-Boot | `2026.07` | `https://ftp.denx.de/pub/u-boot/u-boot-2026.07.tar.bz2` |
| OpenixCLI | `de80fb95aabd3bd4f2afe1e355f9bc2f5bb94bca` | `https://github.com/100askTeam/OpenixCLI.git` |

Linux and U-Boot archives are verified against SHA-256 values in
`manifests/sources.lock`. Buildroot and OpenixCLI are checked out by full Git
commit. A URL, version, hash, missing key, or extra key mismatch fails closed.

## Build

```sh
git clone -b feat/fes-nand-components \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/one-click-build.sh
```

The script automatically clones the pinned OpenixCLI commit into `.deps/` when
there is no sibling checkout. It refuses missing host tools, a dirty explicitly
selected OpenixCLI checkout, or a revision mismatch. It then runs the locked
OpenixCLI tests, builds its release binary, builds every board artifact, packages
the FEL RAM installer, and executes all local acceptance gates.

The mainline source build is completely automatic. Creating the formal FES NAND
container additionally requires Allwinner/Tina packaging tools and a board loader
whose redistribution license is not present in this repository. Supply authorized
local inputs without copying them into Git:

```sh
TINA_SDK=/absolute/path/to/authorized/Tina-SDK \
FES_BOOTSTRAP_LOADER=/absolute/path/to/authorized/loader.img \
./scripts/one-click-build.sh
```

With both variables, the script also builds the FES component package, creates a
hash-pinned closed bundle, and runs OpenixCLI's no-USB preflight. It never starts
a media write automatically. Without them, only this licensed packaging stage is
reported as skipped; U-Boot, Linux, Buildroot, rootfs, FIT images, recovery bundle,
OpenixCLI, and their validation are still built completely.

## Flash and warm-reboot acceptance

Connect UART3 PB6/PB7 at 115200 through an independent USB serial adapter. Put
the T113S3 Pro into FEL and run:

```sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

The `auto` selector accepts exactly one USB device with Allwinner VID `0x1f3a`.
It fails rather than guessing if no candidate or more than one candidate exists.
An explicit `libusb:BUS:PORT` may be supplied instead.

The two channels retain different meanings:

- OpenixCLI JSONL reports only FEL validation, transfer, SPL return, and RAM
  handoff. Its terminal event is scoped as `fel_ram_handoff` and carries
  `installerStatus=not_observed`.
- The UART monitor observes the board-side Linux installer. It accepts only an
  `installer_complete` marker followed by `t113s3pro-mainline login:`. It writes
  the raw UART capture under `out/flash-logs/`.

An installer failure marker, timeout, USB failure, missing serial device, or
ambiguous USB selection produces a nonzero exit. There is no automatic retry
after a terminal FEL failure; return the board to FEL manually before restarting.

## Cold-boot qualification

The installer performs a software reboot after verified NAND writes. Passing
`flash-and-monitor.sh` therefore proves a verified installation and warm reboot,
not loss-of-power retention. Before publishing a newly built artifact set:

1. switch board power fully off for at least one second;
2. start a fresh UART capture;
3. restore power without FEL straps or manual U-Boot commands;
4. require `Trying to boot from sunxi SPI`, UBI attachment, UBIFS mount, and the
   mainline login prompt;
5. archive the UART log, artifact SHA-256, source commits, and task result.

Power relay integration is intentionally outside the generic repository because
relay identity and safety policy are installation-specific.
