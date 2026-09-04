#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
SDK_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/../../../../.." && pwd)"
IMAGES_DIR="${1:-${SDK_ROOT}/out/mainline/t113s3pro/images}"
TARGET_DIR="${2:-${SDK_ROOT}/out/mainline/t113s3pro/target}"
BUILD_DIR="${3:-${SDK_ROOT}/out/mainline/t113s3pro/build/uboot-2026.07}"
OUTPUT_DIR="${4:-${SDK_ROOT}/out/t113s3pro-mainline-nand}"
HOST_DIR="${HOST_DIR:-$(dirname -- "$(dirname -- "${BUILD_DIR}")")/host}"
DTB="${IMAGES_DIR}/allwinner/sun8i-t113s-dshanpi-t113s3pro.dtb"
SPL="${BUILD_DIR}/spl/sunxi-spl.bin"
UBOOT_BIN="${BUILD_DIR}/u-boot.bin"
UBOOT_IMG="${BUILD_DIR}/u-boot.img"
UBOOT_ELF="${BUILD_DIR}/u-boot"
MKIMAGE="${BUILD_DIR}/tools/mkimage"
DTC="${HOST_DIR}/bin/dtc"
FDTOVERLAY="${HOST_DIR}/bin/fdtoverlay"
UBINIZE="${HOST_DIR}/sbin/ubinize"
PAYLOAD_ADDRESS=0x44800000
PAYLOAD_LIMIT=25165824
INSTALLER_ADDRESS=0x44000000
INSTALLER_LIMIT=8388608

if [ ! -f "${SPL}" ]; then
	echo "MAINLINE_SPL_REQUIRED: ${SPL}" >&2
	exit 1
fi
for required in "${IMAGES_DIR}/zImage" "${DTB}" \
	"${UBOOT_BIN}" "${UBOOT_IMG}" "${UBOOT_ELF}" "${MKIMAGE}" \
	"${DTC}" "${FDTOVERLAY}" "${UBINIZE}" \
	"${SCRIPT_DIR}/kernel-fit.its.in" "${IMAGES_DIR}/rootfs.ubifs"; do
	if [ ! -f "${required}" ]; then
		echo "missing mainline FEL artifact: ${required}" >&2
		exit 1
	fi
done

mkdir -p "${OUTPUT_DIR}"

python3 "${SCRIPT_DIR}/verify-mainline-uboot.py" \
	--spl "${SPL}" --uboot-bin "${UBOOT_BIN}" \
	--uboot-img "${UBOOT_IMG}" --uboot-elf "${UBOOT_ELF}" \
	--version 2026.07 \
	--board allwinner/sun8i-t113s-dshanpi-t113s3pro
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

# Build the runtime FIT directly with the mainline mkimage.  This pure FEL
# path does not invoke or consume a Tina/FES container packer.
sed -e "s|@KERNEL@|${IMAGES_DIR}/zImage|" \
	-e "s|@DTB@|${DTB}|" \
	"${SCRIPT_DIR}/kernel-fit.its.in" > "${TMP_DIR}/kernel-fit.its"
"${MKIMAGE}" -f "${TMP_DIR}/kernel-fit.its" "${OUTPUT_DIR}/boot.itb"

# Build the physical layout shared by the mainline recovery installer and FES:
# 1 MiB SPL, 3 MiB Boot1, 1 MiB secure-storage, then one 251 MiB UBI device.
# FES creates the same boot/rootfs volumes inside sys, so both installation
# routes leave an identical persistent mainline layout.
# Repeating the eGON SPL once per eraseblock gives BootROM eight candidates.
dd if=/dev/zero bs=1 count=1048576 status=none | tr '\000' '\377' \
	> "${OUTPUT_DIR}/spl-redundant.bin"
for offset in 0 131072 262144 393216 524288 655360 786432 917504; do
	dd if="${SPL}" of="${OUTPUT_DIR}/spl-redundant.bin" bs=1 \
		seek="${offset}" conv=notrunc status=none
done
dd if=/dev/zero bs=1 count=3145728 status=none | tr '\000' '\377' \
	> "${OUTPUT_DIR}/uboot-redundant.bin"
dd if="${UBOOT_IMG}" of="${OUTPUT_DIR}/uboot-redundant.bin" \
	conv=notrunc status=none

cat > "${TMP_DIR}/sys-ubi.ini" <<EOF
[boot]
mode=ubi
image=${OUTPUT_DIR}/boot.itb
vol_id=0
vol_type=static
vol_name=boot
vol_size=8388608

[rootfs]
mode=ubi
image=${IMAGES_DIR}/rootfs.ubifs
vol_id=1
vol_type=dynamic
vol_name=rootfs
vol_flags=autoresize
EOF
"${UBINIZE}" -m 2048 -p 131072 -s 2048 \
	-o "${OUTPUT_DIR}/sys.ubi" "${TMP_DIR}/sys-ubi.ini"

# Only these three media images are written to NAND. boot.itb is retained in
# the output for FES packaging but is already embedded in sys.ubi, so the FEL
# payload carries only its size/hash metadata instead of a duplicate 6 MiB copy.
# The archive is recovered from a
# reserved DRAM range by mainline Linux; it is never interpreted by BootROM or
# U-Boot and has no vendor container.
PAYLOAD_STAGE="${TMP_DIR}/payload"
mkdir -p "${PAYLOAD_STAGE}"
cp -f "${OUTPUT_DIR}/spl-redundant.bin" \
	"${OUTPUT_DIR}/uboot-redundant.bin" \
	"${OUTPUT_DIR}/sys.ubi" "${PAYLOAD_STAGE}/"
{
	echo "boot_size=$(stat -c %s "${OUTPUT_DIR}/boot.itb")"
	echo "boot_sha256=$(sha256sum "${OUTPUT_DIR}/boot.itb" | cut -d ' ' -f 1)"
} > "${PAYLOAD_STAGE}/BOOT_VOLUME"
(
	cd "${PAYLOAD_STAGE}"
	sha256sum spl-redundant.bin uboot-redundant.bin sys.ubi BOOT_VOLUME > SHA256SUMS
)
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
	-czf "${OUTPUT_DIR}/fel-payload.tar.gz" \
	-C "${PAYLOAD_STAGE}" spl-redundant.bin uboot-redundant.bin \
	sys.ubi BOOT_VOLUME SHA256SUMS
