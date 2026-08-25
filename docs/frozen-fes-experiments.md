# Frozen FES NAND-component experiments

Earlier work tested a different installer boundary:

```text
BootROM FEL -> Tina/IMAGEWTY loader in RAM -> FES -> mainline components
```

OpenixCLI experiments added RAM-only bootstrap selection, NAND-component
packages, storage preflight, Boot0/Boot1 verification and FES reconnect logic.
Several tasks reported successful component transfer and verification, but the
route did not complete the required cold-boot-to-mainline acceptance cycle.

This version does not include, alter, optimize or advertise that route. The
historical task records remain in `logs/t113-task-history-20260824-25.jsonl` so
the investigation can resume in a later version without losing evidence.

Do not reuse the historical recovery loaders or component images as current
release artifacts. Their hashes and status are retained in the private
pre-clean backup and development journal.
