# Mainline local validation — 2026-08-25

Commit under test: `5e837a442bc73c72b199805a6f49968a213eb8f9` plus the validation-only files
added by the following commit.

The mainline worker was tested in both the library and binary targets. Shared
tests intentionally run twice because OpenixCLI currently compiles the command
module in both targets.

## Rust test results

Library target, 12 passed:

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

Binary target, 14 passed:

13. `[PASS] parses_scoped_mainline_worker_plan`
14. `[PASS] text_output_remains_default`
15. `[PASS] spl_return_reconnect_never_migrates_to_another_endpoint`
16. `[PASS] worker_opens_only_the_bound_fel_endpoint`
17. `[PASS] worker_requires_location_to_match_current_bus_and_port`
18. `[PASS] chunk_addresses_add_each_offset_exactly_once`
19. `[PASS] entry_must_be_one_declared_load_address`
20. `[PASS] helper_rejects_dram_bootloader_without_verified_spl_before_device_open`
21. `[PASS] helper_rejects_low_and_overlapping_load_addresses`
22. `[PASS] mainline_spl_requires_exact_egon_spl_file_and_checksum`
23. `[PASS] r528_bootstrap_disables_icache_swaps_sram_and_requires_fel_return`
24. `[PASS] unsupported_soc_is_rejected_before_any_sram_write`
25. `[PASS] validated_plan_requires_one_spl_and_one_bootloader`
26. `[PASS] validated_snapshot_is_immutable_from_later_source_changes_and_is_cleaned_up`

Doc tests: 0 tests, 0 failures.

## Additional gates

27. `[PASS] rustfmt --check on mainline-touched Rust files`
28. `[PASS] cargo check --all-targets`
29. `[PASS] cargo clippy --all-targets -- -D warnings`
30. `[PASS] cargo build --release`
31. `[PASS] release binary exposes boot-mainline`
32. `[PASS] boot-mainline exposes exact device-location binding`
33. `[PASS] boot-mainline exposes text/jsonl output selection`
34. `[PASS] src/flash/mod.rs remains byte-for-byte unchanged from the base branch`

Whole-tree `cargo fmt --check` is not used as this feature gate because the
upstream baseline `src/flash/mod.rs` is already unformatted. Formatting that
file would violate this release's explicit requirement to leave the frozen FES
implementation byte-for-byte unchanged.

Run all executable gates with:

```sh
./scripts/validate-mainline.sh
```

## Hardware boundary

These gates validate parsing, bounds, hashing, endpoint selection, SRAM thunk
construction and build quality without a connected board. The current clean
T113 artifact set still requires a new physical FEL load, NAND readback and
power-cycle boot. No automatic retry is allowed after a terminal FEL failure.
