# Vendor compatibility tools

These are unmodified binary tools used by Tina-compatible packing flows:

- `script`: compile text FEX configuration;
- `update_mbr`: prepare a partition MBR;
- `update_fes1`: merge configuration into FES1;
- `update_uboot`: merge configuration into vendor U-Boot;
- `eDragonEx/dragon` and plugins: build IMAGEWTY containers.

The standard builder consumes already-prepared profile inputs and invokes only `eDragonEx/dragon`. The remaining original tools are retained for source-artifact preparation and compatibility research. See `SOURCE.json` and the repository `NOTICE.md` before redistribution.
