# Developer Guide: Build, Test & Deployment

This document describes the development, build, test, and deployment workflows for **Fishdoc (Notes++)** on Sailfish OS and embedded environments.

---

## 1. Architecture Overview

- **`fishdoc-core`**: Pure Rust core engine containing:
  - AsciiDoc AST parser (`parser/`, `block.rs`, `inline.rs`, `html.rs`).
  - SQLite database, full-text search (FTS), page indexing (`db/`, `search.rs`, `page.rs`).
  - Embedded HTTP/HTTPS web server with TLS and self-signed certificate generation (`server.rs`, `server/tls.rs`).
  - Web Access Authentication: HTTP Basic Auth + OpenID Connect (OIDC) / OAuth 2.0 PKCE (`server/auth.rs`).
  - Static Vue 3 ES Modules web editor & preview (`server/web_assets.rs`).
  - LLM AI Assistant client & agent tool execution loop (`agent/`).
- **`fishdoc-sailfish`**: Sailfish OS Silica QML GUI application and Qt bridge bindings (`src/bridge/`, `qml/`).

---

## 2. Fast Development & Testing Loop

### Running Unit & Integration Tests (Host)
All parser logic, agent execution, TLS certificate generation, server lifecycle, and authentication handlers are tested natively without requiring the Sailfish OS SDK:

```bash
cargo test -p fishdoc-core
```

To run a specific test suite:
```bash
cargo test -p fishdoc-core -- server::tests::test_server_authentication_basic_and_session
```

---

## 3. Building for Sailfish OS (`build-sailfish.sh`)

Sailfish OS binaries are cross-compiled for `aarch64` using the `sfdk` Sailfish SDK Docker container (`sailfish-sdk-build-engine_gobuki`).

### Build Command:
```bash
./build-sailfish.sh
```

### What `build-sailfish.sh` does automatically:
1. **Clean Stale Locks & Daemons**:
   - Terminates orphaned `cargo`, `rpmbuild`, or `sb2d` daemons inside the SDK build container.
   - Clears `.cargo-lock`, `.package-cache`, and `.mb2.lock` files across the build workspace.
2. **Syncs Source**:
   - Syncs project files to `~/SailfishWorkspace/` while excluding host build artifacts (`target/`, `.git/`, `rpms/`).
3. **Rust 1.75 Format Conversion**:
   - Automatically converts `Cargo.lock` version format from v4 (host) to v3 (SDK's Rust 1.75).
4. **Executes `sfdk build`**:
   - Runs `sfdk -c target=SailfishOS-5.1.0.11-aarch64 build` in the container.
5. **Copies Artifacts**:
   - Places the compiled `aarch64` binary in `target/aarch64-unknown-linux-gnu/release/` and output RPM in `rpms/`.

---

## 4. Deploying to Device (`deploy-sailfish.sh`)

### Deployment Command:
```bash
./deploy-sailfish.sh
```

### What `deploy-sailfish.sh` does:
1. Finds the latest `.aarch64.rpm` from `rpms/`.
2. Stops any running instance of `harbour-fishdoc` on the target device via SSH.
3. Transfers the RPM to `/home/defaultuser/Downloads/` on `phone-wifi`.
4. Triggers the Sailfish OS package installation notification via D-Bus (`org.sailfishos.installation.prompt`).

---

## 5. Troubleshooting Common Build & Lock Issues

### Problem 1: Build hangs indefinitely on `cargo build` or `flock`
- **Root Cause**: When a previous build is aborted (e.g., Ctrl+C or terminal timeout), Cargo or `sb2d` leaves `.cargo-lock` files in `target/release/` or `.mb2.lock` in `/home/mersdk/`.
- **How to fix manually**:
  ```bash
  # 1. Kill any background container processes and clean locks:
  docker exec sailfish-sdk-build-engine_gobuki bash -c '
      pkill -u mersdk -9 || true
      rm -f /home/mersdk/.mb2.lock* /home/mersdk/.cargo/.package-cache 2>/dev/null || true
      rm -rf /tmp/sb2-* 2>/dev/null || true
  '

  # 2. Delete workspace lock files:
  find ~/SailfishWorkspace/ -name '.cargo-lock' -delete
  find ~/SailfishWorkspace/ -name '.package-cache' -delete

  # 3. Rerun build:
  ./build-sailfish.sh
  ```
  *(Note: `./build-sailfish.sh` now runs this cleanup automatically before every build).*

### Problem 2: `feature edition2024 is required` or `requires rustc 1.81 or newer`
- **Root Cause**: The Sailfish OS SDK toolchain currently uses **Rust 1.75**. If a dependency update in `Cargo.lock` pulls a crate that requires edition 2024 (Rust 1.85+) or MSRV > 1.75 (such as `ed25519-dalek 2.2.0` or `base64ct 1.8.3`), compilation fails.
- **How to fix**:
  Downgrade the offending crate in `Cargo.lock` using `cargo update -p <crate> --precise <version>`:
  - `ed25519-dalek`: Pin to `2.1.1`
  - `base64ct`: Pin to `1.6.0`
  - `indexmap`: Pin to `2.6.0`
  - `hashbrown`: Pin to `0.15.5`
  - `serde_with`: Pin to `3.8.0`
  - `time`: Pin to `0.3.36`

### Problem 3: `Cargo.lock` syntax error in SDK
- **Root Cause**: Modern host Cargo (1.85+) generates `Cargo.lock` with `version = 4`. Rust 1.75 in the Sailfish SDK only supports `version = 3`.
- **How to fix**:
  `./build-sailfish.sh` automatically converts `version = 4` to `version = 3` when syncing to `~/SailfishWorkspace/`.
