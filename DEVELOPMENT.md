# Developer Guide: Build, Test & Deployment

This document describes the development, build, test, and deployment workflows for **Notes Plus** on Sailfish OS and embedded environments.

---

## 1. Architecture Overview

- **`notesplus-core`**: Pure Rust core engine containing:
  - AsciiDoc AST parser (`parser/`, `block.rs`, `inline.rs`, `html.rs`).
  - SQLite database, full-text search (FTS), page indexing (`db/`, `search.rs`, `page.rs`).
  - Embedded HTTP/HTTPS web server with TLS and self-signed certificate generation (`server/mod.rs`, `server/tls.rs`).
  - Web Access Authentication: Verification code challenge with approve/deny on device (`server/auth.rs`).
  - Static Vue 3 ES Modules web editor & preview (`server/web_assets.rs`).
  - LLM AI Assistant client & agent tool execution loop (`agent/`).
- **`notesplus-sailfish`**: Sailfish OS Silica QML GUI application and Qt bridge bindings (`src/bridge/`, `qml/`).

---

## 2. Fast Development & Testing Loop

### Running Unit & Integration Tests (Host)
All parser logic, agent execution, TLS certificate generation, server lifecycle, and authentication handlers are tested natively without requiring the Sailfish OS SDK:

```bash
cargo test -p notesplus-core
```

To run a specific test suite:
```bash
cargo test -p notesplus-core -- server::tests::test_server_authentication_basic_and_session
```

---

## 3. Building for Sailfish OS (`build-sailfish.sh`)

Sailfish OS binaries are cross-compiled using the `sfdk` Sailfish SDK Docker container (`sailfish-sdk-build-engine_gobuki`). The default target is `aarch64`; use `SAILFISH_TARGET` to select another architecture.

### Build Command:
```bash
./build-sailfish.sh

# armv7hl (Xperia XA2, Xperia 10, community ports):
SAILFISH_TARGET=SailfishOS-5.1.0.11-armv7hl ./build-sailfish.sh

# i486 (emulator):
SAILFISH_TARGET=SailfishOS-5.1.0.11-i486 ./build-sailfish.sh
```

### What `build-sailfish.sh` does automatically:
1. **Cleans Local & Engine Locks**:
   - Cleans local project locks (`.cargo-lock`, `.package-cache`, `.cargo-build-lock`).
   - Safely clears stale `/tmp/sb2-*` and `/home/mersdk/.mb2.lock` in the SDK container only when no active build process (`cargo`, `qmake`, `make`) is running.
