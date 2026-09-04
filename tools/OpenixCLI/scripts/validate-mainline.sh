#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "${ROOT}"

echo "[GATE] cargo test"
cargo test --locked
echo "[GATE] rustfmt --check on mainline-touched Rust files"
rustfmt --edition 2021 --check src/commands/mainline.rs src/commands/scan.rs src/cli.rs
rustfmt --edition 2021 --check --config skip_children=true \
	src/commands/mod.rs src/main.rs
echo "[GATE] cargo check --all-targets"
cargo check --locked --all-targets
echo "[GATE] cargo clippy --all-targets -- -D warnings"
cargo clippy --locked --all-targets -- -D warnings
echo "[GATE] cargo build --release"
cargo build --release --locked
echo "[GATE] boot-mainline help surface"
./target/release/openixcli boot-mainline --help | grep -F -- "--device-location" >/dev/null
./target/release/openixcli boot-mainline --help | grep -F -- "--output <OUTPUT>" >/dev/null
grep -F 'rev = "3752e38ff8e69190c53cd43290a8102beab55e73"' Cargo.toml >/dev/null
grep -F '"phase":"ram_handoff_complete"' src/commands/mainline.rs >/dev/null
grep -F '"installerStatus":"not_observed"' src/commands/mainline.rs >/dev/null
echo "[GATE] existing vendor flash implementation remains unchanged"
git diff --quiet ea0e305b4192df208f0c6fef2364207fbd63a857 -- src/flash/mod.rs
echo "[PASS] OpenixCLI mainline local validation"
