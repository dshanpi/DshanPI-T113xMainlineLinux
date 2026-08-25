#!/usr/bin/env python3
"""Create the bounded OpenixCLI plan for the generated T113S3 FEL bundle."""

import argparse
import hashlib
import json
from pathlib import Path


SPL_ADDRESS = 0x00020000
UBOOT_ADDRESS = 0x42E00000
INSTALLER_ADDRESS = 0x44000000
PAYLOAD_ADDRESS = 0x44800000


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def artifact(role: str, path: Path, address: int) -> dict:
    if not path.is_file():
        raise SystemExit(f"missing artifact: {path}")
    return {
        "role": role,
        "filePath": str(path.resolve()),
        "loadAddress": address,
        "sha256": digest(path),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    directory = args.artifacts.resolve()
    parts = sorted(directory.glob("fel-payload.part-*"))
    if not 1 <= len(parts) <= 5:
        raise SystemExit(f"expected 1..5 payload parts, found {len(parts)}")

    entries = [
        artifact("spl", directory / "fel-sunxi-spl.bin", SPL_ADDRESS),
        artifact("bootloader", directory / "fel-u-boot.bin", UBOOT_ADDRESS),
        artifact("kernel", directory / "fel-installer.itb", INSTALLER_ADDRESS),
    ]
    address = PAYLOAD_ADDRESS
    for part in parts:
        entries.append(artifact("initramfs", part, address))
        address += part.stat().st_size

    plan = {"artifacts": entries, "entryAddress": UBOOT_ADDRESS}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    print(args.output.resolve())


if __name__ == "__main__":
    main()
