#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check if node is available with @vue/compiler-dom
if command -v node >/dev/null 2>&1; then
  if node -e "require('@vue/compiler-dom')" >/dev/null 2>&1; then
    exec node "$SCRIPT_DIR/compile-templates.mjs" "$@"
  fi
fi

# If npx and npm are available
if command -v npm >/dev/null 2>&1; then
  TMP_NPM_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'npm-vue')"
  cleanup_npm() {
    rm -rf "$TMP_NPM_DIR"
  }
  trap cleanup_npm EXIT
  if npm --prefix "$TMP_NPM_DIR" install --no-save @vue/compiler-dom@3.4.21 >/dev/null 2>&1; then
    NODE_PATH="$TMP_NPM_DIR/node_modules" exec node "$SCRIPT_DIR/compile-templates.mjs" "$@"
  fi
  trap - EXIT
  rm -rf "$TMP_NPM_DIR"
fi

# Check if docker is available
if command -v docker >/dev/null 2>&1; then
  exec docker run --rm -v "$REPO_ROOT":/workspace -w /workspace node:alpine sh -c '
    npm --prefix /tmp install --no-save @vue/compiler-dom@3.4.21 >/dev/null 2>&1
    NODE_PATH=/tmp/node_modules node /workspace/scripts/compile-templates.mjs "$@"
  ' sh "$@"
fi

# Check if podman is available
if command -v podman >/dev/null 2>&1; then
  exec podman run --rm -v "$REPO_ROOT":/workspace:z -w /workspace node:alpine sh -c '
    npm --prefix /tmp install --no-save @vue/compiler-dom@3.4.21 >/dev/null 2>&1
    NODE_PATH=/tmp/node_modules node /workspace/scripts/compile-templates.mjs "$@"
  ' sh "$@"
fi

echo "Error: Neither Node.js (with @vue/compiler-dom) nor Docker/Podman is available." >&2
echo "Please install Node.js and @vue/compiler-dom, or install Docker/Podman to compile Vue templates." >&2
exit 1
