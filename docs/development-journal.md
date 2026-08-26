# Development journal

## Result

On 2026-08-25 the T113S3 Pro completed a real power-cycle boot from SPI-NAND:

```text
Trying to boot from sunxi SPI
U-Boot 2026.07 (Aug 25 2026 - 07:00:31 -0400) DshanPi T113S3 Pro
Verifying Hash Integrity ... sha256+ OK
Kernel command line: ... ubi.mtd=sys root=ubi0:rootfs ...
ubi0: attached mtd4 (name "sys", size 242 MiB)
VFS: Mounted root (ubifs filesystem)
DshanPi T113S3 Pro - mainline Buildroot
t113s3pro-mainline login:
```

Final installer task: `mainline-1787655837814079629`, status `success`, phase
`complete`, progress 100%, exit code 0.

The repository was subsequently reconstructed from a clean clone and rebuilt.
That rebuild passes all local gates but has different artifacts. Hardware task
`mainline-1787708850567538011` proved that this distinction is material: it
reached the 50% RAM-installer handoff but timed out without the first installer
marker. Those hashes are now `failed-do-not-use`, not merely pending. See
[`verification-status.md`](verification-status.md) for the separation between
the hardware-proven baseline and the current clean build.

The preserved hardware-proven bundle was then selected by exact SHA-256.
Task `mainline-1787709324680503509` completed the NAND installer at 100% with
exit code 0. A two-second Lynx Power cycle on device 5/channel 6 cold-booted
the board through SPI-NAND, mounted UBIFS and reached the login prompt again.

An earlier task, `mainline-1787708569776828829`, failed because a diagnostic
client closed Lynx's shared `/dev/ttyACM0` handle while the installer monitor
was active. That failure is classified as an operator/tooling error and is not
evidence against either artifact set. The gateway must observe task state
without opening or closing the UI-owned serial session.

## Failure sequence and fixes

1. **FEL USB unavailable in the VM.** OpenixCLI failed with USB initialization
   errors until `1f3a:efe8` was explicitly attached to the guest.
2. **SPL return failed.** R528 SRAM overlap and BootROM clock state required an
   audited SRAM swap/return thunk, preserved PLL state and bounded reopen logic.
3. **Wrong boot source.** Cold boot stopped at `Unknown boot source 4`; value 4
   was added as R528 SPI-NAND.
4. **Environment reset.** U-Boot repeatedly reset while loading an unavailable
   environment. The board now uses `ENV_IS_NOWHERE`.
5. **Conflicting layouts.** Older trees used 3 MiB U-Boot and started boot at
   4 MiB. All components were aligned to the final 1/4/1/8/242 MiB layout.
6. **Quad read failure.** U-Boot proper read physical NAND as all `0xff` while
   SPL and Linux could read it. Removing forced quad width from the U-Boot-only
   DTS restored valid eGON and FIT magic reads.
7. **Wrong UBI parameter.** `ubi.mtd=rootfs` referred to a volume name rather
   than the MTD partition. Changing it to `ubi.mtd=sys` allowed UBIFS root mount.
8. **Manual boot was not accepted.** A temporary U-Boot `setenv` proved the
   rootfs, but the result was accepted only after rebuilding, reflashing and
   cold-booting with the permanent argument.
9. **Local gates were mistaken for hardware readiness.** The reconstructed
   source produced internally consistent files but its installer never
   started on the board. Artifact status is now an explicit four-way label:
   `verified`, `experimental`, `failed-do-not-use`, or `recovery-only`.

## Evidence policy

All T113/project-3 task rows were exported before cleanup. The public JSONL is
path-sanitized. The unredacted SQLite database, complete local worktrees,
remote build tree and every artifact iteration are retained in the private
pre-clean archive described by its local `README.md` and `SHA256SUMS`.

## Two-repository automation hardening

After the first source publication audit, the workflow was tightened so a clean
machine does not silently clone default branches or mistake FEL transfer for
NAND success:

