#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
LOCK="${ROOT}/manifests/sources.lock"
"${ROOT}/scripts/source_lock.py" "${LOCK}"
# The lock is restricted to plain KEY=VALUE assignments by source_lock.py.
# shellcheck disable=SC1090
. "${LOCK}"

OPENIXCLI_MANAGED=0
if [ -n "${OPENIXCLI_DIR:-}" ]; then
	OPENIXCLI_DIR="$(CDPATH= cd -- "${OPENIXCLI_DIR}" && pwd)"
elif [ -f "${ROOT}/../OpenixCLI/Cargo.toml" ]; then
	OPENIXCLI_DIR="$(CDPATH= cd -- "${ROOT}/../OpenixCLI" && pwd)"
else
	OPENIXCLI_DIR="${ROOT}/.deps/OpenixCLI"
	OPENIXCLI_MANAGED=1
fi

for tool in git make gcc python3 cpio zstd rsync cargo; do
	command -v "${tool}" >/dev/null 2>&1 || {
		echo "missing required host tool: ${tool}" >&2
		exit 2
	}
done

if [ "${OPENIXCLI_MANAGED}" -eq 1 ]; then
	mkdir -p "${ROOT}/.deps"
	if [ ! -d "${OPENIXCLI_DIR}/.git" ]; then
		git clone --filter=blob:none --no-checkout "${OPENIXCLI_GIT_URL}" "${OPENIXCLI_DIR}"
	fi
	git -C "${OPENIXCLI_DIR}" fetch --no-tags origin "${OPENIXCLI_COMMIT}"
	git -C "${OPENIXCLI_DIR}" checkout --detach --force "${OPENIXCLI_COMMIT}"
else
	[ -f "${OPENIXCLI_DIR}/Cargo.toml" ] || {
		echo "OpenixCLI not found at ${OPENIXCLI_DIR}" >&2
		exit 2
	}
	[ -z "$(git -C "${OPENIXCLI_DIR}" status --porcelain --untracked-files=no)" ] || {
		echo "Refusing a locally modified OpenixCLI tree: ${OPENIXCLI_DIR}" >&2
		exit 2
	}
fi

openix_head="$(git -C "${OPENIXCLI_DIR}" rev-parse HEAD)"
[ "${openix_head}" = "${OPENIXCLI_COMMIT}" ] || {
	echo "OpenixCLI revision mismatch: expected ${OPENIXCLI_COMMIT}, got ${openix_head}" >&2
	exit 2
}

echo "[1/5] Testing pinned OpenixCLI sources"
cargo test --locked --manifest-path "${OPENIXCLI_DIR}/Cargo.toml"
echo "[2/5] Building OpenixCLI release binary"
cargo build --release --locked --manifest-path "${OPENIXCLI_DIR}/Cargo.toml"
echo "[3/5] Building and packaging the T113S3 Pro mainline system"
"${ROOT}/scripts/build-and-package.sh"
echo "[4/5] Running repository and artifact acceptance gates"
"${ROOT}/scripts/validate-local.py"

echo "[5/5] Preparing optional licensed FES bundle"
if [ -n "${TINA_SDK:-}" ]; then
	FES_BUILD="${ROOT}/out/t113s3pro-mainline-fes-build"
	"${ROOT}/scripts/package-fes-components.sh" "${FES_BUILD}"
	if [ -n "${FES_BOOTSTRAP_LOADER:-}" ]; then
		FES_BUNDLE="${ROOT}/out/t113s3pro-mainline-fes"
		"${ROOT}/scripts/prepare-fes-bundle.py" \
			--bootstrap "${FES_BOOTSTRAP_LOADER}" \
			--firmware-package "${FES_BUILD}/mainline-spinand-components.img" \
			--boot0 "${FES_BUILD}/boot0-mainline.bin" \
			--boot1 "${FES_BUILD}/boot1-mainline.img" \
			--boot "${FES_BUILD}/boot.itb" \
			--rootfs "${FES_BUILD}/rootfs.ubifs" \
			--output "${FES_BUNDLE}"
		"${OPENIXCLI_DIR}/target/release/openixcli" --output jsonl \
			flash-nand-components --preflight-only \
			--manifest "${FES_BUNDLE}/manifest.json" \
			--device-location libusb:0:0 --bus 0 --port 0 \
			--mode full_erase --post-action none
	else
		echo "FES_BUNDLE_SKIPPED: set FES_BOOTSTRAP_LOADER to an authorized board loader"
	fi
else
	echo "FES_PACKAGE_SKIPPED: set TINA_SDK and FES_BOOTSTRAP_LOADER for the licensed FES route"
fi

echo
echo "Build complete"
echo "OpenixCLI: ${OPENIXCLI_DIR}/target/release/openixcli"
echo "Artifacts: ${ROOT}/out/t113s3pro-mainline-fel"
echo "FES flash (after licensed bundle creation): OPENIXCLI_BIN=${OPENIXCLI_DIR}/target/release/openixcli ${ROOT}/scripts/flash-fes-nand.sh ${ROOT}/out/t113s3pro-mainline-fes"
