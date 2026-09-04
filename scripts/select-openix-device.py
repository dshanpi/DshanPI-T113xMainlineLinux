#!/usr/bin/env python3
"""Select exactly one Allwinner USB endpoint from OpenixCLI JSONL scan output."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys


def parse_devices(output: str) -> list[dict[str, object]]:
    devices: list[dict[str, object]] = []
    for number, line in enumerate(output.splitlines(), 1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"invalid OpenixCLI JSONL on line {number}: {error}") from error
        if event.get("event") == "device":
            devices.append(event)
    return devices


def select_location(devices: list[dict[str, object]]) -> str:
    candidates = [device for device in devices if device.get("vid") == 0x1F3A]
    if not candidates:
        raise ValueError("no Allwinner FEL/FES USB device found; enter FEL and retry")
    if len(candidates) != 1:
        locations = ", ".join(str(item.get("location", "unknown")) for item in candidates)
        raise ValueError(f"multiple Allwinner devices found ({locations}); specify one explicitly")
    location = candidates[0].get("location")
    if not isinstance(location, str) or not location.startswith("libusb:"):
        raise ValueError("OpenixCLI returned an invalid physical USB location")
    return location


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--openixcli", default="openixcli")
    args = parser.parse_args()
    result = subprocess.run(
        [args.openixcli, "--output", "jsonl", "scan"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown scan failure"
        print(f"OpenixCLI scan failed: {detail}", file=sys.stderr)
        return 2
    try:
        print(select_location(parse_devices(result.stdout)))
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
