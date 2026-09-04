# Troubleshooting

## FEL is not found

- `lsusb` must show `1f3a:efe8` inside the same host/VM that runs OpenixCLI.
- In VMware, explicitly attach the Allwinner device to the guest after every
  USB re-enumeration.
- Do not start a flash while the saved physical binding is offline.

## `Unknown boot source 4`

The R528 BootROM reports SPI-NAND as media value 4. Confirm that
`0004-sunxi-recognize-r528-spinand-boot-source.patch` was applied.

## Reset at `Loading Environment`

Confirm `CONFIG_ENV_IS_NOWHERE=y` and that FAT/SPI environment backends are
disabled. The board has no usable MMC environment in this configuration.

## U-Boot reads all `ff`

Do not force quad data width in the U-Boot SPI-NAND flash node. The validated
U-Boot path uses single-I/O reads. Linux has a separate DTS and may use quad
mode after its SPI-NAND core is active.

## `UBI error: cannot open mtd rootfs`

The MTD partition is named `sys`; the UBI volume is named `rootfs`. The correct
pair is `ubi.mtd=sys root=ubi0:rootfs`.

## Host reaches 50% and waits

FEL transfer is only the first half. Inspect UART for installer startup,
payload hash validation, MTD geometry and `LYNX_PROGRESS` records. Do not report
success from transferred bytes alone.

## `USB initialization failed`

Another process or the host OS may own the endpoint, or the VM may not have
captured it. Stop background scans, confirm one owner, re-enter FEL manually and
start a new task. Failed tasks are never retried automatically.

## U-Boot reports `Unknown command 'efex'`

The current mainline U-Boot configuration does not provide the vendor `efex`
command. Re-enter FEL with the board's physical FEL strap/button and reset or
power cycle. Do not treat repeated serial commands as a valid retry path.

## MCP times out at `installer_verify_rootfs` 94%

The MCP monitor has a 180-second completion window, while the Linux installer
runs independently on the board after FEL RAM handoff. A timeout is terminal
for that host task but does not prove that the board stopped. Continue passive
UART observation without closing the shared serial handle, retrying, or cycling
power. Accept the installation only if UART later shows `installer_complete`,
the reboot reaches the login prompt, and a separate cold boot passes.

Task `mainline-1788508715752023200` on 2026-09-04 wrote and read back SPL and
U-Boot, formatted `sys.ubi`, and attached its `boot/rootfs` volumes, but emitted
no completion marker before the timeout. That exact run is incomplete, not a
successful flash qualification.

## `nandwrite` warns about blocks containing only `0xff`

The redundant SPL and reserved U-Boot partition images intentionally contain
padding filled with `0xff`. This warning alone is not a write failure. Require
the subsequent `/dev/mtd0 readback SHA-256 OK` and `/dev/mtd1 readback SHA-256
OK` markers.
