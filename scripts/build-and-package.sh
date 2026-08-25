#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
"${ROOT}/scripts/build.sh" "$@"
"${ROOT}/scripts/package-mainline-fel.sh"
