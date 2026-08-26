#!/usr/bin/env python3
"""Strict parser and verifier for the reproducible upstream source lock."""

from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path
from urllib.parse import urlparse


REQUIRED_KEYS = {
    "BUILDROOT_GIT_URL",
    "BUILDROOT_COMMIT",
    "LINUX_VERSION",
    "LINUX_ARCHIVE_URL",
    "LINUX_ARCHIVE_SHA256",
    "UBOOT_VERSION",
    "UBOOT_ARCHIVE_URL",
    "UBOOT_ARCHIVE_SHA256",
    "OPENIXCLI_GIT_URL",
    "OPENIXCLI_COMMIT",
}


def load_source_lock(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for number, raw in enumerate(path.read_text(encoding="ascii").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            raise ValueError(f"malformed source-lock line {number}")
        key, value = line.split("=", 1)
        if not re.fullmatch(r"[A-Z][A-Z0-9_]*", key) or not value or value != value.strip():
            raise ValueError(f"invalid source-lock assignment on line {number}")
        if key in values:
            raise ValueError(f"duplicate key: {key}")
        values[key] = value
    return values


def _exact_url(value: str, scheme: str, host: str, path: str, label: str) -> None:
    parsed = urlparse(value)
    if (parsed.scheme, parsed.hostname, parsed.path, parsed.query, parsed.fragment) != (
        scheme,
        host,
        path,
        "",
        "",
    ):
        raise ValueError(f"{label} must use the pinned official URL")


def validate_source_lock(lock: dict[str, str]) -> None:
    missing = REQUIRED_KEYS - set(lock)
    extra = set(lock) - REQUIRED_KEYS
    if missing or extra:
        raise ValueError(f"unexpected source-lock keys: missing={sorted(missing)} extra={sorted(extra)}")
    for key in ("BUILDROOT_COMMIT", "OPENIXCLI_COMMIT"):
        if not re.fullmatch(r"[0-9a-f]{40}", lock[key]):
            raise ValueError(f"{key} must be a full lowercase Git commit")
    for key in ("LINUX_ARCHIVE_SHA256", "UBOOT_ARCHIVE_SHA256"):
        if not re.fullmatch(r"[0-9a-f]{64}", lock[key]):
            raise ValueError(f"{key} must be a lowercase SHA-256")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", lock["LINUX_VERSION"]):
        raise ValueError("LINUX_VERSION must be MAJOR.MINOR.PATCH")
    if not re.fullmatch(r"[0-9]{4}\.[0-9]{2}", lock["UBOOT_VERSION"]):
        raise ValueError("UBOOT_VERSION must be YYYY.MM")
    _exact_url(
        lock["BUILDROOT_GIT_URL"],
        "https",
        "gitlab.com",
        "/buildroot.org/buildroot.git",
        "Buildroot repository",
    )
    _exact_url(
        lock["LINUX_ARCHIVE_URL"],
        "https",
        "cdn.kernel.org",
        f'/pub/linux/kernel/v6.x/linux-{lock["LINUX_VERSION"]}.tar.xz',
        "Linux archive URL on official kernel.org CDN",
    )
    _exact_url(
        lock["UBOOT_ARCHIVE_URL"],
        "https",
        "ftp.denx.de",
        f'/pub/u-boot/u-boot-{lock["UBOOT_VERSION"]}.tar.bz2',
        "U-Boot archive URL on official DENX server",
    )
    _exact_url(
        lock["OPENIXCLI_GIT_URL"],
        "https",
        "github.com",
        "/100askTeam/OpenixCLI.git",
        "OpenixCLI repository",
    )


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verify_file(path: Path, expected: str) -> None:
    actual = file_sha256(path)
    if actual != expected:
        raise ValueError(f"SHA-256 mismatch for {path.name}: expected={expected} actual={actual}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("lock", type=Path)
    args = parser.parse_args()
    lock = load_source_lock(args.lock)
    validate_source_lock(lock)
    print(f"SOURCE_LOCK_OK:{args.lock}")


if __name__ == "__main__":
    main()
