# DshanPi T113-S3 Pro DDR3 SPI NAND loader

Canonical artifact: `t113s3-ddr3-spinand-dshanpi-t113s3pro-loader.bin`

This profile contains the six preprocessed IMAGEWTY inputs from the recorded Tina SDK build. `input/sys_config.source.fex` is retained as readable provenance for the compiled `sys_config.bin`; it is not a seventh container entry.

Observed RAM-bootstrap evidence:

- BootROM FEL Device ID: `0x00185900`;
- vendor FES Device ID: `0x00161000`;
- FES USB mode: `Srv`;
- target storage identified as SPI NAND;
- no claim of successful NAND write (`hardware_validation = "ram-bootstrap-passed"`).

The MBR entry exists only for IMAGEWTY compatibility and must never be written to media by a loader consumer.