2. **Rust 1.75 Lockfile Compatibility**:
   - Ensures `Cargo.lock` uses `version = 3` (supported by the SDK's Rust 1.75).
3. **Executes `sfdk build` Directly In-Tree**:
   - Runs `sfdk -c target=$SAILFISH_TARGET build` directly inside the project root directory (default: `SailfishOS-5.1.0.11-aarch64`).
4. **Copies Artifacts**:
   - Places the compiled binary and generated RPM in `rpms/`.

---

## 4. Parallel Multi-Project Builds

Multiple Sailfish OS projects (e.g. `harbour-notesplus`, `harbour-retrofit`, `harbour-netpwrctrl`) can build concurrently using the same Sailfish SDK build engine container (`sailfish-sdk-build-engine_gobuki`):

### How Isolation Works
* **In-Tree Project Workspaces**: Each project builds directly in its own repository directory. There is no shared sync workspace (such as `~/SailfishWorkspace`), eliminating workspace overwrite collisions (`rsync --delete`).
* **Isolated Intermediate Artifacts**: Build caches (`target/`, Makefiles, object files, `.sfdk/`, `RPMS/`, `rpms/`) remain strictly local to each project directory.
* **Non-Destructive Lock Cleanup**: The build scripts verify active container processes (`pgrep -u mersdk`) before cleaning engine `/tmp` directories or locks, preventing one build from interrupting another.

### Container-Level Shared Resources & Concurrency Behavior
* **`mb2` Global Locking (`/home/mersdk/.mb2.lock`)**:
  - The underlying `mb2` build tool uses Linux `flock` during critical phases (such as RPM spec evaluation, dependency checking, and the `%install` phase).
  - When two builds reach an `mb2` critical section simultaneously, the second build automatically waits on the file lock for a few seconds and resumes once the first leaves the critical section.
* **Cargo Shared Package Cache (`/home/mersdk/.cargo/`)**:
  - Cargo uses internal file locks on its global registry and git checkouts. Concurrent crate downloads or index updates are safely serialized by Cargo.
* **Scratchbox2 Target Rootfs Snapshots**:
  - Projects build against the default target snapshot (`SailfishOS-5.1.0.11-aarch64.default`) concurrently without conflict because standard builds only read target libraries without modifying system packages in the rootfs.
  - *Optional*: For full target rootfs clone isolation, `sfdk -S %pool build` can be used to dynamically allocate an isolated snapshot clone from the snapshot pool.

---

## 5. Deploying to Device (`deploy-sailfish.sh`)

### Deployment Command:
```bash
./deploy-sailfish.sh
```

### What `deploy-sailfish.sh` does:
1. Finds the latest `.aarch64.rpm` from `rpms/`.
2. Stops any running instance of `harbour-notesplus` on the target device via SSH.
3. Transfers the RPM to `/home/defaultuser/Downloads/` on `phone-wifi`.
4. Triggers the Sailfish OS package installation notification via D-Bus (`org.sailfishos.installation.prompt`).

---

## 6. GitHub CI/CD & OpenRepos Release Pipelines

The repository includes automated GitHub Actions workflows under `.github/workflows/`:

1. **`build.yml` (Continuous Integration)**:
   - Triggers on pull requests and pushes to `main`/`master`.
   - Runs `cargo test` across all crates.
   - Builds RPM packages for target architectures (`aarch64`, `armv7hl`, `i486`).
   - Uploads RPM packages as workflow build artifacts.

2. **`release.yml` (Release & OpenRepos Publishing)**:
   - Triggers on version tags (e.g. `v0.1.0`) or manual workflow dispatch.
   - Builds RPMs for all target architectures.
   - Creates a GitHub Release with attached `.rpm` files and SHA256 checksums.
   - Publishes the RPM packages to [OpenRepos.net](https://openrepos.net) using repository secrets:
     - `OPENREPOS_USERNAME`: OpenRepos account username
     - `OPENREPOS_PASSWORD`: OpenRepos account password

---

## 7. Troubleshooting Common Build & Lock Issues

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

  # 2. Delete local project lock files:
  find . -name '.cargo-lock' -delete
  find . -name '.package-cache' -delete

  # 3. Rerun build:
  ./build-sailfish.sh
  ```
  *(Note: `./build-sailfish.sh` now runs safe lock cleanup automatically before every build).*

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
  `./build-sailfish.sh` automatically ensures `Cargo.lock` uses `version = 3` before building.

### Problem 4: `GLIBC_2.33 not found` when building with older SDK targets
- **Root Cause**: The `SailfishOS-4.6.0.13` tooling ships glibc 2.30. Rust's `serde_derive` proc-macro binary (and any proc-macro crate compiled inside sb2) gets linked against the tooling's glibc, which produces a `.so` requiring GLIBC 2.33+. The `SailfishOS-5.1.0.11` tooling ships glibc 2.41 which is sufficient.
- **Scope**: This is NOT a dependency version issue — it affects ANY Rust project using proc-macros (serde, syn, etc.) on the 4.6 SDK. Downgrading crate versions does not help.
- **Status**: Resolved by using `SailfishOS-5.1.0.11` targets (aarch64, armv7hl, i486).

### Problem 5: `can't find crate for zerofrom_derive` / ICU dependency chain
- **Root Cause**: `url` >=2.5.3 pulls in `idna` >=1.0 which depends on the full ICU4C Unicode normalization stack (`icu_collections`, `icu_normalizer`, `displaydoc`, `zerofrom`). These proc-macro crates trigger sb2 cross-compilation bugs.
- **Fix**: Downgrade `url` to 2.5.2: `cargo update -p url --precise 2.5.2`. This replaces `idna 1.x` (ICU-based) with `idna 0.5` (pure Rust), removing the entire ICU chain.
- **Note**: This fix alone does not resolve Problem 4 — the serde glibc issue remains.

---

## 8. Device Compatibility

### Build targets

RPMs are built with `SailfishOS-5.1.0.11` for three architectures:

| SDK Target | RPM Arch | Rust Target Triple |
|------------|----------|--------------------|
| `SailfishOS-5.1.0.11-aarch64` | aarch64 | `aarch64-unknown-linux-gnu` |
| `SailfishOS-5.1.0.11-armv7hl` | armv7hl | `armv7-unknown-linux-gnueabihf` |
| `SailfishOS-5.1.0.11-i486` | i486 | `i686-unknown-linux-gnu` |

All SDK 5.1 targets ship glibc 2.41, which is sufficient for Rust proc-macro crates.

### Verified devices

| Device | Arch | Sailfish OS | Status |
|--------|------|-------------|--------|
| Jolla C2 | aarch64 | 5.1+ | Verified |
| Xperia 10 III | aarch64 | 5.1+ | Verified |
| Jolla Phone 2026 | aarch64 | 5.1+ | Verified |
| Xperia 10 II / IV / V | aarch64 | 4.5+ | Compatible (if SFOS 5.1+) |
| Xperia XA2 / XA2 Plus / XA2 Ultra | armv7hl | 4.6+ | Compatible (armv7hl RPM) |
| Xperia 10 / 10 Plus (1st gen) | armv7hl | 3.2+ | Compatible (armv7hl RPM, if SFOS 5.1+) |
| Sailfish OS Emulator | i486 | 5.1+ | Compatible (i486 RPM) |

### Historical blockers (now resolved)

The following blockers previously prevented armv7hl builds with SDK 4.6:

1. **glibc 2.30 too old** — Resolved: SDK 5.1 ships glibc 2.41.
2. **sb2 SIGSEGV in `libsb2.so.1(opendir)`** — Was specific to the 4.6 SDK's sb2 version. SDK 5.1 includes a newer sb2.
3. **`ring` / `cc` crate can't find cross-compiler** — The `cc` crate defaults to looking for `arm-linux-gnueabihf-gcc` / `arm-linux-gnueabihf-g++`, which don't exist in sb2. In sb2, `gcc`/`g++` are transparent cross-compiler wrappers. Fix: `export CC=gcc` and `export CXX=g++` in the spec file's `%build` section.
4. **`__fp16` type unavailable on armv7** — The vendored whisper.cpp uses `__fp16` (a GCC ARM extension) which is not supported by the sb2 armv7 GCC. Fix: `build.rs` defines `__fp16` as `uint16_t` when targeting `arm`.
5. **Stale object files from previous architecture** — In-tree builds leave `.o` files from the previous architecture. Fix: `make distclean` before `qmake` in the spec file.

### Dependency note: `url` crate and ICU chain

The `url` crate >=2.5.3 pulls in `idna` >=1.0 which depends on the full ICU4C Unicode normalization stack (`icu_collections`, `icu_normalizer`, `displaydoc`, `zerofrom`), adding ~20 heavy proc-macro crates. Downgrading to `url 2.5.2` replaces the ICU-based `idna 1.x` with pure-Rust `idna 0.5`, significantly reducing the dependency tree. This does NOT enable 4.6 builds (glibc limitation remains) but reduces build times and binary size.

```bash
cargo update -p url --precise 2.5.2
```

### Building for a specific architecture

The default build target is `aarch64`. To build for another architecture:

```bash
SAILFISH_TARGET=SailfishOS-5.1.0.11-armv7hl ./build-sailfish.sh
SAILFISH_TARGET=SailfishOS-5.1.0.11-i486 ./build-sailfish.sh
```

The CI pipelines build all three architectures automatically on every push and release.
