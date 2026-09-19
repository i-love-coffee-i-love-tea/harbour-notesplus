#!/usr/bin/env bash
# release.d hook: run cargo tests before stamping a release.
#
# Exports from release.sh: (none needed)
#
set -euo pipefail

cargo test -p notesplusplus-core --quiet
echo "All tests passed"
