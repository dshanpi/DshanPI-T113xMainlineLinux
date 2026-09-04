#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
BUNDLE="${1:-${ROOT}/out/t113s3pro-mainline-fes}"
OPENIXCLI_BIN="${OPENIXCLI_BIN:-${ROOT}/tools/OpenixCLI/target/release/openixcli}"
DEVICE_LOCATION="${DEVICE_LOCATION:-}"
BUS="${BUS:-}"
PORT="${PORT:-}"

if [ -z "${DEVICE_LOCATION}" ] || [ -z "${BUS}" ] || [ -z "${PORT}" ]; then
	echo "Set DEVICE_LOCATION=libusb:BUS:PORT, BUS and PORT after entering FEL." >&2
	exit 2
fi
if [ "${DEVICE_LOCATION}" != "libusb:${BUS}:${PORT}" ]; then
	echo "USB binding mismatch: ${DEVICE_LOCATION} vs bus=${BUS},port=${PORT}" >&2
	exit 2
fi

exec "${OPENIXCLI_BIN}" --output jsonl flash-nand-components \
	--manifest "${BUNDLE}/manifest.json" \
	--device-location "${DEVICE_LOCATION}" --bus "${BUS}" --port "${PORT}" \
	--mode full_erase --verify --post-action none
