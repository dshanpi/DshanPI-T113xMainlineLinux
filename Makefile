.PHONY: bootstrap build package validate all

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
