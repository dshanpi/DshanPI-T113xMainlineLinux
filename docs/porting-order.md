# Mainline porting order

The order matters. Changing storage, bootloader and Linux at the same time made
several failures indistinguishable during bring-up.

1. **Freeze hardware facts.** Record SoC revision, DRAM type/size, oscillator,
   UART instance/pins, NAND ID and geometry.
2. **Prove UART and FEL.** Keep BootROM FEL detection and UART capture working
   before changing SPL.
3. **Bring up mainline SPL in RAM.** Validate eGON header/checksum, DRAM size and
   return to FEL. Do not write NAND yet.
4. **Load same-build U-Boot proper in DRAM.** Reject mixed SPL/U-Boot timestamps,
   wrong DT identity and wrong load/entry addresses.
5. **Bring up Linux in RAM.** Verify UART3, pinctrl, clocks, SPI-NAND detection,
   MTD geometry, Ethernet and rootfs tools.
6. **Freeze one physical layout.** Make U-Boot DTS, Linux DTS, installer checks,
   boot command and kernel command line agree exactly.
7. **Verify storage reads before writes.** Read eGON/FIT magic from expected
   offsets. A quad-mode mismatch can look like an erased NAND.
8. **Install through the RAM Linux environment.** Hash the payload in RAM,
   write each component and read it back.
9. **Attach and mount UBI before reboot.** A successful transfer alone is not
   installation success.
10. **Power-cycle.** Do not accept a manual U-Boot `setenv` boot as the final
    result. Cold boot must reach the login prompt with compiled-in parameters.
11. **Rebuild from clean clones.** Delete no evidence until a second clean
    checkout reproduces the same hashes and hardware result.

## Required consistency checks

- UART is `ttyS3`, not `ttyS4`.
- U-Boot and Linux agree on `spl`, `uboot`, `secure-storage`, `boot`, `sys`.
- Kernel parameter is `ubi.mtd=sys`; UBI volume name is `rootfs`.
- U-Boot proper does not force the failed quad read mode.
- U-Boot environment is `nowhere`; no absent MMC/FAT/SPI environment is read.
- `boot_media=4` is recognized as R528 SPI-NAND.
- Installer FIT and payload memory ranges do not overlap.
