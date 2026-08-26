#!/usr/bin/env python3
"""Unit tests for the pinned upstream-source workflow."""

from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from source_lock import REQUIRED_KEYS, load_source_lock, validate_source_lock, verify_file


VALID_LOCK = """\
BUILDROOT_GIT_URL=https://gitlab.com/buildroot.org/buildroot.git
BUILDROOT_COMMIT=86102dd8279ac6c4c0244f3e490af98dc7460d5e
LINUX_VERSION=6.18.8
LINUX_ARCHIVE_URL=https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.18.8.tar.xz
LINUX_ARCHIVE_SHA256=37f0c5d5c242c1d604e87d48f08795e861a5a85f725b4ca11d0a538f12ff8cff
UBOOT_VERSION=2026.07
UBOOT_ARCHIVE_URL=https://ftp.denx.de/pub/u-boot/u-boot-2026.07.tar.bz2
UBOOT_ARCHIVE_SHA256=78e8bfc382fe388f9b55aa1daf8c563522a037779b5d4c349d1415e381f1243e
OPENIXCLI_GIT_URL=https://github.com/100askTeam/OpenixCLI.git
OPENIXCLI_COMMIT=de80fb95aabd3bd4f2afe1e355f9bc2f5bb94bca
"""


class SourceLockTests(unittest.TestCase):
    def write_lock(self, text: str) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "sources.lock"
        path.write_text(text, encoding="ascii")
        return path

    def test_exact_official_sources_are_accepted(self) -> None:
        lock = load_source_lock(self.write_lock(VALID_LOCK))
        validate_source_lock(lock)
        self.assertEqual(set(lock), REQUIRED_KEYS)

    def test_duplicate_and_unknown_keys_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate key"):
            load_source_lock(self.write_lock(VALID_LOCK + "LINUX_VERSION=6.18.9\n"))
        with self.assertRaisesRegex(ValueError, "unexpected source-lock keys"):
            validate_source_lock(load_source_lock(self.write_lock(VALID_LOCK + "MIRROR=http://example.com\n")))

    def test_non_official_or_mismatched_urls_are_rejected(self) -> None:
        unofficial = VALID_LOCK.replace("cdn.kernel.org", "example.com")
        with self.assertRaisesRegex(ValueError, "official kernel.org"):
            validate_source_lock(load_source_lock(self.write_lock(unofficial)))
        mismatched = VALID_LOCK.replace("linux-6.18.8.tar.xz", "linux-6.18.9.tar.xz")
        with self.assertRaisesRegex(ValueError, "Linux archive URL"):
            validate_source_lock(load_source_lock(self.write_lock(mismatched)))

    def test_checksum_verification_rejects_modified_archive(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "archive.tar.xz"
        archive.write_bytes(b"official-source")
        expected = hashlib.sha256(b"official-source").hexdigest()
        verify_file(archive, expected)
        archive.write_bytes(b"modified-source")
        with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
            verify_file(archive, expected)


if __name__ == "__main__":
    unittest.main()
