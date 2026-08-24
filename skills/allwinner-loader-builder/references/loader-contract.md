# Loader consumer contract

The loader artifact is a transport bootstrap, not a system firmware.

Required flow:

```text
validate manifest and hashes
-> bind one physical USB device
-> enter BootROM FEL
-> load FES1 and board data into RAM
-> run vendor U-Boot in RAM
-> release stale FEL handle
-> match the unique FES re-enumeration
-> verify FES Device ID, storage type, and nonzero capacity
-> write and verify a separate final-system image
```

Before building a new profile, collect authoritative values for:

- public chip name and internal SoC family;
- BootROM FEL and vendor FES Device IDs;
- DRAM type, size, clock, and configuration source;
- storage type and capacity/geometry;
- stable board/product name;
- SDK/source revision and SHA-256 of all six inputs.

The canonical name has exactly four identity fields followed by `loader.bin`:

```text
<chip>-<memory-type>-<storage-type>-<product-name>-loader.bin
```

Detailed format requirements live in the repository's `docs/loader-standard-v1.md`; read that document when modifying the schema or container layout.
