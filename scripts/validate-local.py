#!/usr/bin/env python3
"""Run the non-hardware acceptance gates for the T113S3 Pro port."""

from __future__ import annotations

import ast
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "out/mainline/t113s3pro"
ARTIFACTS = ROOT / "out/t113s3pro-mainline-fel"
UBOOT = OUTPUT / "build/uboot-2026.07"
BUILDROOT = ROOT / "buildroot/buildroot-mainline"
passed = 0
failed = 0


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def command(*args: str, cwd: Path = ROOT) -> str:
    result = subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.stdout


def check(identifier: str, description: str, action) -> None:
    global passed, failed
    try:
        result = action()
        if result is False:
            raise AssertionError("predicate returned false")
    except Exception as error:  # one failure must not hide later gate results
        failed += 1
        print(f"[FAIL] {identifier} {description}: {error}")
    else:
        passed += 1
        print(f"[PASS] {identifier} {description}")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def source_lock() -> dict[str, str]:
    entries = {}
    for line in (ROOT / "manifests/sources.lock").read_text().splitlines():
        if line and not line.startswith("#"):
            key, value = line.split("=", 1)
            entries[key] = value
    return entries


def verify_checksum_file(manifest: Path, directory: Path) -> None:
    for line in manifest.read_text().splitlines():
        expected, name = line.split(None, 1)
        path = directory / name.strip()
        require(path.is_file(), f"missing {path}")
        require(digest(path) == expected, f"SHA-256 mismatch: {path.name}")


def verify_evidence_manifest(manifest: Path) -> None:
    entries: dict[str, str] = {}
    for number, line in enumerate(manifest.read_text().splitlines(), 1):
        fields = line.split(None, 1)
        require(len(fields) == 2, f"malformed manifest line {number}")
        checksum, name = fields
        require(re.fullmatch(r"[0-9a-f]{64}", checksum) is not None,
                f"invalid SHA-256 on line {number}")
        require(name not in entries, f"duplicate artifact: {name}")
        entries[name] = checksum
    required = {
        "boot.itb",
        "fel-installer.itb",
        "fel-sunxi-spl.bin",
        "fel-u-boot.bin",
        "spl-redundant.bin",
        "sys.ubi",
        "uboot-redundant.bin",
    }
    require(required.issubset(entries), "candidate evidence manifest is incomplete")


def verify_fes_schema_layout() -> None:
    schema = json.loads((ROOT / "manifests/fes-nand-components.schema.json").read_text())
    physical = schema["properties"]["layout"]["properties"]["partitions"]["const"]
    require(
        physical == [
            {"name": "spl", "offset": 0, "size": 1048576},
            {"name": "uboot", "offset": 1048576, "size": 3145728},
            {"name": "secure-storage", "offset": 4194304, "size": 1048576},
            {"name": "sys", "offset": 5242880, "size": 263192576},
        ],
        "physical NAND layout is not exactly pinned",
    )
    actual = schema["properties"]["layout"]["properties"]["fesPartitions"]["const"]
    expected = [
        {"name": "boot", "addressSectors": 504, "sizeSectors": 16632},
        {"name": "rootfs", "addressSectors": 17136, "sizeSectors": 81900},
        {"name": "UDISK", "addressSectors": 99036, "sizeSectors": 0},
    ]
    require(actual == expected, "FES MBR layout is not exactly pinned")
    content_types = schema["$defs"]["component"]["properties"]["contentType"]["enum"]
    require(
        content_types == ["egon-boot0", "legacy-uboot", "fit", "ubifs"],
        "FES component content types are not pinned",
    )


def verify_payload() -> None:
    with tempfile.TemporaryDirectory(prefix="t113-payload-verify-") as temp:
        target = Path(temp)
        with tarfile.open(ARTIFACTS / "fel-payload.tar.gz", "r:gz") as archive:
            names = set(archive.getnames())
            expected = {
                "spl-redundant.bin",
                "uboot-redundant.bin",
                "sys.ubi",
                "BOOT_VOLUME",
                "SHA256SUMS",
            }
            require(names == expected, f"unexpected payload members: {sorted(names)}")
            archive.extractall(target, filter="data")
        verify_checksum_file(target / "SHA256SUMS", target)


