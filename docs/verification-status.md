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

The same hardware-proven artifact set was exercised again later on 2026-08-25.
Task `mainline-1787709324680503509` completed at 100% with exit code 0. Lynx
Power then switched device 5 off for two seconds and back on through relay
channel 6. FEL disappeared, the board mounted the `sys` UBI device and reached
`t113s3pro-mainline login:` without a manual U-Boot command. The sanitized
task and power records are in
[`../logs/hardware-revalidation-20260825.jsonl`](../logs/hardware-revalidation-20260825.jsonl),
and the observed UART markers are in
[`../logs/hardware-revalidation-cold-boot-20260825.log`](../logs/hardware-revalidation-cold-boot-20260825.log).

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
the hardware-proven baseline. Hardware task `mainline-1787708850567538011`
loaded all of those clean-build artifacts into RAM but never emitted an
installer marker. Lynx terminated the task with
`MAINLINE_INSTALLER_TIMEOUT:no completion marker within 180 seconds` at 50%.

The hashes in `clean-build-20260825.sha256` are therefore classified
**failed-do-not-use**, not pending. They must not be flashed again until the
source drift is identified, a new artifact set is generated, and the complete
install/readback/power-cycle gate passes. Passing local syntax, format and hash
checks did not prove that this exact rebuild could execute on hardware.

## Required source-recovery gate

Reconcile the clean repository against the exact source/build inputs that
produced `verified-hardware-artifacts.sha256`. Rebuild from a new clone, run the
generated plan through the companion OpenixCLI branch, retain both its JSONL
protocol output and independent UART capture, then power-cycle the board.
Acceptance requires the markers listed in [`../logs/README.md`](../logs/README.md).
Do not automatically retry a terminal FEL failure; return the board to FEL
manually and start a new task.
