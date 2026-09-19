#!/usr/bin/env bash
# release.d hook: validate that precompiled web templates are up to date.
#
# Ensures notesplus-core/assets/web/template.js matches notesplus-core/assets/web/index.html.
#
# Exit 0 if up to date, exit 1 if stale.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Validating precompiled Vue templates freshness..."
exec "$REPO_ROOT/scripts/compile-templates.sh" --check
