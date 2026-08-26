#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
python3 "${ROOT}/scripts/source_lock.py" "${ROOT}/manifests/sources.lock"
. "${ROOT}/manifests/sources.lock"
TREE="${ROOT}/buildroot/buildroot-mainline"

if [ -d "${TREE}/.git" ] && ! git -C "${TREE}" rev-parse --verify HEAD >/dev/null 2>&1; then
	broken="${TREE}.incomplete.$(date +%Y%m%d%H%M%S)"
	mv "${TREE}" "${broken}"
	echo "Moved incomplete Buildroot clone to: ${broken}"
fi

if [ ! -d "${TREE}/.git" ]; then
	mkdir -p "${ROOT}/buildroot"
	git clone --filter=blob:none --no-checkout "${BUILDROOT_GIT_URL}" "${TREE}"
fi

git -C "${TREE}" remote set-url origin "${BUILDROOT_GIT_URL}"

current="$(git -C "${TREE}" rev-parse HEAD)"
if [ "${current}" != "${BUILDROOT_COMMIT}" ]; then
	git -C "${TREE}" fetch origin "${BUILDROOT_COMMIT}"
	git -C "${TREE}" checkout --detach "${BUILDROOT_COMMIT}"
fi

mkdir -p "${TREE}/board/dshanpi"
cp -a "${ROOT}/board/dshanpi/t113s3pro" "${TREE}/board/dshanpi/"
cp -a "${ROOT}/configs/dshanpi_t113s3pro_nand_defconfig" "${TREE}/configs/"

echo "Buildroot ready: ${TREE} @ ${BUILDROOT_COMMIT}"
