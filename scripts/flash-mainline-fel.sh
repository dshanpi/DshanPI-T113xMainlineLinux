#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
ARTIFACTS="${1:-${ROOT}/out/t113s3pro-mainline-fel}"
LOCATION="${2:-auto}"
OPENIXCLI_BIN="${OPENIXCLI_BIN:-openixcli}"

command -v "${OPENIXCLI_BIN}" >/dev/null 2>&1 || {
	echo "OpenixCLI executable not found: ${OPENIXCLI_BIN}" >&2
	exit 2
}

if [ "${LOCATION}" = auto ]; then
	LOCATION="$(python3 "${ROOT}/scripts/select-openix-device.py" --openixcli "${OPENIXCLI_BIN}")"
	echo "Selected the only Allwinner USB device: ${LOCATION}" >&2
fi

rest="${LOCATION#libusb:}"
bus="${rest%%:*}"
port="${rest#*:}"
if [ "${rest}" = "${LOCATION}" ] || [ -z "${bus}" ] || [ -z "${port}" ]; then
	echo "invalid libusb location: ${LOCATION}" >&2
	exit 2
fi

PLAN="${ARTIFACTS}/openix-mainline-plan.json"
python3 "${ROOT}/scripts/make-openix-plan.py" "${ARTIFACTS}" "${PLAN}"
exec "${OPENIXCLI_BIN}" --output jsonl boot-mainline \
	--plan "${PLAN}" --device-location "${LOCATION}" \
	--bus "${bus}" --port "${port}"
