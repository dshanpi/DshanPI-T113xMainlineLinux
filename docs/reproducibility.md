# Reproducibility

## Pinned inputs

`manifests/sources.lock` pins the Buildroot Git commit and SHA-256 values of the
Linux and U-Boot archives. Buildroot is configured with forced download hash
checking.

## Clean-room procedure

```sh
git clone -b feat/mainline-fel-ram-installer \
  https://github.com/100askTeam/OpenixCLI.git
git clone -b feat/t113s3pro-mainline \
  https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git

cd DshanPI-T113xMainlineLinux
./scripts/build-everything.sh
OPENIXCLI_BIN=../OpenixCLI/target/release/openixcli \
  ./scripts/flash-and-monitor.sh \
  ./out/t113s3pro-mainline-fel auto /dev/ttyACM0 300
```

After flashing, remove power for at least one second and capture UART from the
first SPL byte through the login prompt. Compare the required markers listed in
`logs/README.md`.

Generated images are release artifacts, not source files. Each published
bundle must carry `FEL_SHA256SUMS`, `FEL_ARTIFACTS`, the OpenixCLI plan and a
top-level `SHA256SUMS` file.

Historical manifests under `manifests/` identify evidence from a specific past
build; they are not expected to match a later build containing new FIT/UBI
metadata. Current output is verified against the `FEL_SHA256SUMS` generated in
that same build. Hardware promotion records the new hashes only after the full
installer and cold-boot gate passes.

The build helper uses `cargo test --locked` and `cargo build --release --locked`.
The flash helper rejects zero or multiple Allwinner USB candidates, and the UART
monitor requires the board-side `installer_complete` marker before accepting a
later login prompt. A successful OpenixCLI RAM handoff alone is never classified
as NAND installation success.
