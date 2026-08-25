# Verification status

## Hardware-proven baseline

On 2026-08-25 installer task `mainline-1787655837814079629` completed with
exit code 0. A subsequent real power cycle booted from SPI-NAND, attached the
`sys` UBI device, mounted the `rootfs` UBIFS volume and reached
`t113s3pro-mainline login:`. The UART evidence is preserved in
[`../logs/final-cold-boot.log`](../logs/final-cold-boot.log), and the exact
host-side artifact hashes used by that run are preserved in
[`../manifests/verified-hardware-artifacts.sha256`](../manifests/verified-hardware-artifacts.sha256).

This proves the board port and the pure-mainline RAM-installer design on the
tested DshanPi T113S3 Pro with a Winbond W25N02KV. It is not a blanket claim
for other NAND parts, layouts or manufacturing bad-block populations.

## Clean repository rebuild

The source tree was reconstructed from a clean clone and rebuilt on
2026-08-25. The following local gates passed:

- pinned Buildroot commit resolved exactly;
- all four U-Boot board patches applied to pristine U-Boot 2026.07 sources;
- U-Boot SPL and proper shared one build identity;
- SPL eGON magic, checksum and exact length passed;
- U-Boot proper load and entry addresses passed;
- Linux 6.18.8, DTB, installer initramfs, FIT images and `sys.ubi` built;
- the bounded OpenixCLI load plan parsed as valid JSON;
- installer scripts passed shell syntax checks and Python helpers compiled;
- the packaged artifact hashes are recorded in
  [`../manifests/clean-build-20260825.sha256`](../manifests/clean-build-20260825.sha256).

Build timestamps and UBI image metadata make this clean rebuild differ from
the hardware-proven baseline. The clean-build hashes therefore remain
**hardware cold-boot revalidation pending**. They must not be published or
described as a stable manufacturing release until a new FEL installation,
readback verification and power-cycle boot have passed on the target board.

## Required final gate

When hardware is available, run the generated plan through the companion
OpenixCLI branch, retain both its JSONL protocol output and the independent
UART capture, then power-cycle the board. Acceptance requires the markers
listed in [`../logs/README.md`](../logs/README.md). Do not automatically retry
a terminal FEL failure; return the board to FEL manually and start a new task.
