#!/usr/bin/env bash
# release.d hook: stamp version in Cargo.toml workspace.package.
#
# Exports from release.sh: $version $cargo_toml
#
set -euo pipefail

sed -i "s/^version = \".*\"/version = \"$version\"/" "$cargo_toml"
echo "Stamped $version in $cargo_toml"
