#!/usr/bin/env bash
# release.d hook: derive rpm/harbour-notesplus.changes from CHANGELOG.md.
#
# Converts the latest version section from CHANGELOG.md into the RPM
# changelog format used by the Sailfish SDK.
#
# Exports from release.sh: $version $changes
#
set -euo pipefail

changelog="CHANGELOG.md"

if [ ! -f "$changelog" ]; then
    echo "Warning: $changelog not found, skipping"
    exit 0
fi

if [ ! -f "$changes" ]; then
    echo "Warning: $changes not found, skipping"
    exit 0
fi

# Extract the latest version section (everything between "## <version>" and the next "## ")
# Then convert markdown bullets to RPM changelog bullets
header=$(LC_ALL=C date +"%a %b %d %Y")
author="gobuki <github.com@gobuki.org>"

tmpfile=$(mktemp)
{
    echo "# harbour-notesplus changelog"
    echo ""
    echo "* $header $author $version-1"
    # Extract lines under the latest version heading, skip blank lines and ### subheadings
    sed -n "/^## $version/,/^## /{/^## $version/d;/^## /d;/^$/d;/^### /d;s/^- /- /p;}" "$changelog"
    echo ""
    # Append old entries (everything from the second "## " onwards, preserving existing format)
    sed -n '/^## [0-9]/,$ p' "$changes" | tail -n +1
} > "$tmpfile"

# Only overwrite if we actually extracted something
if grep -q '^- ' "$tmpfile"; then
    mv "$tmpfile" "$changes"
    echo "Generated $changes from $changelog"
else
    rm -f "$tmpfile"
    echo "Warning: no bullet items found for $version in $changelog, $changes unchanged"
fi
