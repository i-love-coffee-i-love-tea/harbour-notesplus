#!/bin/bash
# Build harbour-fishdoc for Sailfish OS (aarch64) via sfdk build.
#
# Prerequisites (run once):
#   - Sailfish SDK installed with Docker engine
#   - sfdk in PATH (e.g. ~/SailfishOS/bin/sfdk)
#   - SailfishWorkspace configured at ~/SailfishWorkspace with .sfdk/ directory
#   - Build target set: SailfishOS-5.1.0.11-aarch64

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE="${SAILFISH_WORKSPACE:-$HOME/SailfishWorkspace}"
TARGET="SailfishOS-5.1.0.11-aarch64"

if [ ! -d "$WORKSPACE/.sfdk" ]; then
    echo "ERROR: SailfishWorkspace not found at $WORKSPACE"
    echo "Set SAILFISH_WORKSPACE or create the workspace first."
    exit 1
fi

echo "=== Syncing source to SailfishWorkspace ==="
rsync -a --delete \
    --exclude='target/' \
    --exclude='.git/' \
    --exclude='rpms/' \
    --exclude='.sfdk/' \
    --exclude='.mimocode/' \
    --exclude='Cargo.lock' \
    "$SCRIPT_DIR/" "$WORKSPACE/"

# Convert Cargo.lock v4 (host) → v3 (SDK's Rust 1.75 can't parse v4)
if [ -f "$SCRIPT_DIR/Cargo.lock" ]; then
    sed 's/^version = 4$/version = 3/' "$SCRIPT_DIR/Cargo.lock" > "$WORKSPACE/Cargo.lock"
fi

echo "=== Building via sfdk build ==="
cd "$WORKSPACE"
sfdk -c target="$TARGET" build

echo "=== Copying binary back ==="
mkdir -p "$SCRIPT_DIR/target/aarch64-unknown-linux-gnu/release"
cp "$WORKSPACE/target/aarch64-unknown-linux-gnu/release/harbour-fishdoc" \
   "$SCRIPT_DIR/target/aarch64-unknown-linux-gnu/release/harbour-fishdoc"

echo "=== Copying RPM ==="
mkdir -p "$SCRIPT_DIR/rpms"
cp "$WORKSPACE"/RPMS/*.rpm "$SCRIPT_DIR/rpms/" 2>/dev/null || true

echo "=== Done ==="
file "$SCRIPT_DIR/target/aarch64-unknown-linux-gnu/release/harbour-fishdoc"
ls -lh "$SCRIPT_DIR/rpms/"*.rpm 2>/dev/null
