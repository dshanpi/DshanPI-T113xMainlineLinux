# SPI-NAND layout

The validated 256 MiB development layout is fixed in both U-Boot and Linux
device trees:

| Region | Offset | Size | Purpose |
|---|---:|---:|---|
| `spl` | `0x00000000` | 1 MiB | repeated mainline eGON SPL |
| `uboot` | `0x00100000` | 3 MiB | FES Boot1 reservation; mainline U-Boot proper |
| `secure-storage` | `0x00400000` | 1 MiB | FES-compatible reserved area, read-only |
| `sys` | `0x00500000` | 251 MiB | UBI; formal FES layout contains `boot`, `rootfs`, and autoresize `UDISK` volumes |

This boundary comes from the T113 vendor SPI-NAND implementation itself:
Boot0 uses blocks 0-7, Boot1 starts at block 8 and ends at block 32, and eight
additional eraseblocks are reserved for secure storage. With 128 KiB blocks,
the UBI `sys` region therefore begins at 5 MiB.

The installer checks MTD name, size, write size and erase size before writing.
It refuses an unexpected geometry or an excessive bad-block count. It writes
with `nandwrite`, formats `sys` with `ubiformat`, attaches UBI, mounts UBIFS and
checks the `boot` volume hash and that `/sbin/init` exists in `rootfs`.

The boot partitions are still a board-specific development layout. Do not
assume that fixed offsets and redundancy rules are a production provisioning
scheme for arbitrary NAND parts.
