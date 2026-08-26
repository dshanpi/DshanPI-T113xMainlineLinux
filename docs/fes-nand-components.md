# FES NAND component provisioning

Status: experimental until a complete FES write, physical power cycle, and
mainline cold boot have been observed on the target board.

This command implements a route that is deliberately separate from
`boot-mainline`:

```text
BootROM FEL
  -> board-specific vendor loader in RAM only
  -> FES NAND discovery and erase
  -> mainline Boot0 and Boot1
  -> mainline boot/rootfs components
  -> FES verification
  -> operator-controlled cold boot
```

`boot-mainline` remains an experimental recovery/diagnostic RAM installer. It
does not call this command, and this command does not load the mainline Linux
installer.

## Closed manifest

`flash-nand-components` requires a JSON manifest that pins:

- board, SoC, storage type, capacity, page size, and erase size;
- the RAM-only bootstrap IMAGEWTY file and SHA-256;
- the persistent component IMAGEWTY file and SHA-256;
- physical NAND region bounds;
- every FES MBR partition name, starting sector, and sector count;
- each loose component, its exact IMAGEWTY filename, and SHA-256.

Before USB is opened, OpenixCLI parses the component container, compares its
embedded MBR with the manifest, extracts every persistent component, and
compares its SHA-256 with the loose source artifact. Paths must be relative to
the manifest directory and cannot escape through `..` or symlinks.

## Hardware safety gates

- Only NAND or SPI-NAND is accepted after FES discovery.
- Detected capacity must exactly match the manifest.
- Only `partition_erase` or `full_erase` is accepted.
- The selected endpoint must be exactly `libusb:BUS:PORT`.
- FEL-to-FES reconnection remains bound to the same USB bus and port.
- Automatic retry is disabled. A failure requires a new manual FEL entry.
- Missing or failed Boot0/Boot1 verification is fatal.
- `--post-action none` leaves the device in FES for controlled power cycling.

Host-only validation does not open USB:

```sh
openixcli --output jsonl flash-nand-components \
  --manifest /path/to/bundle/manifest.json \
  --device-location libusb:3:2 --bus 3 --port 2 \
  --mode full_erase --verify --post-action none --preflight-only
```

Remove `--preflight-only` only after `scan --detailed` confirms the intended
board is in FEL. The operation is destructive.

## Output contract

With `--output jsonl`, stdout contains JSON objects only. Events include host
preflight, phase changes, current partition, byte progress, speed, protocol
logs, terminal completion, and terminal error. No ANSI progress bar is
created. This channel contains FES flashing protocol output only; UART belongs
to the independent serial terminal and must not be copied into it.

Successful media provisioning reports `coldBootStatus=not_observed`. FES
verification is not proof of a successful cold boot; that status changes only
after an external power-cycle observer records the mainline boot evidence.
