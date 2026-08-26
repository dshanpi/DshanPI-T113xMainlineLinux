#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
TINA_SDK="${TINA_SDK:-}"
OUTPUT="${1:-${ROOT}/out/t113s3pro-mainline-fes-build}"
BUILD_OUTPUT="${BUILD_OUTPUT:-${ROOT}/out/mainline/t113s3pro}"
FEL_OUTPUT="${FEL_OUTPUT:-${ROOT}/out/t113s3pro-mainline-fel}"

if [ -z "${TINA_SDK}" ]; then
	echo "Set TINA_SDK to an authorized T113 Tina SDK checkout." >&2
	exit 2
fi

UBOOT="${BUILD_OUTPUT}/build/uboot-2026.07"
IMAGES="${BUILD_OUTPUT}/images"
BASE="${TINA_PACK_OUT:-${TINA_SDK}/out/t113/evb1_auto_nand/pack_out}"
TOOLS="${TINA_SDK}/tools/pack/pctools/linux"
SCRIPT_TOOL="${TOOLS}/mod_update/script"
UPDATE_MBR="${TOOLS}/mod_update/update_mbr"
DRAGON="${TOOLS}/eDragonEx/dragon"
SPL="${UBOOT}/spl/sunxi-spl.bin"
UBOOT_BIN="${UBOOT}/u-boot.bin"
MKIMAGE="${UBOOT}/tools/mkimage"
BOOT="${FEL_OUTPUT}/boot.itb"
ROOTFS="${IMAGES}/rootfs.ubifs"

for file in "${BASE}/image.cfg" "${SCRIPT_TOOL}" "${UPDATE_MBR}" "${DRAGON}" \
	"${SPL}" "${UBOOT_BIN}" "${MKIMAGE}" "${BOOT}" "${ROOTFS}"; do
	[ -f "${file}" ] || { echo "missing FES package input: ${file}" >&2; exit 1; }
done

mkdir -p "${OUTPUT}"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT INT TERM
STAGE="${TMP}/pack"
mkdir -p "${STAGE}"
cp -a "${BASE}/." "${STAGE}/"

# FES derives the Boot1 transfer length from the legacy uImage header. Pad the
# payload before mkimage so the complete image is 4 KiB aligned.
payload_size="$(stat -c %s "${UBOOT_BIN}")"
padded="$(( ((payload_size + 64 + 4095) / 4096 * 4096) - 64 ))"
dd if=/dev/zero bs=1 count="${padded}" status=none | tr '\000' '\377' > "${OUTPUT}/u-boot-page-aligned.bin"
dd if="${UBOOT_BIN}" of="${OUTPUT}/u-boot-page-aligned.bin" conv=notrunc status=none
"${MKIMAGE}" -A arm -T firmware -C none -O u-boot -a 0x42e00000 -e 0x42e00000 \
	-n "U-Boot 2026.07 for sunxi board" -d "${OUTPUT}/u-boot-page-aligned.bin" \
	"${OUTPUT}/boot1-mainline.img"
[ "$(( $(stat -c %s "${OUTPUT}/boot1-mainline.img") % 4096 ))" -eq 0 ] || exit 1

cp "${SPL}" "${OUTPUT}/boot0-mainline.bin"
cp "${BOOT}" "${OUTPUT}/boot.itb"
cp "${ROOTFS}" "${OUTPUT}/rootfs.ubifs"
rm -f "${STAGE}/boot0_nand.fex" "${STAGE}/u-boot.fex" \
	"${STAGE}/boot.fex" "${STAGE}/rootfs-ubifs.fex"
cp "${OUTPUT}/boot0-mainline.bin" "${STAGE}/boot0_nand.fex"
cp "${OUTPUT}/boot1-mainline.img" "${STAGE}/u-boot.fex"
cp "${OUTPUT}/boot.itb" "${STAGE}/boot.fex"
cp "${OUTPUT}/rootfs.ubifs" "${STAGE}/rootfs-ubifs.fex"

# FES maps these entries to UBI volumes inside the physical sys MTD region.
# Sizes are expressed in sectors and aligned to its 504-sector logical block.
cat > "${STAGE}/sys_partition.fex" <<'EOF'
[mbr]
size = 252

[partition_start]

[partition]
    name         = boot
    size         = 16632
    downloadfile = "boot.fex"
    user_type    = 0x8000
    verify       = 1

[partition]
    name         = rootfs
    size         = 81900
    downloadfile = "rootfs-ubifs.fex"
    user_type    = 0x8000
    verify       = 1

[partition]
    name         = UDISK
    user_type    = 0x8100
EOF

cat > "${STAGE}/image.cfg" <<'EOF'
[MAIN_TYPE]
ITEM_COMMON = "COMMON  "
ITEM_BOOT = "BOOT    "

[FILELIST]
    {filename = "sys_partition.fex", maintype = ITEM_COMMON, subtype = "SYS_CONFIG000000",},
    {filename = "boot0_nand.fex", maintype = ITEM_BOOT, subtype = "BOOT0_0000000000",},
    {filename = "u-boot.fex", maintype = "12345678", subtype = "UBOOT_0000000000",},
    {filename = "sunxi_mbr.fex", maintype = "12345678", subtype = "1234567890___MBR",},
    {filename = "dlinfo.fex", maintype = "12345678", subtype = "1234567890DLINFO",},

[IMAGE_CFG]
version = 0x100234
pid = 0x00001234
vid = 0x00008743
hardwareid = 0x100
firmwareid = 0x100
bootromconfig = "bootrom_071203_00001234.cfg"
rootfsconfig = "rootfs.cfg"
filelist = FILELIST
encrypt = 0
imagename = mainline-spinand-components.img
EOF

sed -i 's/$/\r/' "${STAGE}/sys_partition.fex"
(
	cd "${STAGE}"
	"${SCRIPT_TOOL}" sys_partition.fex
	"${UPDATE_MBR}" sys_partition.bin 4
	"${UPDATE_MBR}" sys_partition.bin 4 sunxi_mbr.fex dlinfo.fex 61079552 40960 0
	"${DRAGON}" image.cfg sys_partition.fex
)
cp "${STAGE}/mainline-spinand-components.img" "${OUTPUT}/"
(
	cd "${OUTPUT}"
	sha256sum boot0-mainline.bin boot1-mainline.img boot.itb rootfs.ubifs \
		mainline-spinand-components.img > COMPONENT_SHA256SUMS
)
echo "FES_COMPONENT_PACKAGE:${OUTPUT}/mainline-spinand-components.img"
echo "STATUS:experimental-pending-cold-boot"
