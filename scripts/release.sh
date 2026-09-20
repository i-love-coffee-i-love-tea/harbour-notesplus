#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

dry_run=false
no_tag=false
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) dry_run=true; shift ;;
        --no-tag)  no_tag=true; shift ;;
        *) break ;;
    esac
done

if [ $# -ne 1 ]; then
    echo "usage: $0 [--dry-run] [--no-tag] <version>"
    echo "  e.g. $0 0.2.0"
    echo "       $0 --dry-run 0.2.0"
    echo "       $0 --no-tag 0.2.0"
    exit 1
fi

cd "$PROJECT_ROOT"

version="$1"

# Validate semver-ish format
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: version must be X.Y.Z (got '$version')"
    exit 1
fi

cargo_toml="Cargo.toml"
spec="rpm/harbour-notesplus.spec"
appdata="rpm/harbour-notesplus.appdata.xml"
changes="rpm/harbour-notesplus.changes"
qml_about="notesplus-sailfish/qml/pages/AboutPage.qml"
hook_dir="$SCRIPT_DIR/release.d"

export version cargo_toml spec appdata changes qml_about dry_run no_tag

if $dry_run; then
    echo "[dry-run] Running validation hooks..."
    errors=0
    for hook in "$hook_dir"/1[0-9]-*.sh; do
        [ -x "$hook" ] || continue
        name=$(basename "$hook")
        output=$("$hook" 2>&1) && rc=0 || rc=$?
        if [ $rc -eq 0 ]; then
            printf "  %-40s OK\n" "$name"
        else
            printf "  %-40s FAIL\n" "$name"
            echo "$output" | sed 's/^/    /'
            errors=$((errors + 1))
        fi
    done
    echo ""
    if [ $errors -gt 0 ]; then
        echo "[dry-run] $errors error(s) found. Fix the issues above and re-run."
        exit 1
    fi
    echo "[dry-run] 0 error(s) found."
    exit 0
fi

for hook in "$hook_dir"/*.sh; do
    [ -x "$hook" ] || continue
    name=$(basename "$hook")
    if $no_tag && [[ "$name" == 9[0-9]-* ]]; then
        echo "Skipping hook: $name (--no-tag)"
        continue
    fi
    echo "Running hook: $name"
    "$hook"
done
