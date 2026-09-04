# Agent requirements

These rules apply to every agent changing or consuming this repository.

1. Treat every generated loader as a RAM-only bootstrap artifact. Never flash its FES, vendor U-Boot, MBR, DTB, or configuration entries to target storage.
2. Keep the final system image as a separate input. Never embed a root filesystem, mainline raw image, partition payload, Boot0, or boot package in a loader profile.
3. Name outputs exactly `<chip>-<memory-type>-<storage-type>-<product-name>-loader.bin` using normalized lowercase kebab-case tokens.
4. Use `loader.json` as the source of truth. Do not infer hardware compatibility from a filename alone.
5. Require the six v1 roles: `mbr-placeholder`, `sys-config`, `board-config`, `dtb-config`, `fes1`, and `uboot`. Do not accept duplicate roles or unexpected IMAGEWTY types.
6. Pin and verify SHA-256 for every input before invoking a packer. Fail closed on any mismatch.
7. Build through `python3 tools/allwinner_loader.py`; do not hand-edit generated `.bin` or `.manifest.json` files.
8. Run `make check` before committing. For a release, run `make dist` and verify `dist/SHA256SUMS`.
9. Preserve provenance. Third-party blobs and vendor tools must carry a source URL, source revision, and `NOASSERTION` license marker unless an authoritative license is known.
10. Do not claim hardware support from a software-only test. Set `hardware_validation` accurately and include evidence for any `hil-passed` profile.
11. A profile change that alters an input hash, hardware identity, entry order, or output hash requires a new release tag.
