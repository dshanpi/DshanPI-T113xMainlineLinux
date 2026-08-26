#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OPENIXCLI_DIR="${OPENIXCLI_DIR:-$(CDPATH= cd -- "${ROOT}/.." && pwd)/OpenixCLI}"

for tool in git make gcc python3 cpio zstd rsync cargo; do
	command -v "${tool}" >/dev/null 2>&1 || {
		echo "missing required host tool: ${tool}" >&2
		exit 2
	}
done

if [ ! -f "${OPENIXCLI_DIR}/Cargo.toml" ]; then
	echo "OpenixCLI not found at ${OPENIXCLI_DIR}" >&2
	echo "clone it beside this repository or set OPENIXCLI_DIR" >&2
	exit 2
fi

echo "[1/4] Testing pinned OpenixCLI sources"
cargo test --locked --manifest-path "${OPENIXCLI_DIR}/Cargo.toml"
echo "[2/4] Building OpenixCLI release binary"
cargo build --release --locked --manifest-path "${OPENIXCLI_DIR}/Cargo.toml"
echo "[3/4] Building and packaging the T113S3 Pro mainline system"
"${ROOT}/scripts/build-and-package.sh"
echo "[4/4] Running repository and artifact acceptance gates"
"${ROOT}/scripts/validate-local.py"

echo
echo "Build complete"
echo "OpenixCLI: ${OPENIXCLI_DIR}/target/release/openixcli"
echo "Artifacts: ${ROOT}/out/t113s3pro-mainline-fel"
echo "Next: OPENIXCLI_BIN=${OPENIXCLI_DIR}/target/release/openixcli ${ROOT}/scripts/flash-and-monitor.sh"
