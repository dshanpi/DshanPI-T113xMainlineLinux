#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
ARTIFACTS="${1:-${ROOT}/out/t113s3pro-mainline-fel}"
LOCATION="${2:-auto}"
SERIAL_DEVICE="${3:-/dev/ttyACM0}"
TIMEOUT="${4:-300}"
LOG_DIR="${ROOT}/out/flash-logs"
STAMP="$(date +%Y%m%d-%H%M%S)"
UART_LOG="${LOG_DIR}/uart-${STAMP}.log"
OPENIXCLI_BIN="${OPENIXCLI_BIN:-${ROOT}/tools/OpenixCLI/target/release/openixcli}"

command -v "${OPENIXCLI_BIN}" >/dev/null 2>&1 || {
	echo "OpenixCLI executable not found: ${OPENIXCLI_BIN}" >&2
	exit 2
}
[ -c "${SERIAL_DEVICE}" ] || {
	echo "serial device is unavailable: ${SERIAL_DEVICE}" >&2
	exit 2
}
mkdir -p "${LOG_DIR}"

python3 "${ROOT}/scripts/serial-installer-monitor.py" \
	--device "${SERIAL_DEVICE}" --timeout "${TIMEOUT}" --log "${UART_LOG}" &
monitor_pid=$!
trap 'kill "${monitor_pid}" 2>/dev/null || true' EXIT HUP INT TERM

if ! OPENIXCLI_BIN="${OPENIXCLI_BIN}" \
	"${ROOT}/scripts/flash-mainline-fel.sh" "${ARTIFACTS}" "${LOCATION}"; then
	kill "${monitor_pid}" 2>/dev/null || true
	wait "${monitor_pid}" 2>/dev/null || true
	echo "FEL RAM handoff failed; NAND installer result is unknown" >&2
	exit 1
fi

echo "FEL RAM handoff complete; waiting for independent UART installer evidence"
if ! wait "${monitor_pid}"; then
	echo "UART acceptance failed; log retained at ${UART_LOG}" >&2
	exit 1
fi
trap - EXIT HUP INT TERM
echo "Warm reboot acceptance passed; UART log: ${UART_LOG}"
echo "A controlled power-off cold boot is still required for release qualification."
