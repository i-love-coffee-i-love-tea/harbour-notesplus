#!/usr/bin/env bash
# release.d hook: add a new changelog entry for this version at the top.
#
# Exports from release.sh: $version $changes
#
set -euo pipefail

today=$(date +"%a %b %d %Y")
header="$today gobuki <github.com@gobuki.org> $version-1"

# Skip if this version already has an entry
if grep -q "$version" "$changes"; then
    echo "Changelog entry for $version already exists in $changes, skipping"
    exit 0
fi

tmpfile=$(mktemp)
{
    echo "# harbour-notesplus changelog"
    echo ""
    echo "* $header"
    echo "- (describe changes here)"
    echo ""
    # Append everything after the first header block
    tail -n +3 "$changes"
} > "$tmpfile"
mv "$tmpfile" "$changes"

echo "Added changelog entry for $version to $changes — edit the description before committing"
