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
