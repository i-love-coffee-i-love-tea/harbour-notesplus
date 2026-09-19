#!/usr/bin/env bash
# release.d hook: run cargo tests before stamping a release.
#
# Exports from release.sh: (none needed)
#
set -euo pipefail

cargo test -p notesplus-core --quiet
sed -i 's/^version = 4$/version = 3/' Cargo.lock
echo "All tests passed"
