# Logs and acceptance evidence

`t113-task-history-20260824-25.jsonl` contains every exported T113/project-3
task row from the bring-up period, including failures and frozen FES
experiments. Absolute local paths are replaced with symbolic workspace paths.

`final-cold-boot.log` is the successful power-cycle UART capture. Acceptance
requires these markers:

```text
Trying to boot from sunxi SPI
U-Boot 2026.07
Kernel command line: ... ubi.mtd=sys root=ubi0:rootfs ...
ubi0: attached mtd4 (name "sys", size 242 MiB)
VFS: Mounted root (ubifs filesystem)
t113s3pro-mainline login:
```

The public repository intentionally excludes the raw Lynx SQLite database
because it may contain unrelated board and host metadata.

`local-validation-20260825.txt` records every non-hardware gate from the clean
repository rebuild. Re-run the same gates with `make validate`. Those gates do
not override the later hardware failure of the clean-build hashes.

`hardware-revalidation-20260825.jsonl` records only device 5's three new
installer outcomes and the final relay power cycle. It contains no host paths,
unrelated devices, user data or controller serial number.

`hardware-revalidation-cold-boot-20260825.log` contains the UART markers
observed after the final two-second power cycle. Together they prove the
preserved hardware baseline again; they do not rehabilitate the failed clean
rebuild.

`fes-host-validation-20260826.jsonl` records the new formal FES-layout package
hashes, closed-manifest preflight, JSONL failure-path gate, and 75 local source
checks. It explicitly records `hardwareStatus=pending`; it is not cold-boot
evidence.

`fes-hardware-validation-20260826.jsonl` records each destructive-route attempt.
The first attempt stopped before erase because FES identified SPI-NAND but the
host queried capacity before selecting storage type 5. The second attempt
disproved the initial `flash_set_off` hypothesis and also stopped before erase.
Tina source inspection established that command `0x020e` exposes UBI logical
user capacity, which may be unavailable before layout creation, rather than raw
chip capacity. Later rows must retain both failures rather than replacing them.
The third attempt passed those gates and verified MBR, boot, and rootfs, then
failed before Boot1/Boot0 because partition flash access had not been closed.
It records the required `set_on` partition phase followed by `set_off` before
the dedicated boot-component phase.
