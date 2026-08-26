.PHONY: bootstrap build package validate all fes-preflight

bootstrap:
	./scripts/bootstrap-buildroot.sh

build:
	./scripts/build.sh

package:
	./scripts/package-mainline-fel.sh

validate:
	./scripts/validate-local.py

all:
	./scripts/build-and-package.sh
	./scripts/validate-local.py

# FES media writes are deliberately not a Make target. This gate opens no USB.
fes-preflight:
	@test -n "$(FES_BUNDLE)" || (echo "set FES_BUNDLE=/path/to/bundle" >&2; exit 2)
	@test -n "$(OPENIXCLI_BIN)" || (echo "set OPENIXCLI_BIN=/path/to/openixcli" >&2; exit 2)
	$(OPENIXCLI_BIN) --output jsonl flash-nand-components --preflight-only \
		--manifest $(FES_BUNDLE)/manifest.json --device-location libusb:0:0 \
		--bus 0 --port 0 --mode full_erase --post-action none
