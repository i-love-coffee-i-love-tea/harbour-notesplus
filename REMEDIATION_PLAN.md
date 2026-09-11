# Notes++ Quality & Security Remediation Plan

This remediation plan establishes a prioritized, actionable backlog to address the security vulnerabilities, architectural bottlenecks, code duplications, and robustness issues identified in the **Notes++** codebase (`notesplusplus-core` and `notesplusplus-sailfish`).

The work is organized to maximize net positive impact (utility): resolving the highest-risk threats to user data and device integrity first, followed by high-return stability fixes, performance optimizations, and maintainability enhancements.

---

## Prioritization & Utility Impact Matrix

| Phase | Priority | Focus Area | Net Value / Risk Reduction | Est. Effort |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 1** | **P0 (Critical)** | Core Security & Data Loss Prevention | Eliminates remote unauthorized access and prevents data file truncation | Low–Medium |
| **Phase 2** | **P1 (High)** | Network & Parser Hardening | Mitigates SSRF, Stored XSS, and timing side-channels | Low–Medium |
| **Phase 3** | **P2 (Medium)** | Performance, I/O & Concurrency | Reduces battery drain, file I/O latency, and UI/server thread stalls | Medium |
| **Phase 4** | **P3 (Quality)** | Architecture, Deduplication & FFI | Improves developer velocity, testability, and UI responsiveness | Medium–High |

---

## Phase 1: Critical Security & Data Loss Prevention (P0)

### 1.1 Restrict Authentication Challenge Approval to Direct In-Process UI Calls (Resolved)

- **Target Files:**
  - `notesplusplus-core/src/server/routes/auth.rs` (removed `handle_challenge_approve` and `handle_challenge_deny`)
  - `notesplusplus-core/src/server/routes/mod.rs` (removed `/api/auth/code/approve` and `/api/auth/code/deny` routes)
  - `notesplusplus-sailfish/src/bridge/pages.rs` (`approve_auth_challenge` direct native call)
- **Problem & Impact:**
  The challenge approval endpoint (`/api/auth/code/approve`) was previously reachable over the local Wi-Fi/LAN without authentication. Any LAN client could initiate a challenge and approve it directly via HTTP without local device interaction, obtaining a valid bearer token.
- **Remediation:**
  1. Removed the `/api/auth/code/approve` and `/api/auth/code/deny` HTTP endpoints entirely from the server router and handler modules.
  2. Enforced that challenge approval/denial occurs strictly in-process via direct native calls (`bridge.approve_auth_challenge()`) triggered by physical user interaction on the device UI (`AuthPrompt.qml`).
  3. Eliminated network attack surface for challenge authorization bypass.
- **Verification:**
  - Integration test `test_challenge_approve_endpoint_not_exposed_over_http` confirms HTTP approval attempts return 404 and in-process approval succeeds.

---

### 1.2 Fix Agent Tool Filename Sanitization Bug

- **Target Files:**
  - `notesplusplus-core/src/agent/session.rs` (`execute_tool`)
  - `notesplusplus-core/src/page.rs` (`sanitize_filename`, `ensure_adoc_extension`)
- **Problem & Impact:**
  In `AgentSession::execute_tool`, handling of `read_note` and `edit_note` runs `page::sanitize_filename(filename)`. Since `sanitize_filename` replaces `.` with `_`, `notes.adoc` becomes `notes_adoc`, resulting in "File not found" errors or creating corrupted extensionless files.
- **Remediation Steps:**
  1. Separate base stem sanitization from file extension handling.
  2. Update `session.rs` to strip `.adoc` (if present), sanitize the stem, and invoke `page::ensure_adoc_extension(&sanitized_stem)`.
  3. Prevent path traversal attempts (`..`, absolute paths) by enforcing canonical path containment within `notes_dir`.
- **Verification:**
  - Unit test `execute_tool` with `notes.adoc`, `sub/notes.adoc`, `my-note`, and `../../evil.adoc`.
  - Confirm file is created with `.adoc` intact and strictly within `notes_dir`.

---

### 1.3 Implement Atomic File Write Strategy