payload_size="$(stat -c %s "${OUTPUT_DIR}/fel-payload.tar.gz")"
if [ "${payload_size}" -gt "${PAYLOAD_LIMIT}" ]; then
	echo "FEL payload exceeds reserved 24 MiB: ${payload_size}" >&2
	exit 1
fi
payload_sha="$(sha256sum "${OUTPUT_DIR}/fel-payload.tar.gz")"
payload_sha="${payload_sha%% *}"
{
	echo "payload_address=${PAYLOAD_ADDRESS}"
	echo "payload_size=${payload_size}"
	echo "payload_sha256=${payload_sha}"
} > "${OUTPUT_DIR}/fel-payload-layout"

"${SCRIPT_DIR}/make-fel-installer-initramfs.sh" \
	"${TARGET_DIR}" "${OUTPUT_DIR}/fel-base-initramfs.cpio.gz" \
	"${OUTPUT_DIR}/fel-payload-layout"
"${DTC}" -Wno-avoid_unnecessary_addr_size -@ -I dts -O dtb \
	-o "${OUTPUT_DIR}/fel-payload-reserved-memory.dtbo" \
	"${SCRIPT_DIR}/fel-payload-reserved-memory.dtso"
"${FDTOVERLAY}" -i "${DTB}" \
	-o "${OUTPUT_DIR}/fel-installer.dtb" \
	"${OUTPUT_DIR}/fel-payload-reserved-memory.dtbo"
sed -e "s|@KERNEL@|${IMAGES_DIR}/zImage|" \
	-e "s|@DTB@|${OUTPUT_DIR}/fel-installer.dtb|" \
	-e "s|@RAMDISK@|${OUTPUT_DIR}/fel-base-initramfs.cpio.gz|" \
	"${SCRIPT_DIR}/installer-fit.its.in" > "${TMP_DIR}/fel-installer.its"
"${MKIMAGE}" -f "${TMP_DIR}/fel-installer.its" \
	"${OUTPUT_DIR}/fel-installer.itb"
installer_size="$(stat -c %s "${OUTPUT_DIR}/fel-installer.itb")"
if [ "${installer_size}" -gt "${INSTALLER_LIMIT}" ]; then
	echo "FEL installer overlaps payload address: ${installer_size}" >&2
	exit 1
fi

rm -f "${OUTPUT_DIR}"/fel-payload.part-*
# LYNX accepts at most eight mainline artifacts.  SPL, U-Boot proper and the
# installer consume three slots, so keep the payload within five chunks.
# PAYLOAD_LIMIT is 24 MiB, therefore 5 MiB chunks always satisfy that limit.
split -b 5242880 -d -a 2 "${OUTPUT_DIR}/fel-payload.tar.gz" \
	"${OUTPUT_DIR}/fel-payload.part-"
cp -f "${SPL}" "${OUTPUT_DIR}/fel-sunxi-spl.bin"
cp -f "${UBOOT_BIN}" "${OUTPUT_DIR}/fel-u-boot.bin"

# Remove every stale artifact from the abandoned vendor-container and
# separate-installer experiments so it cannot be selected accidentally.
rm -f "${OUTPUT_DIR}/installer-android-v2.img" \
	"${OUTPUT_DIR}/installer-android-v2-auto.img" \
	"${OUTPUT_DIR}/openix-fel-mainline-auto.img" \
	"${OUTPUT_DIR}/fel-installer-chunked.itb" \
	"${OUTPUT_DIR}/fel-installer-chunked.its" \
	"${OUTPUT_DIR}/fel-installer-initramfs.cpio.gz" \
	"${OUTPUT_DIR}/fel-installer.its" \
	"${OUTPUT_DIR}/installer-initramfs.cpio.gz" \
	"${OUTPUT_DIR}/ubi-sim-extract"

(
	cd "${OUTPUT_DIR}"
	sha256sum fel-sunxi-spl.bin fel-u-boot.bin fel-installer.itb \
		fel-payload.part-* > FEL_SHA256SUMS
)
uboot_address="$(sed -n 's/^CONFIG_TEXT_BASE=//p' "${BUILD_DIR}/.config")"
{
	echo "chain=BootROM_FEL-mainline_SPL-mainline_U-Boot-mainline_Linux_MTD_UBI"
	echo "spl=fel-sunxi-spl.bin"
	echo "uboot=fel-u-boot.bin"
	echo "uboot_address=${uboot_address}"
	echo "installer=fel-installer.itb"
	echo "installer_address=${INSTALLER_ADDRESS}"
	echo "payload_address=${PAYLOAD_ADDRESS}"
	echo "payload_size=${payload_size}"
	echo "payload_parts=$(find "${OUTPUT_DIR}" -maxdepth 1 -name 'fel-payload.part-*' -printf '%f\n' | sort | tr '\n' ' ')"
} > "${OUTPUT_DIR}/FEL_ARTIFACTS"

echo "Pure-mainline FEL artifacts: ${OUTPUT_DIR}"
