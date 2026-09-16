#!/bin/bash
# Build harbour-notesplus for Sailfish OS (aarch64) via sfdk build.
#
# Prerequisites (run once):
#   - Sailfish SDK installed with Docker engine
#   - sfdk in PATH (e.g. ~/SailfishOS/bin/sfdk)
#   - Build target set: SailfishOS-5.1.0.11-aarch64

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="${SAILFISH_TARGET:-SailfishOS-5.1.0.11-aarch64}"

echo "=== Cleaning local project locks ==="
find "$SCRIPT_DIR" -name '.cargo-lock' -delete 2>/dev/null || true
find "$SCRIPT_DIR" -name '.cargo-build-lock' -delete 2>/dev/null || true
find "$SCRIPT_DIR" -name '.package-cache' -delete 2>/dev/null || true

# Clear stale SDK engine locks if no active build is running
if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' | grep -q 'sailfish-sdk-build-engine'; then
    docker exec sailfish-sdk-build-engine_gobuki bash -c '
        if ! pgrep -u mersdk cargo >/dev/null 2>&1; then
            rm -f /home/mersdk/.mb2.lock* /home/mersdk/.cargo/.package-cache 2>/dev/null || true
            rm -rf /tmp/sb2-* /tmp/rpm-tmp.* 2>/dev/null || true
        fi
    ' 2>/dev/null || true
fi

# Ensure Cargo.lock format matches Rust 1.75 requirement (v3)
if [ -f "$SCRIPT_DIR/Cargo.lock" ]; then
    sed -i 's/^version = 4$/version = 3/' "$SCRIPT_DIR/Cargo.lock"
fi

echo "=== Building harbour-notesplus directly via sfdk ==="
cd "$SCRIPT_DIR"
sfdk -c target="$TARGET" build

echo "=== Copying RPM to rpms/ ==="
mkdir -p "$SCRIPT_DIR/rpms"
if [ -d "$SCRIPT_DIR/RPMS" ]; then
    if ! cp -u "$SCRIPT_DIR"/RPMS/*.rpm "$SCRIPT_DIR/rpms/" 2>/dev/null; then
        if ! cp "$SCRIPT_DIR"/RPMS/*.rpm "$SCRIPT_DIR/rpms/" 2>/dev/null; then
            echo "ERROR: Failed to copy RPMs from RPMS/ to rpms/" >&2
            exit 1
        fi
    fi
fi

echo "=== Done ==="
if [ -f "$SCRIPT_DIR/target/aarch64-unknown-linux-gnu/release/harbour-notesplus" ]; then
    file "$SCRIPT_DIR/target/aarch64-unknown-linux-gnu/release/harbour-notesplus"
fi
ls -lh "$SCRIPT_DIR/rpms/"*.rpm 2>/dev/null || true
