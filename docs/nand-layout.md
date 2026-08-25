# SPI-NAND layout

The validated 256 MiB development layout is fixed in both U-Boot and Linux
device trees:

| Region | Offset | Size | Purpose |
|---|---:|---:|---|
| `spl` | `0x00000000` | 1 MiB | repeated mainline eGON SPL |
| `uboot` | `0x00100000` | 4 MiB | mainline U-Boot proper image |
| `secure-storage` | `0x00500000` | 1 MiB | reserved, read-only |
| `boot` | `0x00600000` | 8 MiB | kernel/DTB FIT |
| `sys` | `0x00e00000` | 242 MiB | UBI containing `rootfs` |

The installer checks MTD name, size, write size and erase size before writing.
It refuses an unexpected geometry or an excessive bad-block count. It writes
with `nandwrite`, formats `sys` with `ubiformat`, attaches UBI, mounts UBIFS and
checks that `/sbin/init` exists.

The boot partitions are still a board-specific development layout. Do not
assume that fixed offsets and redundancy rules are a production provisioning
scheme for arbitrary NAND parts.
