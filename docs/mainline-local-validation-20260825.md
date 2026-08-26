# Mainline local validation — 2026-08-25

Branch under test: `feat/mainline-fel-ram-installer`, including the
two-repository automation hardening update.

The mainline worker was tested in both the library and binary targets. Shared
tests intentionally run twice because OpenixCLI currently compiles the command
module in both targets.

## Rust test results

Library target, 13 passed:

1. `[PASS] spl_return_reconnect_never_migrates_to_another_endpoint`
2. `[PASS] worker_opens_only_the_bound_fel_endpoint`
3. `[PASS] worker_requires_location_to_match_current_bus_and_port`
4. `[PASS] chunk_addresses_add_each_offset_exactly_once`
5. `[PASS] entry_must_be_one_declared_load_address`
6. `[PASS] helper_rejects_dram_bootloader_without_verified_spl_before_device_open`
7. `[PASS] helper_rejects_low_and_overlapping_load_addresses`
8. `[PASS] mainline_spl_requires_exact_egon_spl_file_and_checksum`
9. `[PASS] r528_bootstrap_disables_icache_swaps_sram_and_requires_fel_return`
10. `[PASS] unsupported_soc_is_rejected_before_any_sram_write`
11. `[PASS] validated_plan_requires_one_spl_and_one_bootloader`
12. `[PASS] validated_snapshot_is_immutable_from_later_source_changes_and_is_cleaned_up`
13. `[PASS] completion_event_is_scoped_to_ram_handoff`

Binary target, 15 passed:

14. `[PASS] parses_scoped_mainline_worker_plan`
15. `[PASS] text_output_remains_default`
16. `[PASS] spl_return_reconnect_never_migrates_to_another_endpoint`
17. `[PASS] worker_opens_only_the_bound_fel_endpoint`
18. `[PASS] worker_requires_location_to_match_current_bus_and_port`
19. `[PASS] chunk_addresses_add_each_offset_exactly_once`
20. `[PASS] entry_must_be_one_declared_load_address`
21. `[PASS] helper_rejects_dram_bootloader_without_verified_spl_before_device_open`
22. `[PASS] helper_rejects_low_and_overlapping_load_addresses`
23. `[PASS] mainline_spl_requires_exact_egon_spl_file_and_checksum`
24. `[PASS] r528_bootstrap_disables_icache_swaps_sram_and_requires_fel_return`
25. `[PASS] unsupported_soc_is_rejected_before_any_sram_write`
26. `[PASS] validated_plan_requires_one_spl_and_one_bootloader`
27. `[PASS] validated_snapshot_is_immutable_from_later_source_changes_and_is_cleaned_up`
28. `[PASS] completion_event_is_scoped_to_ram_handoff`

Doc tests: 0 tests, 0 failures.

## Additional gates

29. `[PASS] rustfmt --check on mainline-touched Rust files`
30. `[PASS] cargo check --locked --all-targets`
31. `[PASS] cargo clippy --locked --all-targets -- -D warnings`
32. `[PASS] cargo build --release --locked`
33. `[PASS] release binary exposes boot-mainline`
34. `[PASS] boot-mainline exposes exact device-location binding`
35. `[PASS] boot-mainline exposes text/jsonl output selection`
36. `[PASS] libefex revision is pinned in Cargo.toml and Cargo.lock`
37. `[PASS] completion JSONL is scoped to RAM handoff with installer unobserved`
38. `[PASS] JSONL scan emits physical device identity for strict wrappers`
39. `[PASS] src/flash/mod.rs remains byte-for-byte unchanged from the base branch`

Whole-tree `cargo fmt --check` is not used as this feature gate because the
upstream baseline `src/flash/mod.rs` is already unformatted. Formatting that
file would violate this release's explicit requirement to leave the frozen FES
implementation byte-for-byte unchanged.

Run all executable gates with:

```sh
./scripts/validate-mainline.sh
```

## Hardware qualification

These executable gates validate parsing, bounds, hashing, endpoint selection,
SRAM thunk construction and build quality without requiring a connected board.
The companion T113 repository's source-recovery build was subsequently tested
with this exact OpenixCLI commit (`f10ff48cf938d5a85e45e2a78f241f6602baff06`).
Lynx task `mainline-1787715829104265529` completed the board-side installer at
100% with exit code 0. The board then mounted `sys/rootfs` and reached
`t113s3pro-mainline login:` after the installer reboot and after two controlled
power-off cold boots.

The sanitized task, power-cycle and UART-marker evidence is preserved in the
companion branch at
`logs/source-rebuild-hardware-validation-20260825.jsonl`; the exact artifact
hashes are in
`manifests/hardware-verified-source-rebuild-20260825.sha256`. This qualifies the
tested DshanPi T113S3 Pro and Winbond W25N02KV development/recovery workflow. It
does not establish a general manufacturing guarantee for other NAND devices or
bad-block populations. No automatic retry is allowed after a terminal FEL
failure.
