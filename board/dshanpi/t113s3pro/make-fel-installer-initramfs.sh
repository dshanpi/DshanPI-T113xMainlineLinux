#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
SDK_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/../../../../.." && pwd)"
TARGET_DIR="${1:-${SDK_ROOT}/out/mainline/t113s3pro/target}"
OUTPUT="${2:-${SDK_ROOT}/out/t113s3pro-mainline-nand/fel-base-initramfs.cpio.gz}"
LAYOUT="${3:-${SDK_ROOT}/out/t113s3pro-mainline-nand/fel-payload-layout}"
ROOT="$(mktemp -d)"
trap 'rm -rf "${ROOT}"' EXIT INT TERM

for required in bin/busybox lib/ld-linux-armhf.so.3 lib/libc.so.6 \
	lib/libresolv.so.2 usr/sbin/flash_erase usr/sbin/nanddump \
	usr/sbin/nandwrite usr/sbin/ubiformat usr/sbin/ubiattach \
	usr/sbin/ubidetach usr/sbin/ubinfo; do
	if [ ! -e "${TARGET_DIR}/${required}" ]; then
		echo "missing installer component: ${TARGET_DIR}/${required}" >&2
		exit 1
	fi
done
if [ ! -f "${LAYOUT}" ]; then
	echo "missing FEL payload layout: ${LAYOUT}" >&2
	exit 1
fi

mkdir -p "${ROOT}/bin" "${ROOT}/sbin" "${ROOT}/usr/sbin" \
	"${ROOT}/lib" "${ROOT}/dev" "${ROOT}/proc" "${ROOT}/sys" \
	"${ROOT}/run" "${ROOT}/tmp" "${ROOT}/mnt" "${ROOT}/payload"
cp -a "${TARGET_DIR}/bin/busybox" "${ROOT}/bin/"
for applet in base64 cat cmp dd dmesg echo grep hexdump ls mkdir mount \
	reboot rm sh sha256sum stty sync tar umount uname wc zcat; do
	ln -s busybox "${ROOT}/bin/${applet}"
done
cp -a "${TARGET_DIR}/lib/ld-linux-armhf.so.3" \
	"${TARGET_DIR}/lib/libc.so.6" "${TARGET_DIR}/lib/libresolv.so.2" \
	"${ROOT}/lib/"
cp -a "${TARGET_DIR}/usr/sbin/flash_erase" \
	"${TARGET_DIR}/usr/sbin/nanddump" \
	"${TARGET_DIR}/usr/sbin/nandwrite" \
	"${TARGET_DIR}/usr/sbin/ubiformat" \
	"${TARGET_DIR}/usr/sbin/ubiattach" \
	"${TARGET_DIR}/usr/sbin/ubidetach" \
	"${TARGET_DIR}/usr/sbin/ubinfo" "${ROOT}/usr/sbin/"
cp -a "${SCRIPT_DIR}/installer-init" "${ROOT}/init"
cp -a "${LAYOUT}" "${ROOT}/payload-layout"
chmod 0755 "${ROOT}/init"
mkdir -p "$(dirname -- "${OUTPUT}")"
(
	cd "${ROOT}"
	find . -print0 | cpio --null -o --format=newc --quiet | gzip -9
) > "${OUTPUT}"

echo "Chunked-FEL installer initramfs: ${OUTPUT}"

