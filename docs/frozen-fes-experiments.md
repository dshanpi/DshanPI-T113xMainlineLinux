# Frozen FES NAND-component experiments

Earlier work tested a different installer boundary:

```text
BootROM FEL -> Tina/IMAGEWTY loader in RAM -> FES -> mainline components
```

OpenixCLI experiments added RAM-only bootstrap selection, NAND-component
packages, storage preflight, Boot0/Boot1 verification and FES reconnect logic.
Several tasks reported successful component transfer and verification, but
these frozen historical experiments did not complete the required
cold-boot-to-mainline acceptance cycle. The redesigned, hash-pinned v5 route
later completed that cycle; see `fes-nand-provisioning.md` and the 2026-08-26
hardware evidence. Do not revive artifacts listed on this page.

This page does not promote the frozen artifacts. The historical task records
remain in `logs/t113-task-history-20260824-25.jsonl` so their failures stay
auditable.

Do not reuse the historical recovery loaders or component images as current
release artifacts. Their hashes and status are retained in the private
pre-clean backup and development journal.
