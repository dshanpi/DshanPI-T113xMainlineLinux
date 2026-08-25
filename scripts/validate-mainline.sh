#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "${ROOT}"

echo "[GATE] cargo test"
cargo test
echo "[GATE] rustfmt --check on mainline-touched Rust files"
rustfmt --edition 2021 --check src/commands/mainline.rs src/cli.rs
rustfmt --edition 2021 --check --config skip_children=true \
	src/commands/mod.rs src/main.rs
echo "[GATE] cargo check --all-targets"
cargo check --all-targets
echo "[GATE] cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings
echo "[GATE] cargo build --release"
cargo build --release
echo "[GATE] boot-mainline help surface"
./target/release/openixcli boot-mainline --help | grep -F -- "--device-location" >/dev/null
./target/release/openixcli boot-mainline --help | grep -F -- "--output <OUTPUT>" >/dev/null
echo "[GATE] existing vendor flash implementation remains unchanged"
git diff --quiet ea0e305b4192df208f0c6fef2364207fbd63a857 -- src/flash/mod.rs
echo "[PASS] OpenixCLI mainline local validation"
