#!/usr/bin/env bash
# release.d hook: stamp version and release in RPM spec.
#
# Exports from release.sh: $version $spec
#
set -euo pipefail

sed -i "s/^Version:.*/Version:    $version/" "$spec"
sed -i "s/^Release:.*/Release:    1/" "$spec"
echo "Stamped Version=$version Release=1 in $spec"
