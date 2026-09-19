#!/usr/bin/env bash
# release.d hook: validate all version-stamped files exist.
#
# Exports from release.sh: $cargo_toml $spec $appdata $changes $qml_about
#
set -euo pipefail

for f in "$cargo_toml" "$spec" "$appdata" "$changes" "$qml_about" "CHANGELOG.md"; do
    if [ ! -f "$f" ]; then
        echo "error: $f not found"
        exit 1
    fi
done

echo "All release files present"
