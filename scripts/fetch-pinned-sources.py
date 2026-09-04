#!/usr/bin/env python3
"""Fetch Linux and U-Boot from their pinned official archives."""

from __future__ import annotations

import argparse
import os
import shutil
import tempfile
import time
import urllib.request
from pathlib import Path

from source_lock import load_source_lock, validate_source_lock, verify_file


def fetch(url: str, target: Path, expected: str, offline: bool) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_file():
        try:
            verify_file(target, expected)
            print(f"SOURCE_ARCHIVE_OK:{target}")
            return
        except ValueError:
            invalid = target.with_name(f"{target.name}.invalid-{int(time.time())}")
            target.replace(invalid)
            print(f"SOURCE_ARCHIVE_QUARANTINED:{invalid}")
    if offline:
        raise SystemExit(f"SOURCE_ARCHIVE_MISSING_OFFLINE:{target}")
    request = urllib.request.Request(url, headers={"User-Agent": "DshanPI-T113xMainlineLinux/1"})
    temporary: Path | None = None
    try:
        with urllib.request.urlopen(request, timeout=60) as response, tempfile.NamedTemporaryFile(
            prefix=f".{target.name}.", suffix=".part", dir=target.parent, delete=False
        ) as output:
            temporary = Path(output.name)
            shutil.copyfileobj(response, output, length=1024 * 1024)
            output.flush()
            os.fsync(output.fileno())
        verify_file(temporary, expected)
        os.replace(temporary, target)
        temporary = None
        print(f"SOURCE_ARCHIVE_FETCHED:{target}")
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--lock", type=Path, default=root / "manifests/sources.lock")
    parser.add_argument("--download-dir", type=Path, default=root / "buildroot/buildroot-mainline/dl")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    lock = load_source_lock(args.lock)
    validate_source_lock(lock)
    fetch(
        lock["LINUX_ARCHIVE_URL"],
        args.download_dir / "linux" / f'linux-{lock["LINUX_VERSION"]}.tar.xz',
        lock["LINUX_ARCHIVE_SHA256"],
        args.offline,
    )
    fetch(
        lock["UBOOT_ARCHIVE_URL"],
        args.download_dir / "uboot" / f'u-boot-{lock["UBOOT_VERSION"]}.tar.bz2',
        lock["UBOOT_ARCHIVE_SHA256"],
        args.offline,
    )


if __name__ == "__main__":
    main()