- **Target Files:**
  - `notesplusplus-core/src/server/routes/pages.rs` (`handle_update_page`, `handle_create_page`)
  - `notesplusplus-core/src/agent/session.rs` (`execute_tool` -> `edit_note`)
  - `notesplusplus-core/src/page.rs` (new helper `atomic_write_note`)
- **Problem & Impact:**
  Direct calls to `std::fs::write(&file_path, &content)` risk file truncation or corruption if the device powers off, battery dies, or the process crashes during write operations.
- **Remediation Steps:**
  1. Implement a shared helper `atomic_write(path: &Path, content: &str) -> std::io::Result<()>`.
  2. The helper writes to a temporary sibling file (e.g. `path.with_extension("tmp")`), flushes buffers (`sync_all()`), and atomically renames the temporary file over the target path (`std::fs::rename`).
  3. Replace all direct `std::fs::write` calls across server routes and agent tools with `atomic_write`.
- **Verification:**
  - Unit test checking that failed writes leave original file untouched.
  - Verify file permissions and sync consistency on Linux/Sailfish targets.

---

## Phase 2: Network & Parser Security Hardening (P1)

### 2.1 Robust IP-Based SSRF Mitigation & Redirect Hardening

- **Target Files:**
  - `notesplusplus-core/src/agent/tools.rs` (`is_blocked_host`, `fetch_url`)
  - `notesplusplus-core/src/server/routes/agent.rs` (`handle_fetch_url`)
- **Problem & Impact:**
  SSRF filtering currently matches substrings (e.g. `"10."`, `"localhost"`), causing false positives on legitimate public domains (`top10.com`) and failing against hex/decimal IP encodings, 0.0.0.0, DNS rebinding, and 3xx HTTP redirects.
- **Remediation Steps:**
  1. Parse the URL and resolve hostnames to IP addresses via `std::net::ToSocketAddrs` before connecting.
  2. Check resolved IPs against IP ranges:
     - Loopback (`127.0.0.0/8`, `::1`)
     - Private IPv4 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`)
     - Link-local / Cloud Metadata (`169.254.0.0/16`, `fe80::/10`)
     - Unspecified / Broadcast (`0.0.0.0/8`, `255.255.255.255/32`)
  3. Configure the HTTP agent (`ureq::AgentBuilder`) with `redirects(0)` or validate target IPs on each redirect hop manually.
- **Verification:**
  - Tests covering `http://127.0.0.1`, `http://2130706433`, `http://top10.com`, `http://localtest.me`, `http://169.254.169.254`.

---

### 2.2 Whitelist URL Schemes in HTML Link Rendering (Stored XSS)

- **Target Files:**
  - `notesplusplus-core/src/html.rs` (`render_spans` -> `InlineSpan::Link`)
- **Problem & Impact:**
  AsciiDoc links formatted as `link:javascript:alert(1)[Click]` render directly into `<a href="javascript:alert(1)">`, permitting arbitrary JavaScript execution when notes are previewed or exported to HTML.
- **Remediation Steps:**
  1. Inspect the URL scheme in `render_spans` for `InlineSpan::Link`.
  2. Allow only safe schemes: `http://`, `https://`, `mailto:`, `tel:`, or relative paths (`#`, `/`, `.`).
  3. For disallowed schemes (e.g. `javascript:`, `data:`, `vbscript:`), strip or sanitize the `href` attribute (e.g. render as `about:invalid` or text span).
- **Verification:**
  - Unit test verifying `link:javascript:steal()[test]` renders as safe HTML without executable scheme.
  - Verify standard links (`https://example.com`, `mailto:user@test.org`) render normally.

---

### 2.3 Constant-Time Comparison for Bearer Token Verification

- **Target Files:**
  - `notesplusplus-core/src/server/routes/mod.rs` (`is_authorized`)
  - `notesplusplus-core/src/server/auth.rs`
- **Problem & Impact:**
  Standard `==` string equality leaks timing information proportional to the number of matching prefix bytes, facilitating side-channel attacks on secret bearer tokens over low-latency networks.
