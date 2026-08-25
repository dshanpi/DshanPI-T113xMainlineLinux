#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
TREE="${ROOT}/buildroot/buildroot-mainline"
OUTPUT="${ROOT}/out/mainline/t113s3pro"

"${ROOT}/scripts/bootstrap-buildroot.sh"
make -C "${TREE}" O="${OUTPUT}" dshanpi_t113s3pro_nand_defconfig
make -C "${TREE}" O="${OUTPUT}" "$@"

echo "Mainline Buildroot output: ${OUTPUT}"
