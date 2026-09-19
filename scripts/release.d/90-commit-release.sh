#!/usr/bin/env bash
# release.d hook: stage and commit the release files.
#
# Exports from release.sh: $version $cargo_toml $spec $appdata $changes $qml_about
#
set -euo pipefail

git add "$cargo_toml" "$spec" "$appdata" "$changes" "$qml_about" "CHANGELOG.md"

# Also stage Cargo.lock if it changed (cargo update on version bump)
git add Cargo.lock 2>/dev/null || true

if git diff --cached --quiet; then
    echo "Release commit already up to date for $version"
    exit 0
fi

git commit -m "chore(release): bump version to $version" --trailer "Co-authored-by: Junie <junie@jetbrains.com>"
