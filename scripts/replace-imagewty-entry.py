#!/usr/bin/env python3
"""Replace one uncompressed, same-size entry in an IMAGEWTY v3 container."""

import argparse
import hashlib
import shutil
import struct
from pathlib import Path


HEADER_SIZE = 1024
MAGIC = b"IMAGEWTY"
VERSION_V3 = 0x0300


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--container", required=True, type=Path)
    parser.add_argument("--entry", required=True)
    parser.add_argument("--replacement", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if args.output.resolve() == args.container.resolve():
        raise SystemExit("OUTPUT_MUST_NOT_OVERWRITE_INPUT")

    with args.container.open("rb") as stream:
        image_header = stream.read(HEADER_SIZE)
        if image_header[:8] != MAGIC:
            raise SystemExit("IMAGEWTY_MAGIC_MISMATCH")
        if u32(image_header, 8) != VERSION_V3:
            raise SystemExit("IMAGEWTY_V3_REQUIRED")
        file_count = u32(image_header, 60)
        if file_count == 0 or file_count > 4096:
            raise SystemExit("IMAGEWTY_FILE_COUNT_INVALID")

        match = None
        for index in range(file_count):
            stream.seek(HEADER_SIZE * (index + 1))
            header = stream.read(HEADER_SIZE)
            if len(header) != HEADER_SIZE:
                raise SystemExit("IMAGEWTY_FILE_HEADER_TRUNCATED")
            filename = header[36:292].split(b"\0", 1)[0].decode("ascii")
            stored_length = u32(header, 292)
            original_length = u32(header, 300)
            offset = u32(header, 308)
            if filename == args.entry:
                if match is not None:
                    raise SystemExit("IMAGEWTY_ENTRY_DUPLICATE")
                match = (offset, stored_length, original_length)

    if match is None:
        raise SystemExit("IMAGEWTY_ENTRY_NOT_FOUND")
    offset, stored_length, original_length = match
    replacement_size = args.replacement.stat().st_size
    if stored_length != original_length:
        raise SystemExit("IMAGEWTY_COMPRESSED_ENTRY_REJECTED")
    if replacement_size != original_length:
        raise SystemExit(
            f"IMAGEWTY_REPLACEMENT_SIZE_MISMATCH:expected={original_length}:actual={replacement_size}"
        )
    if offset + stored_length > args.container.stat().st_size:
        raise SystemExit("IMAGEWTY_ENTRY_RANGE_INVALID")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(args.container, args.output)
    replacement = args.replacement.read_bytes()
    with args.output.open("r+b") as stream:
        stream.seek(offset)
        stream.write(replacement)

    print(f"IMAGEWTY_ENTRY_REPLACED:{args.entry}")
    print(f"OFFSET:{offset}")
    print(f"LENGTH:{original_length}")
    print(f"INPUT_SHA256:{sha256(args.container)}")
    print(f"REPLACEMENT_SHA256:{sha256(args.replacement)}")
    print(f"OUTPUT_SHA256:{sha256(args.output)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