- **Remediation Steps:**
  1. Integrate `subtle` crate or implement a constant-time slice comparison function:
     ```rust
     fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
         if a.len() != b.len() { return false; }
         a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
     }
     ```
  2. Apply constant-time validation to all token and session secret comparisons in route auth gates.
- **Verification:**
  - Unit tests verifying token equality matches identical tokens and rejects mismatched tokens.

---

## Phase 3: Performance, I/O & Concurrency (P2)

### 3.1 Replace Filesystem Scans with SQLite Index Queries

- **Target Files:**
  - `notesplusplus-core/src/server/routes/pages.rs` (`list_all_notes_json`)
  - `notesplusplus-core/src/db.rs`
- **Problem & Impact:**
  `list_all_notes_json` performs a full directory traversal and disk read of every `.adoc` file upon every request to construct note titles and preview snippets, bypassing the SQLite database cache.
- **Remediation Steps:**
  1. Update `list_all_notes_json` to query `SELECT id, title, path, updated_at FROM pages` directly from SQLite.
  2. Ensure page creation, edit, and delete handlers update the SQLite `pages` table transactionally.
  3. Keep filesystem scans solely as a background reconciliation/sync fallback on application boot.
- **Verification:**
  - Benchmark listing endpoint with 500+ notes; verify zero file I/O during standard list requests.

---

### 3.2 Granular Mutex Strategy for LLM Agent State

- **Target Files:**
  - `notesplusplus-core/src/server/mod.rs` (`ServerContext`)
  - `notesplusplus-core/src/agent/session.rs`
- **Problem & Impact:**
  The LLM agent session (`ctx.session: Arc<Mutex<AgentSession>>`) is locked for the entire duration of streaming LLM network requests, blocking health checks, status queries, and concurrent read requests.
- **Remediation Steps:**
  1. Separate active turn execution state from metadata/configuration.
  2. Use channel-based communication or an asynchronous/state-machine approach where the lock is released while waiting on external network I/O from LLM endpoints.
  3. Allow read-only status inspection (`is_busy`, pending tool calls) without acquiring an exclusive long-held execution lock.
- **Verification:**
  - Test issuing a status request while an LLM streaming query is active; verify response returns immediately.

---

### 3.3 Database Connection Pooling / Shared Connections

- **Target Files:**
  - `notesplusplus-core/src/server/mod.rs`
  - `notesplusplus-core/src/db.rs`
- **Problem & Impact:**
  A new SQLite database connection (`Connection::open(&ctx.db_path)`) is opened and closed for each incoming HTTP request, incurring redundant disk and file-descriptor overhead.
- **Remediation Steps:**
  1. Provide a managed SQLite connection pool (`r2d2_sqlite`) or an `Arc<Mutex<Connection>>` with WAL (Write-Ahead Logging) mode enabled.
  2. Configure SQLite `PRAGMA journal_mode=WAL;` and `PRAGMA synchronous=NORMAL;` for optimal concurrency and read performance on embedded flash storage.
- **Verification:**
  - Verify concurrent read and write operations under multi-threaded test conditions.

---

## Phase 4: Code Quality, Deduplication & Architecture (P3)

### 4.1 Consolidate Fragmented Title & Slugification Helpers

- **Target Files:**
  - `notesplusplus-core/src/page.rs`
  - `notesplusplus-core/src/server/routes/pages.rs`
  - `notesplusplus-core/src/agent/session.rs`
  - `notesplusplus-core/src/agent/tools.rs`
- **Problem & Impact:**
  Title extraction (`extract_title`, `extract_title_from_adoc`, `extract_doc_title`) and filename slugification are implemented across multiple modules with diverging edge-case behavior.
- **Remediation Steps:**
  1. Consolidate title extraction into a single canonical function in `notesplusplus-core/src/page.rs`:
     - Checks AsciiDoc header `= Document Title`
     - Falls back to first section heading `== Heading`
     - Falls back to first non-empty line or file stem
  2. Consolidate slugification and extension normalization in `page.rs` and re-export across server and agent modules.
  3. Remove duplicated implementations in `routes/pages.rs` and `agent/session.rs`.