def verify_redundant_spl() -> None:
    spl = (ARTIFACTS / "fel-sunxi-spl.bin").read_bytes()
    redundant = (ARTIFACTS / "spl-redundant.bin").read_bytes()
    require(len(redundant) == 1024 * 1024, "SPL partition image is not 1 MiB")
    for offset in range(0, 1024 * 1024, 128 * 1024):
        require(redundant[offset : offset + len(spl)] == spl, f"bad SPL copy at {offset:#x}")


def verify_egon() -> None:
    spl = (ARTIFACTS / "fel-sunxi-spl.bin").read_bytes()
    require(len(spl) == 24576, f"unexpected SPL length {len(spl)}")
    require(spl[4:12] == b"eGON.BT0", "missing eGON.BT0 magic")
    declared = struct.unpack_from("<I", spl, 16)[0]
    require(declared == len(spl), "eGON length does not match file")
    words = bytearray(spl)
    expected = struct.unpack_from("<I", words, 12)[0]
    struct.pack_into("<I", words, 12, 0x5F0A6C39)
    actual = sum(struct.unpack_from("<I", words, offset)[0] for offset in range(0, len(words), 4)) & 0xFFFFFFFF
    require(actual == expected, "eGON additive checksum mismatch")


def verify_plan() -> None:
    with tempfile.TemporaryDirectory(prefix="t113-plan-verify-") as temp:
        plan_path = Path(temp) / "plan.json"
        command(
            sys.executable,
            str(ROOT / "scripts/make-openix-plan.py"),
            str(ARTIFACTS),
            str(plan_path),
        )
        plan = json.loads(plan_path.read_text())
        entries = plan["artifacts"]
        require(plan["entryAddress"] == 0x42E00000, "wrong entry address")
        require(4 <= len(entries) <= 8, "artifact count exceeds OpenixCLI bound")
        expected = [
            ("spl", 0x00020000),
            ("bootloader", 0x42E00000),
            ("kernel", 0x44000000),
        ]
        require([(item["role"], item["loadAddress"]) for item in entries[:3]] == expected,
                "fixed plan roles or addresses changed")
        for item in entries:
            path = Path(item["filePath"])
            require(path.is_absolute(), "plan path is not absolute")
            require(digest(path) == item["sha256"], f"plan hash mismatch: {path.name}")
        ranges = sorted((item["loadAddress"], item["loadAddress"] + Path(item["filePath"]).stat().st_size) for item in entries)
        require(all(first[1] <= second[0] for first, second in zip(ranges, ranges[1:])),
                "plan load ranges overlap")


def verify_log_json() -> None:
    rows = (ROOT / "logs/t113-task-history-20260824-25.jsonl").read_text().splitlines()
    require(len(rows) == 232, f"expected 232 task rows, got {len(rows)}")
    for number, row in enumerate(rows, 1):
        try:
            json.loads(row)
        except json.JSONDecodeError as error:
            raise AssertionError(f"invalid JSON on row {number}: {error}") from error


def verify_revalidation_json() -> None:
    rows = (ROOT / "logs/hardware-revalidation-20260825.jsonl").read_text().splitlines()
    require(len(rows) == 5, f"expected 5 revalidation rows, got {len(rows)}")
    parsed = [json.loads(row) for row in rows]
    require(parsed[0]["taskId"] == "mainline-1787708569776828829", "operator failure missing")
    require(parsed[1]["taskId"] == "mainline-1787708850567538011", "clean-build failure missing")
    require(parsed[2]["taskId"] == "mainline-1787709324680503509", "verified success missing")
    require(parsed[2]["status"] == "success" and parsed[2]["progressPct"] == 100.0,
            "verified task is not terminal success")
    require([row["action"] for row in parsed[3:]] == ["power_off", "power_on"],
            "cold power cycle is incomplete")


def verify_fes_host_log() -> None:
    path = ROOT / "logs/fes-host-validation-20260826.jsonl"
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    require(len(rows) == 5, "unexpected FES host evidence row count")
    require(rows[0]["status"] == "experimental-pending-cold-boot", "wrong FES status")
    require(rows[2]["bootstrapRolesVerified"] is True, "bootstrap roles not verified")
    require(rows[2]["embeddedComponentsVerified"] is True, "components not verified")
    require(rows[3]["eraseAttempted"] is False, "no-device gate attempted erase")
    require(rows[4]["passed"] == 75 and rows[4]["failed"] == 0, "wrong local gate result")
    require(rows[4]["currentFesLayoutHardwareStatus"] == "pending", "hardware status overstated")


