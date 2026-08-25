# Reproducibility

## Pinned inputs

`manifests/sources.lock` pins the Buildroot Git commit and SHA-256 values of the
Linux and U-Boot archives. Buildroot is configured with forced download hash
checking.

## Clean-room procedure

```sh
git clone https://github.com/100askTeam/OpenixCLI.git
git clone https://github.com/dshanpi/DshanPI-T113xMainlineLinux.git

cd OpenixCLI
cargo test
cargo build --release
export PATH="$PWD/target/release:$PATH"

cd ../DshanPI-T113xMainlineLinux
make all
openixcli scan
./scripts/flash-mainline-fel.sh ./out/t113s3pro-mainline-fel libusb:BUS:PORT
```

After flashing, remove power for at least one second and capture UART from the
first SPL byte through the login prompt. Compare the required markers listed in
`logs/README.md`.

Generated images are release artifacts, not source files. Each published
bundle must carry `FEL_SHA256SUMS`, `FEL_ARTIFACTS`, the OpenixCLI plan and a
top-level `SHA256SUMS` file.
