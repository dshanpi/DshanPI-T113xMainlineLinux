.PHONY: bootstrap build package all

bootstrap:
	./scripts/bootstrap-buildroot.sh

build:
	./scripts/build.sh

package:
	./scripts/package-mainline-fel.sh

all:
	./scripts/build-and-package.sh
