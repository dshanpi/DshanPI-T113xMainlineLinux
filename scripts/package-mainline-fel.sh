#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
TREE="${ROOT}/buildroot/buildroot-mainline"
OUTPUT="${ROOT}/out/mainline/t113s3pro"
ARTIFACTS="${ROOT}/out/t113s3pro-mainline-fel"

if [ ! -d "${TREE}" ]; then
	echo "Buildroot is missing; run scripts/build.sh first" >&2
	exit 1
fi

HOST_DIR="${OUTPUT}/host" \
	"${TREE}/board/dshanpi/t113s3pro/make-mainline-fel-images.sh" \
	"${OUTPUT}/images" "${OUTPUT}/target" \
	"${OUTPUT}/build/uboot-2026.07" "${ARTIFACTS}"

echo "Mainline FEL installer artifacts: ${ARTIFACTS}"