def main() -> int:
    lock = source_lock()
    checks = [
        ("T001", "source lock pins Buildroot commit", lambda: require(lock.get("BUILDROOT_COMMIT") == "86102dd8279ac6c4c0244f3e490af98dc7460d5e", "wrong Buildroot commit")),
        ("T002", "source lock pins Linux 6.18.8", lambda: require(lock.get("LINUX_VERSION") == "6.18.8", "wrong Linux version")),
        ("T003", "source lock pins U-Boot 2026.07", lambda: require(lock.get("UBOOT_VERSION") == "2026.07", "wrong U-Boot version")),
        ("T004", "Buildroot checkout matches source lock", lambda: require(command("git", "rev-parse", "HEAD", cwd=BUILDROOT).strip() == lock["BUILDROOT_COMMIT"], "Buildroot HEAD mismatch")),
        ("T005", "Linux archive hash is pinned", lambda: require(re.fullmatch(r"[0-9a-f]{64}", lock.get("LINUX_ARCHIVE_SHA256", "")) is not None, "invalid Linux hash")),
        ("T006", "U-Boot archive hash is pinned", lambda: require(re.fullmatch(r"[0-9a-f]{64}", lock.get("UBOOT_ARCHIVE_SHA256", "")) is not None, "invalid U-Boot hash")),
    ]
    for item in checks:
        check(*item)

    required = [
        "board/dshanpi/t113s3pro/linux-dts/allwinner/sun8i-t113s-dshanpi-t113s3pro.dts",
        "board/dshanpi/t113s3pro/uboot.fragment",
        "configs/dshanpi_t113s3pro_nand_defconfig",
        "board/dshanpi/t113s3pro/installer-init",
        "manifests/fes-nand-components.schema.json",
        "manifests/retired-fes-artifacts.txt",
        "scripts/prepare-fes-bundle.py",
        "scripts/flash-fes-nand.sh",
        "scripts/package-fes-components.sh",
        "docs/fes-nand-provisioning.md",
        "logs/fes-host-validation-20260826.jsonl",
    ]
    for index, name in enumerate(required, 7):
        check(f"T{index:03d}", f"required source exists: {name}", lambda name=name: require((ROOT / name).is_file(), "missing source"))

    shell_files = sorted((ROOT / "scripts").glob("*.sh")) + sorted((ROOT / "board/dshanpi/t113s3pro").glob("*.sh")) + [ROOT / "board/dshanpi/t113s3pro/installer-init"]
    next_id = 7 + len(required)
    for path in shell_files:
        check(f"T{next_id:03d}", f"shell syntax: {path.relative_to(ROOT)}", lambda path=path: command("sh", "-n", str(path)))
        next_id += 1

    check(
        f"T{next_id:03d}",
        "FES NAND manifest schema is valid JSON",
        lambda: json.loads((ROOT / "manifests/fes-nand-components.schema.json").read_text()),
    )
    next_id += 1
    check(
        f"T{next_id:03d}",
        "FES NAND manifest pins exact boot/rootfs/UDISK sectors",
        verify_fes_schema_layout,
    )
    next_id += 1

    python_files = sorted((ROOT / "scripts").glob("*.py")) + [ROOT / "board/dshanpi/t113s3pro/verify-mainline-uboot.py"]
    for path in python_files:
        check(f"T{next_id:03d}", f"Python syntax: {path.relative_to(ROOT)}", lambda path=path: ast.parse(path.read_text()))
        next_id += 1

    check(
        f"T{next_id:03d}",
        "host workflow helper unit tests",
        lambda: command(sys.executable, str(ROOT / "scripts/test-workflow-helpers.py")),
    )
    next_id += 1

    verifier = ROOT / "board/dshanpi/t113s3pro/verify-mainline-uboot.py"
    check(f"T{next_id:03d}", "SPL and U-Boot proper pass same-build verifier", lambda: command(
        sys.executable, str(verifier), "--spl", str(UBOOT / "spl/sunxi-spl.bin"),
        "--uboot-bin", str(UBOOT / "u-boot.bin"), "--uboot-img", str(UBOOT / "u-boot.img"),
        "--uboot-elf", str(UBOOT / "u-boot"), "--version", "2026.07",
        "--board", "allwinner/sun8i-t113s-dshanpi-t113s3pro")); next_id += 1
    check(f"T{next_id:03d}", "packaged SPL passes independent eGON checks", verify_egon); next_id += 1
    check(f"T{next_id:03d}", "U-Boot environment is nowhere", lambda: require("CONFIG_ENV_IS_NOWHERE=y" in (UBOOT / ".config").read_text(), "ENV_IS_NOWHERE missing")); next_id += 1
    check(f"T{next_id:03d}", "U-Boot FAT/SPI environment backends are disabled", lambda: require(not re.search(r"CONFIG_ENV_IS_IN_(FAT|SPI_FLASH)=y", (UBOOT / ".config").read_text()), "forbidden environment backend")); next_id += 1
    check(f"T{next_id:03d}", "U-Boot console index selects UART3", lambda: require(
        "CONFIG_CONS_INDEX=4" in (UBOOT / ".config").read_text()
        and "@@ -0,0 +1,16 @@" in (ROOT / "board/dshanpi/t113s3pro/patches/uboot/0002-sunxi-add-dshanpi-t113s3pro-spinand-target.patch").read_text(),
        "UART3 console index missing from built config or permanent patch")); next_id += 1
    check(f"T{next_id:03d}", "FEL artifact checksums verify", lambda: verify_checksum_file(ARTIFACTS / "FEL_SHA256SUMS", ARTIFACTS)); next_id += 1
    check(f"T{next_id:03d}", "source-recovery candidate evidence manifest is complete", lambda: verify_evidence_manifest(ROOT / "manifests/source-recovery-candidate-20260825.sha256")); next_id += 1
    check(f"T{next_id:03d}", "payload archive membership and internal hashes verify", verify_payload); next_id += 1
    check(f"T{next_id:03d}", "eight redundant SPL eraseblock copies verify", verify_redundant_spl); next_id += 1
    check(f"T{next_id:03d}", "U-Boot redundant image matches FES 3 MiB Boot1 reservation", lambda: require((ARTIFACTS / "uboot-redundant.bin").stat().st_size == 3 * 1024 * 1024, "wrong U-Boot partition image size")); next_id += 1
    check(f"T{next_id:03d}", "installer FIT stays below 8 MiB", lambda: require((ARTIFACTS / "fel-installer.itb").stat().st_size <= 8 * 1024 * 1024, "installer FIT too large")); next_id += 1
    check(f"T{next_id:03d}", "payload archive stays below 24 MiB", lambda: require((ARTIFACTS / "fel-payload.tar.gz").stat().st_size <= 24 * 1024 * 1024, "payload too large")); next_id += 1
    check(f"T{next_id:03d}", "OpenixCLI plan roles, hashes and ranges verify", verify_plan); next_id += 1
    check(f"T{next_id:03d}", "boot FIT parses with mainline mkimage", lambda: command(str(UBOOT / "tools/mkimage"), "-l", str(ARTIFACTS / "boot.itb"))); next_id += 1
    check(f"T{next_id:03d}", "installer FIT parses with mainline mkimage", lambda: command(str(UBOOT / "tools/mkimage"), "-l", str(ARTIFACTS / "fel-installer.itb"))); next_id += 1
    retired = ["installer-android-v2.img", "installer-android-v2-auto.img", "openix-fel-mainline-auto.img", "fel-installer-chunked.itb"]
    check(f"T{next_id:03d}", "retired vendor/experimental artifacts are absent", lambda: require(not any((ARTIFACTS / name).exists() for name in retired), "retired artifact present")); next_id += 1
    dts = (ROOT / required[0]).read_text()
    pin_dtsi = (OUTPUT / "build/linux-6.18.8/arch/riscv/boot/dts/allwinner/sunxi-d1s-t113.dtsi").read_text()
    check(f"T{next_id:03d}", "UART3 PB6/PB7 console mapping is present", lambda: require(
        "pinctrl-0 = <&uart3_pb_pins>" in dts
        and 'uart3_pb_pins: uart3-pb-pins' in pin_dtsi
        and 'pins = "PB6", "PB7"' in pin_dtsi
        and 'function = "uart3"' in pin_dtsi,
        "UART3 pin mapping missing")); next_id += 1
    check(f"T{next_id:03d}", "SPI-NAND layout matches FES physical reservations", lambda: require(
        all(name in dts for name in ('label = "spl"', 'label = "uboot"', 'label = "secure-storage"', 'label = "sys"', 'reg = <0x00500000 0x0fb00000>'))
        and 'label = "boot"' not in dts,
        "FES-compatible partition layout incomplete")); next_id += 1
    check(f"T{next_id:03d}", "U-Boot loads FIT from the FES-created boot UBI volume", lambda: require(
        "ubi part sys; ubi read 0x44000000 boot" in (ROOT / "board/dshanpi/t113s3pro/uboot.fragment").read_text(),
        "UBI boot-volume command missing")); next_id += 1
    check(f"T{next_id:03d}", "Linux config enables SPI-NAND, UBI and UBIFS", lambda: require(all(item in (ROOT / "board/dshanpi/t113s3pro/linux.fragment").read_text() for item in ("CONFIG_MTD_SPI_NAND=y", "CONFIG_MTD_UBI=y", "CONFIG_UBIFS_FS=y")), "Linux NAND/UBI options missing")); next_id += 1
    check(f"T{next_id:03d}", "task history contains exactly 232 valid JSON rows", verify_log_json); next_id += 1
    check(f"T{next_id:03d}", "latest hardware revalidation JSONL is complete", verify_revalidation_json); next_id += 1
    check(f"T{next_id:03d}", "FES host validation evidence is bounded and pending hardware", verify_fes_host_log); next_id += 1
    cold_log = (ROOT / "logs/final-cold-boot.log").read_text(errors="replace")
    markers = ["Trying to boot from sunxi SPI", "U-Boot 2026.07", "ubi0: attached mtd4", "VFS: Mounted root (ubifs filesystem)", "t113s3pro-mainline login:"]
    for marker in markers:
        check(f"T{next_id:03d}", f"historical cold-boot marker: {marker}", lambda marker=marker: require(marker in cold_log, "marker absent")); next_id += 1
    task_log = (ROOT / "logs/t113-task-history-20260824-25.jsonl").read_text()
    check(f"T{next_id:03d}", "history retains successful installer task ID", lambda: require("mainline-1787655837814079629" in task_log, "success task absent")); next_id += 1
    latest_cold_log = (ROOT / "logs/hardware-revalidation-cold-boot-20260825.log").read_text(errors="replace")
    check(f"T{next_id:03d}", "latest power-cycle reaches UBIFS login", lambda: require(
        "VFS: Mounted root (ubifs filesystem)" in latest_cold_log
        and "t113s3pro-mainline login:" in latest_cold_log,
        "latest cold-boot markers absent")); next_id += 1
    check(f"T{next_id:03d}", "published logs contain no local workspace path", lambda: require("/home/" not in task_log + cold_log, "unsanitized workspace path")); next_id += 1
    check(f"T{next_id:03d}", "hardware and clean-build manifests remain distinct", lambda: require((ROOT / "manifests/verified-hardware-artifacts.sha256").read_text() != (ROOT / "manifests/clean-build-20260825.sha256").read_text(), "manifests unexpectedly identical")); next_id += 1
    check(f"T{next_id:03d}", "recovery candidate and failed manifests remain distinct", lambda: require((ROOT / "manifests/source-recovery-candidate-20260825.sha256").read_text() != (ROOT / "manifests/clean-build-20260825.sha256").read_text(), "candidate reused failed artifacts")); next_id += 1
    check(f"T{next_id:03d}", "hardware-verified source rebuild manifest is complete", lambda: verify_evidence_manifest(ROOT / "manifests/hardware-verified-source-rebuild-20260825.sha256")); next_id += 1
    rebuild_rows = (ROOT / "logs/source-rebuild-hardware-validation-20260825.jsonl").read_text().splitlines()
    check(f"T{next_id:03d}", "source rebuild hardware evidence has installer and two cold boots", lambda: require(
        len(rebuild_rows) == 9
        and all(json.loads(row) for row in rebuild_rows)
        and '"taskId":"mainline-1787715829104265529"' in rebuild_rows[1]
        and sum('"cold-boot-pass"' in row for row in rebuild_rows) == 2,
        "source rebuild hardware evidence incomplete")); next_id += 1
    check(f"T{next_id:03d}", "no Python bytecode is tracked", lambda: require(not command("git", "ls-files").strip().endswith(".pyc") and not any(line.endswith(".pyc") for line in command("git", "ls-files").splitlines()), "tracked bytecode")); next_id += 1

    print(
        f"SUMMARY pass={passed} fail={failed} "
        "historical_hardware_bundle=verified current_fes_layout=pending-hardware"
    )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
