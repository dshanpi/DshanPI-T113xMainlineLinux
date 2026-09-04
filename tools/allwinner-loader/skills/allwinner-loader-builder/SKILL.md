---
name: allwinner-loader-builder
description: Build, inspect, verify, or publish RAM-only Allwinner FEL-to-FES IMAGEWTY loader binaries from hardware manifests. Use when Codex works with Allwinner loader.bin artifacts, FES1/U-Boot/sys_config/DTB/board configuration inputs, loader naming by chip-memory-storage-product, adding a new board profile, or integrating another repository with dshanpi/allwinner-loader automation.
---

# Allwinner Loader Builder

Use the repository's deterministic wrapper and fail closed on identity or hash mismatches.

## Locate the tool

Prefer the current repository when it contains `tools/allwinner_loader.py`. Otherwise use `ALLWINNER_LOADER_REPO` when set. If neither is available, clone `https://github.com/dshanpi/allwinner-loader.git` into a task-specific temporary directory.

Read `references/loader-contract.md` before adding or changing a profile. Also obey the target repository's `AGENTS.md`.

## Build or verify

1. Locate `loader.json` and all referenced inputs.
2. Confirm the requested chip, DRAM type/size/clock, storage type, product, FEL/FES Device IDs, and source provenance from evidence. Do not guess hardware identity from a filename.
3. Run:

```bash
python3 tools/allwinner_loader.py build \
  --manifest path/to/loader.json \
  --output-dir dist \
  --check-reproducible
```

4. Run `verify` against the resulting canonical `.bin`.
5. Report the canonical filename, size, SHA-256, validation level, and the six entry hashes.

## Add a profile

Copy an existing profile structure, never its hardware values. Use exactly six ordered RAM-only entries. Pin every pre-pack `sha256` and post-pack `packed_sha256`; they may differ when `dragon` updates an internal checksum. Set `hardware_validation` to `software-only` until RAM bootstrap evidence exists; use `hil-passed` only after a complete media write and verification.

Run `make check` after any profile, tool, action, or specification change.

## Integrate another repository

Prefer the composite GitHub Action:

```yaml
- uses: dshanpi/allwinner-loader@v1.0.0
  with:
    manifest: path/to/loader.json
    output-dir: dist
```

Keep loader and final system image as separate artifacts and inputs. Never pass a loader to a raw-media write function.

## Safety boundaries

- Use FES and vendor U-Boot only in RAM to obtain USB Product/FES service.
- Never add Boot0, Boot1, a root filesystem, partitions, or a full Tina image to a loader.
- Never silently substitute a loader from another memory, storage, board, or chip variant.
- Stop before hardware access when a manifest, entry hash, Device ID, storage identity, or capacity check fails.
