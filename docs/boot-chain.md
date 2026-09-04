# Boot and installation chain

## Installation boot

1. T113 BootROM exposes FEL USB.
2. OpenixCLI validates the artifact plan and the exact mainline eGON SPL.
3. The SPL runs from SRAM, initializes 128 MiB DDR3, then returns through the
   R528 SRAM swap/thunk to BootROM FEL.
4. OpenixCLI loads same-build U-Boot proper at `0x42e00000`.
5. It loads the installer FIT at `0x44000000` and payload chunks beginning at
   `0x44800000`.
6. U-Boot executes the installer FIT.
7. Linux 6.18.8 starts from RAM, recovers the payload from reserved DRAM, and
   uses MTD/UBI tools to install the system.

## Cold boot

1. BootROM reads a mainline eGON SPL from SPI-NAND.
2. R528 reports boot-media value 4; the board patch maps it to the SPI-NAND SPL
   loader.
3. SPL reads mainline U-Boot proper from offset `0x00100000`.
4. U-Boot reads the FIT from the `boot` MTD partition.
5. Linux attaches MTD partition `sys` as UBI device 0.
6. Linux mounts UBI volume `rootfs` as UBIFS.

The permanent command line is:

```text
earlycon=uart8250,mmio32,0x02500c00 console=ttyS3,115200 \
ignore_loglevel loglevel=8 clk_ignore_unused \
ubi.mtd=sys root=ubi0:rootfs rootfstype=ubifs rw rootwait
```