- **Verification:**
  - Comprehensive unit test suite covering title extraction edge cases (empty files, attributes, comments, Unicode).

---

### 4.2 Decouple Route Handlers via Repository Layer

- **Target Files:**
  - `notesplusplus-core/src/server/routes/`
  - `notesplusplus-core/src/page.rs`
- **Problem & Impact:**
  HTTP route handlers currently intertwine raw HTTP parsing, database access, filesystem I/O, error formatting, and JSON generation, hindering automated testing without HTTP fixtures.
- **Remediation Steps:**
  1. Define a `NoteRepository` trait defining core operations: `list()`, `get(id)`, `create(note)`, `update(id, content)`, `delete(id)`.
  2. Implement `FsSqliteNoteRepository` implementing the trait.
  3. Refactor route handlers to operate purely as thin presentation adapters over `NoteRepository`.
- **Verification:**
  - Unit tests for repository logic isolated from HTTP transport.

---

### 4.3 Modernize Qt Quick / Rust FFI Data Transfer

- **Target Files:**
  - `notesplusplus-sailfish/src/ffi.rs`
  - `notesplusplus-sailfish/src/bridge/`
  - `notesplusplus-sailfish/qml/`
- **Problem & Impact:**
  Passing whole note lists and search result sets as serialized JSON strings over FFI forces QML to parse large JSON trees in JavaScript, degrading UI rendering frame rates and increasing GC pressure on mobile hardware.
- **Remediation Steps:**
  1. Implement a `QAbstractListModel` binding or structured `QVariantList` objects for search results and page catalogs.
  2. Expose incremental updates (insert, update, remove signals) rather than full list invalidations.
  3. Move intensive parsing and indexing off the main Qt UI thread onto background worker threads.
- **Verification:**
  - Validate smooth 60fps scrolling on Sailfish OS device with >1000 notes loaded.

---

### 4.4 Embedded HTTP Server Hardening

- **Target Files:**
  - `notesplusplus-core/src/server/http.rs`
  - `notesplusplus-core/src/server/mod.rs`
- **Problem & Impact:**
  The custom HTTP/1.1 implementation lacks header size limits, request body size enforcement, and read/write timeouts, leaving it exposed to Slowloris attacks or unconstrained memory allocation.
- **Remediation Steps:**
  1. Enforce strict limits on request header lines (e.g. max 8KB headers, max 100 headers).
  2. Set TCP stream read and write timeouts (`set_read_timeout`, `set_write_timeout`) on incoming connections.
  3. Enforce maximum payload size limits for `POST`/`PUT` bodies.
- **Verification:**
  - Test against oversized HTTP request headers and slow streaming connections; ensure socket is cleanly closed with appropriate HTTP error code.

---

## Suggested Execution Roadmap

```mermaid
graph LR
    P0[Phase 1: P0 Critical<br/>Auth Bypass, Agent Bug, Atomic Writes] --> P1[Phase 2: P1 Security<br/>SSRF, XSS Schemes, Constant-Time]
    P1 --> P2[Phase 3: P2 Performance<br/>SQLite List Cache, Granular Locks, Pooling]
    P2 --> P3[Phase 4: P3 Quality<br/>Deduplication, Repository Trait, FFI Models]
```

1. **Sprint 1 (Immediate Safety & Reliability):**
   - Address Phase 1 items (1.1, 1.2, 1.3).
   - High net yield: Secures device against LAN takeover and protects user notes against write corruption.

2. **Sprint 2 (Security Hardening):**
   - Address Phase 2 items (2.1, 2.2, 2.3).
   - High net yield: Closes external exploit vectors (SSRF & XSS) in agent and export tools.

3. **Sprint 3 (Performance & Battery Efficiency):**
   - Address Phase 3 items (3.1, 3.2, 3.3).
   - High net yield: Eliminates flash I/O bottlenecks and improves app responsiveness.

4. **Sprint 4 (Maintainability & Clean Architecture):**
   - Address Phase 4 items (4.1, 4.2, 4.3, 4.4).
   - High net yield: Lowers ongoing maintenance overhead and streamlines future feature additions.
