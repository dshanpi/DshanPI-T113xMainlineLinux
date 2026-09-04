PYTHON ?= python3

.PHONY: check test dist clean

check: test
	$(PYTHON) tools/allwinner_loader.py validate-all
	$(PYTHON) tools/allwinner_loader.py build-all --output-dir dist --check-reproducible

test:
	$(PYTHON) -m unittest discover -s tests -v

dist:
	$(PYTHON) tools/allwinner_loader.py build-all --output-dir dist --clean --check-reproducible

clean:
	rm -rf dist

