#!/usr/bin/env python3
"""Build, inspect, and verify RAM-only Allwinner IMAGEWTY loaders."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
HEADER_SIZE = 1024
MAGIC = b"IMAGEWTY"
HEADER_VERSION = 0x0300
TOKEN_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
VALIDATION_LEVELS = {"software-only", "ram-bootstrap-passed", "hil-passed"}
EXPECTED_ENTRIES = [
    ("mbr-placeholder", "12345678", "1234567890___MBR"),
    ("sys-config", "COMMON", "SYS_CONFIG_BIN00"),
    ("board-config", "COMMON", "BOARD_CONFIG_BIN"),
    ("dtb-config", "COMMON", "DTB_CONFIG000000"),
    ("fes1", "FES", "FES_1-0000000000"),
    ("uboot", "12345678", "UBOOT_0000000000"),
]


class LoaderError(RuntimeError):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_name(data: dict) -> str:
    return "-".join(
        (data["chip"], data["memory_type"], data["storage_type"], data["product_name"], "loader.bin")
    )


def load_manifest(path: Path) -> tuple[dict, list[dict]]:
    path = path.resolve()
    with path.open("r", encoding="utf-8") as stream:
        data = json.load(stream)
    entries = data.get("entries", [])
    errors: list[str] = []
    for key in ("chip", "memory_type", "storage_type", "product_name"):
        value = data.get(key, "")
        if not isinstance(value, str) or not TOKEN_RE.fullmatch(value):
            errors.append(f"{key} must be lowercase kebab-case, got {value!r}")
    if data.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if data.get("purpose") != "ram-fes-bootstrap":
        errors.append("purpose must be ram-fes-bootstrap")
    if data.get("flash_payload") is not False:
        errors.append("flash_payload must be false")
    if data.get("container_format") != "imagewty-v3":
        errors.append("container_format must be imagewty-v3")
    if data.get("hardware_validation") not in VALIDATION_LEVELS:
        errors.append(f"hardware_validation must be one of {sorted(VALIDATION_LEVELS)}")
    if not SHA256_RE.fullmatch(str(data.get("expected_output_sha256", ""))):
        errors.append("expected_output_sha256 must be a lowercase SHA-256")
    expected_name = canonical_name(data) if not errors else None
    if expected_name and data.get("output_name") != expected_name:
        errors.append(f"output_name must be {expected_name}")
    if len(entries) != len(EXPECTED_ENTRIES):
        errors.append(f"exactly {len(EXPECTED_ENTRIES)} entries are required")
    for index, expected in enumerate(EXPECTED_ENTRIES):
        if index >= len(entries):
            break
        entry = entries[index]
        actual = (entry.get("role"), entry.get("maintype"), entry.get("subtype"))
        if actual != expected:
            errors.append(f"entry {index + 1} must be {expected}, got {actual}")
        if not SHA256_RE.fullmatch(str(entry.get("sha256", ""))):
            errors.append(f"entry {index + 1} has invalid sha256")
        if not SHA256_RE.fullmatch(str(entry.get("packed_sha256", ""))):
            errors.append(f"entry {index + 1} has invalid packed_sha256")
        if entry.get("flash_to_media") is not False:
            errors.append(f"entry {index + 1} flash_to_media must be false")
        source = (path.parent / str(entry.get("path", ""))).resolve()
        try:
            source.relative_to(path.parent.resolve())
        except ValueError:
            errors.append(f"entry {index + 1} path escapes its profile directory")
            continue
        if not source.is_file():
            errors.append(f"entry {index + 1} source is missing: {source}")
        elif sha256_file(source) != entry.get("sha256"):
            errors.append(f"entry {index + 1} SHA-256 mismatch: {source}")
        entry["_source"] = source
    if errors:
        raise LoaderError("manifest validation failed:\n- " + "\n- ".join(errors))
    return data, entries


def vendor_dragon() -> Path:
    override = os.environ.get("ALLWINNER_DRAGON")
    path = Path(override) if override else ROOT / "tools/vendor/allwinner/eDragonEx/dragon"
    if not path.is_file():
        raise LoaderError(f"vendor dragon is missing: {path}")
    return path.resolve()


def image_cfg(entries: list[dict]) -> str:
    lines = [
        "[MAIN_TYPE]",
        'ITEM_COMMON = "COMMON  "',
        'ITEM_FES = "FES     "',
        "",
        "[FILELIST]",
    ]
    for entry in entries:
        if entry["maintype"] == "COMMON":
            main = "ITEM_COMMON"
        elif entry["maintype"] == "FES":
            main = "ITEM_FES"
        else:
            main = f'"{entry["maintype"].ljust(8)}"'
        lines.append(
            f'{{filename = "{Path(entry["path"]).name}", maintype = {main}, '
            f'subtype = "{entry["subtype"]}",}},'
        )
    lines += ["", "[IMAGE_CFG]", "filelist = FILELIST", "imagename = loader.img", ""]
    return "\n".join(lines)


def parse_image(path: Path, include_hashes: bool = True) -> dict:
    raw = path.read_bytes()
    if len(raw) < HEADER_SIZE or raw[:8] != MAGIC:
        raise LoaderError(f"not an unencrypted IMAGEWTY image: {path}")
    version, header_size, ram_base, image_version, image_size, image_header_size = struct.unpack_from(
        "<6I", raw, 8
    )
    if version != HEADER_VERSION:
        raise LoaderError(f"unsupported IMAGEWTY header version 0x{version:08x}")
    if image_size != len(raw):
        raise LoaderError(f"header image_size={image_size}, actual={len(raw)}")
    num_files = struct.unpack_from("<I", raw, 60)[0]
    if len(raw) < HEADER_SIZE * (num_files + 1):
        raise LoaderError("truncated IMAGEWTY file-header table")
    files = []
    for index in range(num_files):
        offset = HEADER_SIZE * (index + 1)
        filename_len, total_header_size = struct.unpack_from("<II", raw, offset)
        maintype = raw[offset + 8 : offset + 16].decode("ascii").rstrip("\0 ")
        subtype = raw[offset + 16 : offset + 32].decode("ascii").rstrip("\0 ")
        filename = raw[offset + 36 : offset + 292].split(b"\0", 1)[0].decode("utf-8")
        stored_length, _, original_length, _, data_offset = struct.unpack_from("<5I", raw, offset + 292)
        end = data_offset + original_length
        if total_header_size != HEADER_SIZE or filename_len != 256:
            raise LoaderError(f"invalid file header at index {index}")
        if end > len(raw) or data_offset % HEADER_SIZE:
            raise LoaderError(f"invalid payload range at index {index}")
        item = {
            "index": index + 1,
            "filename": filename,
            "maintype": maintype,
            "subtype": subtype,
            "offset": data_offset,
            "stored_length": stored_length,
            "original_length": original_length,
        }
        if include_hashes:
            item["sha256"] = hashlib.sha256(raw[data_offset:end]).hexdigest()
        files.append(item)
    result = {
        "format": "IMAGEWTY",
        "header_version": f"0x{version:04x}",
        "header_size": header_size,
        "ram_base": f"0x{ram_base:08x}",
        "image_version": image_version,
        "image_header_size": image_header_size,
        "image_size": image_size,
        "num_files": num_files,
        "files": files,
    }
    if include_hashes:
        result["sha256"] = hashlib.sha256(raw).hexdigest()
    return result


def verify_image(manifest_path: Path, image_path: Path) -> dict:
    data, entries = load_manifest(manifest_path)
    expected_name = canonical_name(data)
    if image_path.name != expected_name:
        raise LoaderError(f"image name must be {expected_name}, got {image_path.name}")
    info = parse_image(image_path)
    if info["sha256"] != data["expected_output_sha256"]:
        raise LoaderError(
            f'image SHA-256 {info["sha256"]} does not match expected_output_sha256 '
            f'{data["expected_output_sha256"]}'
        )
    if info["num_files"] != len(entries):
        raise LoaderError("image entry count does not match manifest")
    errors = []
    for actual, expected in zip(info["files"], entries):
        for key in ("maintype", "subtype"):
            if actual[key] != expected[key]:
                errors.append(f'{expected["role"]} {key}: {actual[key]!r} != {expected[key]!r}')
        if actual["sha256"] != expected["packed_sha256"]:
            errors.append(
                f'{expected["role"]} packed_sha256: {actual["sha256"]!r} '
                f'!= {expected["packed_sha256"]!r}'
            )
        if actual["original_length"] != expected["_source"].stat().st_size:
            errors.append(f'{expected["role"]} length mismatch')
    if errors:
        raise LoaderError("image verification failed:\n- " + "\n- ".join(errors))
    return info


def public_manifest(data: dict, entries: list[dict], image_path: Path, info: dict) -> dict:
    public_entries = []
    for entry, parsed in zip(entries, info["files"]):
        public_entries.append({
            "role": entry["role"],
            "maintype": entry["maintype"],
            "subtype": entry["subtype"],
            "source_filename": Path(entry["path"]).name,
            "source_sha256": entry["sha256"],
            "packed_sha256": entry["packed_sha256"],
            "length": parsed["original_length"],
            "offset": parsed["offset"],
            "flash_to_media": False,
        })
    keys = (
        "schema_version", "chip", "soc_family", "fel_device_id", "fes_device_id",
        "memory_type", "memory_size_mib", "dram_clock_mhz", "storage_type",
        "storage_capacity_mib", "product_name", "purpose", "flash_payload",
        "container_format", "hardware_validation", "source_project", "source_revision",
        "expected_output_sha256",
    )
    result = {key: data[key] for key in keys if key in data}
    result.update({
        "filename": image_path.name,
        "size": image_path.stat().st_size,
        "sha256": info["sha256"],
        "entries": public_entries,
    })
    return result


def build(manifest_path: Path, output_dir: Path, check_reproducible: bool = False) -> tuple[Path, Path]:
    data, entries = load_manifest(manifest_path)
    output_dir.mkdir(parents=True, exist_ok=True)
    image_path = output_dir / canonical_name(data)

    def build_once(destination: Path) -> None:
        with tempfile.TemporaryDirectory(prefix="allwinner-loader-") as temporary:
            work = Path(temporary)
            for entry in entries:
                shutil.copyfile(entry["_source"], work / Path(entry["path"]).name)
            (work / "image.cfg").write_text(image_cfg(entries), encoding="utf-8")
            (work / "sys_partition.fex").write_text("[mbr]\n[partition_start]\n", encoding="ascii")
            completed = subprocess.run(
                [str(vendor_dragon()), "image.cfg", "sys_partition.fex"],
                cwd=work,
                text=True,
                capture_output=True,
            )
            if completed.returncode or not (work / "loader.img").is_file():
                raise LoaderError(
                    f"vendor dragon failed ({completed.returncode})\n{completed.stdout}\n{completed.stderr}"
                )
            shutil.copyfile(work / "loader.img", destination)

    build_once(image_path)
    info = verify_image(manifest_path, image_path)
    if check_reproducible:
        with tempfile.TemporaryDirectory(prefix="allwinner-loader-repro-") as temporary:
            second = Path(temporary) / image_path.name
            build_once(second)
            if sha256_file(second) != sha256_file(image_path):
                raise LoaderError(f"non-reproducible output for {manifest_path}")
    metadata_path = output_dir / image_path.name.replace(".bin", ".manifest.json")
    metadata_path.write_text(
        json.dumps(public_manifest(data, entries, image_path, info), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return image_path, metadata_path


def manifests() -> list[Path]:
    return sorted((ROOT / "profiles").glob("*/loader.json"))


def write_sums(output_dir: Path) -> Path:
    files = sorted(path for path in output_dir.iterdir() if path.is_file() and path.name != "SHA256SUMS")
    sums = output_dir / "SHA256SUMS"
    sums.write_text("".join(f"{sha256_file(path)}  {path.name}\n" for path in files), encoding="ascii")
    return sums


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build")
    build_parser.add_argument("--manifest", required=True, type=Path)
    build_parser.add_argument("--output-dir", type=Path, default=Path("dist"))
    build_parser.add_argument("--check-reproducible", action="store_true")
    verify_parser = commands.add_parser("verify")
    verify_parser.add_argument("--manifest", required=True, type=Path)
    verify_parser.add_argument("--image", required=True, type=Path)
    inspect_parser = commands.add_parser("inspect")
    inspect_parser.add_argument("image", type=Path)
    commands.add_parser("validate-all")
    all_parser = commands.add_parser("build-all")
    all_parser.add_argument("--output-dir", type=Path, default=Path("dist"))
    all_parser.add_argument("--clean", action="store_true")
    all_parser.add_argument("--check-reproducible", action="store_true")
    args = parser.parse_args()
    try:
        if args.command == "build":
            image, metadata = build(args.manifest, args.output_dir, args.check_reproducible)
            write_sums(args.output_dir)
            print(f"built {image} sha256={sha256_file(image)}")
            print(f"manifest {metadata}")
        elif args.command == "verify":
            print(json.dumps(verify_image(args.manifest, args.image), indent=2))
        elif args.command == "inspect":
            print(json.dumps(parse_image(args.image), indent=2))
        elif args.command == "validate-all":
            found = manifests()
            if not found:
                raise LoaderError("no profiles found")
            for path in found:
                load_manifest(path)
                print(f"valid {path.relative_to(ROOT)}")
        elif args.command == "build-all":
            if args.clean and args.output_dir.exists():
                shutil.rmtree(args.output_dir)
            found = manifests()
            if not found:
                raise LoaderError("no profiles found")
            for path in found:
                image, _ = build(path, args.output_dir, args.check_reproducible)
                print(f"built {image.name} sha256={sha256_file(image)}")
            print(f"checksums {write_sums(args.output_dir)}")
    except (LoaderError, OSError, KeyError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