1. all clone instructions name the two required feature branches;
2. OpenixCLI pins libefex revision `3752e38ff8e69190c53cd43290a8102beab55e73`;
3. OpenixCLI JSONL scan emits numeric USB identity and stable physical location;
4. the mainline completion event is explicitly scoped to `fel_ram_handoff` and
   reports `installerStatus=not_observed`;
5. the board repository builds and tests both repositories through
   `scripts/build-everything.sh`;
6. automatic USB selection accepts exactly one Allwinner device and refuses
   ambiguous candidates;
7. the independent UART acceptance monitor requires board-side installer
   completion before accepting a subsequent login prompt;
8. warm-reboot acceptance and power-off cold-boot qualification are documented
   as separate gates.

These changes improve repeatability and reporting but do not change the NAND
layout. After the automation changes, the rebuilt source candidate was loaded
in task `mainline-1787715829104265529`. The Linux installer wrote and verified
the complete NAND layout, rebooted into UBIFS, and reached the login prompt.
Two subsequent Lynx Power cycles with at least two seconds fully off also
reached the login prompt. The source-recovery candidate was therefore promoted
to `hardware-verified` for the tested T113S3 Pro/W25N02KV combination.

## Formal FES NAND route, 2026-08-26

The historical 1/4/1/8/242 MiB layout above belongs to the verified mainline
RAM installer. It is retained as recovery evidence, not used as the formal FES
component layout.

Inspection of the authorized Tina SPI-NAND FES source established the vendor
physical reservations: Boot0 blocks 0-7 (1 MiB), Boot1 blocks 8-31 (3 MiB),
secure storage blocks 32-39 (1 MiB), and the UBI region from 5 MiB onward. The
current mainline DTS, U-Boot command, bundle schema, and FES package generator
were migrated together to that contract. FES MBR entries are pinned to
`boot=504/16632`, `rootfs=17136/81900`, and `UDISK=99036/0` sectors.

OpenixCLI gained a separate `flash-nand-components` command. It uses the vendor
loader only for RAM bootstrap, keeps the persistent mainline package separate,
checks every SHA-256 and embedded file signature, checks the exact MBR, binds
FEL/FES to one USB topology, rejects non-NAND media/capacity mismatches, and
treats erase, MBR, partition, Boot0, and Boot1 verification failures as fatal.
Automatic task retry is disabled. Its JSONL stream contains protocol progress
only and never UART.

Host validation and a real component-package build passed. No USB device was
present for the final destructive gate, so the new layout remains
`experimental-pending-cold-boot`. It must not inherit the older RAM-installer
hardware status. The exact host evidence is recorded in
`logs/fes-host-validation-20260826.jsonl`.

The first hardware execution reached FES and correctly identified SPI-NAND,
then stopped before erase because the capacity query returned zero. The first
hypothesis was that the detected storage type had to be selected with
`flash_set_off`; a second manual-FEL attempt disproved it and again stopped
before erase. Direct inspection of Tina's `usb_efex.c` and SPI-NAND backend
showed that `flash_set_off` deinitializes sprite, while FES command `0x020e`
returns the current UBI logical user-volume size rather than raw chip capacity.
That value may legally be zero before a usable layout exists.

The formal policy now initializes SPI-NAND with `flash_set_on` and treats the
probe as logical capacity or explicitly unavailable. A nonzero value must
contain the fixed FES layout through sector 99036 and must not exceed the pinned
256 MiB raw bound. Zero is logged as unavailable and is never presented as a
detected raw capacity. Board identity, storage type, loader identity, component
hashes, and the exact MBR remain hard pre-erase gates. Per the no-retry rule,
neither FES session was reused.

The third manual-FEL attempt passed the corrected capacity policy, wrote and
verified the exact MBR, then wrote and verified both `boot` and `rootfs`. The
first Boot1 transfer returned a USB protocol error before Boot1 or Boot0 could
be verified. This established the second half of the vendor lifecycle:
`flash_set_on` must remain active for ordinary partition I/O, but
`flash_set_off` is mandatory after partition verification and before the
separate FES Boot1/Boot0 component commands. The partially provisioned device
was left in FES and was not retried.
