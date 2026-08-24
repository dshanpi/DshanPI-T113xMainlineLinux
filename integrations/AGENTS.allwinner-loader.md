## Allwinner loader generation

When a task creates, changes, downloads, or consumes an Allwinner loader:

1. Use the `allwinner-loader-builder` Skill from `dshanpi/allwinner-loader`.
2. Build only from a validated `loader.json`; verify all source, packed-entry, and complete-output SHA-256 values.
3. Derive the output name as `<chip>-<memory-type>-<storage-type>-<product-name>-loader.bin`.
4. Keep the loader and final system image as separate artifacts and API parameters.
5. Treat loader contents as RAM-only. Never write its MBR, FES, vendor U-Boot, DTB, or configuration entries to target media.
6. Before any erase/write, verify the physical USB binding, FEL/FES Device ID mapping, expected storage type, and a nonzero capacity.
7. Do not fall back to Boot0, Boot1, SyterKit, Phoenix, a full Tina firmware, or a vendor partition flash path.
8. Do not claim hardware validation beyond the level recorded in the manifest.

Use the release-pinned composite action in CI:

```yaml
- uses: dshanpi/allwinner-loader@v1.0.0
  with:
    manifest: path/to/loader.json
    output-dir: dist
```
