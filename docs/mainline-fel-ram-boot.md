# Mainline FEL RAM boot

`openixcli boot-mainline` executes a bounded, hash-verified Allwinner FEL RAM
load plan. It was developed for the DshanPi T113S3 Pro/R528 mainline bring-up
flow and deliberately does not accept arbitrary commands or arbitrary entry
points.

This command performs the RAM stage only:

1. validate and snapshot every artifact;
2. load an exact mainline eGON SPL at `0x00020000`;
3. run the audited R528 SRAM swap/return thunk;
4. require the SPL to return to BootROM FEL;
5. load mainline U-Boot and the declared installer artifacts into DRAM;
6. execute the declared U-Boot entry address.

A successful `complete` event proves that all RAM artifacts were transferred
and the entry point was executed. It does **not** by itself prove that NAND was
written. The mainline Linux installer must separately report MTD writes,
readback verification, UBI/UBIFS validation, and a successful cold boot.

## Plan format

```json
{
  "artifacts": [
    {
      "role": "spl",
      "filePath": "/absolute/path/fel-sunxi-spl.bin",
      "loadAddress": 131072,
      "sha256": "<64 hexadecimal characters>"
    },
    {
      "role": "bootloader",
      "filePath": "/absolute/path/fel-u-boot.bin",
      "loadAddress": 1121976320,
      "sha256": "<64 hexadecimal characters>"
    },
    {
      "role": "kernel",
      "filePath": "/absolute/path/fel-installer.itb",
      "loadAddress": 1140850688,
      "sha256": "<64 hexadecimal characters>"
    }
  ],
  "entryAddress": 1121976320
}
```

Additional installer payload chunks use the `initramfs` role with explicit,
non-overlapping DRAM addresses.

## Invocation

```sh
openixcli --output jsonl boot-mainline \
  --plan /absolute/path/plan.json \
  --device-location libusb:3:2 \
  --bus 3 --port 2
```

`device-location` must exactly match `libusb:BUS:PORT`. The worker opens only
that endpoint and does not migrate to another USB device during SPL return.

## Safety boundary

- one R528/T113 mainline SPL is mandatory and must be first;
- one U-Boot proper artifact is mandatory;
- the entry address must equal the U-Boot load address;
- the SPL header, exact length and eGON checksum are verified;
- artifact roles are enumerated;
- paths must be absolute;
- load addresses must be aligned and non-overlapping;
- each file is hashed into a private snapshot before USB is opened;
- maximum artifact count is 8, maximum file size is 512 MiB, and maximum plan
  size is 768 MiB;
- no automatic retry is performed after a terminal failure.

The validated T113S3 Pro image generator, installer, memory map and complete
hardware log live in the companion `DshanPI-T113xMainlineLinux` repository.
