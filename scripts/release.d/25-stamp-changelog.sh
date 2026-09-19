#!/usr/bin/env bash
# release.d hook: convert "## Unreleased" in CHANGELOG.md to
# "## <version> (<date>)".
#
# Exports from release.sh: $version
#
set -euo pipefail

changelog="CHANGELOG.md"

if [ ! -f "$changelog" ]; then
    echo "Warning: $changelog not found, skipping"
    exit 0
fi

if ! grep -q '^## Unreleased' "$changelog"; then
    echo "No '## Unreleased' section in $changelog, skipping"
    exit 0
fi

if grep -q "^## $version" "$changelog"; then
    echo "Version $version already in $changelog, skipping stamping"
    exit 0
fi

today=$(LC_ALL=C date +%Y-%m-%d)

sed -i '/^## Unreleased$/c\## '"$version"' ('"$today"')' "$changelog"

echo "Stamped $version in $changelog"
