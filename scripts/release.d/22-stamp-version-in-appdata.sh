#!/usr/bin/env bash
# release.d hook: stamp version and today's date in AppData XML release element.
#
# Exports from release.sh: $version $appdata
#
set -euo pipefail

today=$(date +%Y-%m-%d)
sed -i "s|<release version=\"[^\"]*\" date=\"[^\"]*\">|<release version=\"$version\" date=\"$today\">|" "$appdata"
echo "Stamped version=$version date=$today in $appdata"
