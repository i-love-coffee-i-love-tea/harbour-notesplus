#!/usr/bin/env bash
# release.d hook: stamp version in QML about page text.
#
# Exports from release.sh: $version $qml_about
#
set -euo pipefail

sed -i "s/Notes Plus v[0-9]\+\.[0-9]\+\.[0-9]\+/Notes Plus v$version/" "$qml_about"
echo "Stamped v$version in $qml_about"
