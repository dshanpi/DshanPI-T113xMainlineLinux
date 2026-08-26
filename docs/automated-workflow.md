# Automated two-repository workflow

## Contract

The supported source workflow uses exactly these branches:

- `100askTeam/OpenixCLI:feat/mainline-fel-ram-installer`;
- `dshanpi/DshanPI-T113xMainlineLinux:feat/t113s3pro-mainline`.

Clone the repositories beside each other. External upstream sources are not
vendored: the board repository downloads checksum-pinned Buildroot, Linux, and
U-Boot inputs, while OpenixCLI uses a revision-pinned libefex dependency.

## Build

```sh
git clone -b feat/mainline-fel-ram-installer \
  https://github.com/100askTeam/OpenixCLI.git
git clone -b feat/t113s3pro-mainline \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git
cd DshanPI-T113xMainlineLinux
./scripts/build-everything.sh
```

`build-everything.sh` refuses missing host tools, runs the locked OpenixCLI test
suite, builds its release binary, builds every board artifact, packages the FEL
RAM installer, and executes the local acceptance gates. Set `OPENIXCLI_DIR` only
when the companion repository is not the normal sibling directory.

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
