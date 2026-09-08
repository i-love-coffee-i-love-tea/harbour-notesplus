# ADR-001: Separation of Pure Rust Core Engine and Sailfish Silica UI Bridge

#### Status
Accepted

#### Context
Notes++ targets Sailfish OS devices using Qt Quick / QML (Silica), but also incorporates an embedded companion HTTP server for desktop browsers, command-line export tools, and extensive automated test suites. 

Tightly coupling the document engine to Qt/C++ or QObject runtimes would:
1. Impose severe testing friction, requiring emulator or on-device execution for basic parser and search tests.
2. Complicate embedding the web companion server and headless document exporter.
3. Obstruct code reuse across platforms or alternative frontends.

#### Decision
Split the codebase into two distinct decoupled crates:
1. **`notesplusplus-core`**: A pure Rust library with zero Qt/QML dependencies. It encapsulates the AsciiDoc AST parser, preprocessor, HTML/CSS exporter, SQLite FTS5 search index, embedded HTTP web server, and LLM agent client.
2. **`notesplusplus-sailfish`**: The platform-specific Sailfish OS application providing QML views, Silica delegates, and a C++/Rust FFI bridge (`Sailors`/QObject) to expose core capabilities to the mobile UI.

```
┌────────────────────────────────────────────────────────┐
│               Sailfish Silica UI (QML)                 │
└───────────────────────────┬────────────────────────────┘
                            │ QObject Properties / Slots
┌───────────────────────────▼────────────────────────────┐
│         notesplusplus-sailfish (Bridge / FFI)          │
└───────────────────────────┬────────────────────────────┘
                            │ Direct Rust API Calls
┌───────────────────────────▼────────────────────────────┐
│                  notesplusplus-core                    │
│  (Parser, AST, FTS5 Search, Web Server, AI Client)     │
└────────────────────────────────────────────────────────┘
```

#### Consequences
##### Positive / Utility Delivered
- **Rapid Host Testing**: Core logic, parsing edge cases, search indexing, and HTTP endpoints are tested natively on the developer workstation via `cargo test -p notesplusplus-core` in seconds without running the Sailfish SDK or emulator.
- **Clean Separation of Concerns**: UI lifecycle events do not block backend tasks; file I/O, parsing, and search run on dedicated background threads.
- **Headless Portability**: `notesplusplus-core` can be compiled and reused as a CLI tool, background service, or web server across Linux, macOS, and Windows.

##### Trade-offs / Mitigations
- Data exchanged across the QML/Rust boundary requires structured serialization (JSON strings or scalar properties) via the FFI bridge.
