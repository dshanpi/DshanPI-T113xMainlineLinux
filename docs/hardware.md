# Hardware facts

| Item | Validated value |
|---|---|
| Board | DshanPi T113S3 Pro |
| SoC | Allwinner T113-S3 / R528 |
| CPU | 2 x Cortex-A7 |
| DRAM | 128 MiB DDR3 |
| Debug UART | UART3, PB6/PB7, 115200 8N1 |
| SPI-NAND | Winbond W25N02KV, ID `ef aa 22` |
| NAND capacity | 256 MiB |
| Page/OOB | 2048/128 bytes |
| Erase block | 128 KiB |
| Ethernet PHY | RTL8201F, RMII |
| BootROM FEL USB | `1f3a:efe8` |

UART3 is at MMIO `0x02500c00` and becomes Linux `ttyS3`. Earlier UART4
assumptions were incorrect and must not be copied into new configurations.

U-Boot proper uses conservative single-I/O SPI-NAND reads. The tested U-Boot
controller path returned all `0xff` when the flash node forced quad transfers.
Linux retains the validated quad-width properties in its separate device tree.
