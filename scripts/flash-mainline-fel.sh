#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
ARTIFACTS="${1:-${ROOT}/out/t113s3pro-mainline-fel}"
LOCATION="${2:-}"

if [ -z "${LOCATION}" ]; then
	echo "usage: $0 [ARTIFACT_DIR] libusb:BUS:PORT" >&2
	exit 2
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
exec openixcli --output jsonl boot-mainline \
	--plan "${PLAN}" --device-location "${LOCATION}" \
	--bus "${bus}" --port "${port}"
