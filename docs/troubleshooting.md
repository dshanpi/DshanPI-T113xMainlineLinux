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
