#!/usr/bin/env bash
# release.d hook: create a git tag for the release.
#
# Exports from release.sh: $version
#
set -euo pipefail

if ! git rev-parse "v$version" >/dev/null 2>&1; then
    git tag -m "Release v$version" "v$version"
    echo "Tagged v$version"
fi

if ! git rev-parse "$version" >/dev/null 2>&1; then
    git tag -m "Release $version" "$version"
    echo "Tagged $version"
fi
echo "Tagged release — run 'git push && git push --tags' to publish"
