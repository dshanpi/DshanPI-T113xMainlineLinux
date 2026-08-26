#!/usr/bin/env python3
"""Create a closed, hash-pinned T113S3 Pro FES NAND component bundle.

This does not create or modify IMAGEWTY images. The supplied component package
must already contain the exact files listed below. OpenixCLI independently
checks the container contents again before opening USB.
"""

import argparse
import hashlib
import json
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
BOOTSTRAP_MARKERS = [b"mainline u-boot size", b"mainline eGON SPL size"]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def copy(source: Path, output: Path, name: str) -> dict[str, str]:
    if not source.is_file():
        raise SystemExit(f"missing required FES artifact: {source}")
    destination = output / name
    shutil.copyfile(source, destination)
    return {"file": name, "sha256": sha256(destination)}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bootstrap", required=True, type=Path)
    parser.add_argument("--firmware-package", required=True, type=Path)
    parser.add_argument("--boot0", required=True, type=Path)
    parser.add_argument("--boot1", required=True, type=Path)
    parser.add_argument("--boot", required=True, type=Path)
    parser.add_argument("--rootfs", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    package_hash = sha256(args.firmware_package)
    retired = {
        line.split()[0]
        for line in (ROOT / "manifests/retired-fes-artifacts.txt").read_text().splitlines()
        if line and not line.startswith("#")
    }
    if package_hash in retired:
        raise SystemExit(f"refusing retired FES component package: {package_hash}")

    bootstrap_bytes = args.bootstrap.read_bytes()
    missing_markers = [marker.decode("ascii") for marker in BOOTSTRAP_MARKERS if marker not in bootstrap_bytes]
    if missing_markers:
        raise SystemExit(f"bootstrap loader lacks mainline component capability: {missing_markers}")

    args.output.mkdir(parents=True, exist_ok=True)
    bootstrap = copy(args.bootstrap, args.output, "bootstrap-loader.img")
    bootstrap["requiredMarkers"] = [marker.decode("ascii") for marker in BOOTSTRAP_MARKERS]
    package = copy(args.firmware_package, args.output, "mainline-nand-components.img")
    component_sources = {
        "boot0": copy(args.boot0, args.output, "boot0-mainline.bin"),
        "boot1": copy(args.boot1, args.output, "boot1-mainline.img"),
        "boot": copy(args.boot, args.output, "boot.itb"),
        "rootfs": copy(args.rootfs, args.output, "rootfs.ubifs"),
    }
    manifest = {
        "formatVersion": 1,
        "route": "fes_nand_components",
        "board": "dshanpi-t113s3pro",
        "soc": "r528",
        "bootstrap": bootstrap,
        "firmwarePackage": package,
        "storage": {
            "kind": "spi-nand",
            "capacityBytes": 268435456,
            "pageSize": 2048,
            "eraseSize": 131072,
            "capacityProbePolicy": "fes-logical-or-unavailable",
        },
        "layout": {
            "version": "t113s3pro-mainline-v1",
            "partitions": [
                {"name": "spl", "offset": 0, "size": 1048576},
                {"name": "uboot", "offset": 1048576, "size": 3145728},
                {"name": "secure-storage", "offset": 4194304, "size": 1048576},
                {"name": "sys", "offset": 5242880, "size": 263192576},
            ],
            "fesPartitions": [
                {"name": "boot", "addressSectors": 504, "sizeSectors": 16632},
                {"name": "rootfs", "addressSectors": 17136, "sizeSectors": 81900},
                {"name": "UDISK", "addressSectors": 99036, "sizeSectors": 0},
            ],
        },
        "components": [
            {"role": "boot0", "contentType": "egon-boot0", "partition": None, "packageFile": "boot0_nand.fex", **component_sources["boot0"]},
            {"role": "boot1", "contentType": "legacy-uboot", "partition": None, "packageFile": "u-boot.fex", **component_sources["boot1"]},
            {"role": "partition", "contentType": "fit", "partition": "boot", "packageFile": "boot.fex", **component_sources["boot"]},
            {"role": "partition", "contentType": "ubifs", "partition": "rootfs", "packageFile": "rootfs-ubifs.fex", **component_sources["rootfs"]},
        ],
    }
    manifest_path = args.output / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    checksums = sorted(path for path in args.output.iterdir() if path.is_file() and path.name != "SHA256SUMS")
    (args.output / "SHA256SUMS").write_text(
        "".join(f"{sha256(path)}  {path.name}\n" for path in checksums), encoding="ascii"
    )
    print(f"FES_NAND_BUNDLE:{args.output.resolve()}")
    print("STATUS:experimental-pending-cold-boot")


if __name__ == "__main__":
    main()
