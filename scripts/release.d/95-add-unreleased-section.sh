#!/usr/bin/env bash
# release.d hook: add an empty "## Unreleased" section to CHANGELOG.md
# after tagging. Prepares the changelog for the next development cycle.
#
# Exports from release.sh: $version
#
set -euo pipefail

changelog="CHANGELOG.md"

if [ ! -f "$changelog" ]; then
    echo "Warning: $changelog not found, skipping"
    exit 0
fi

if grep -q '^## Unreleased' "$changelog"; then
    echo "'## Unreleased' already in $changelog, skipping"
    exit 0
fi

tmpfile=$(mktemp)
awk '/^# Changelog$/ { print; print ""; print "## Unreleased"; next } { print }' "$changelog" > "$tmpfile"
mv "$tmpfile" "$changelog"

echo "Added '## Unreleased' to $changelog for next development cycle"
