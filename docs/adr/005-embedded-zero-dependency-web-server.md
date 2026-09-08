# ADR-005: Lightweight Embedded HTTP Server with Server-Sent Events (SSE)

#### Status
Accepted

#### Context
Users need to view, edit, and present notes from desktop browsers or laptops on the same local network (Wi-Fi) without installing external client software or syncing to third-party cloud servers. 

Running heavyweight web runtimes (Node.js, Python, or WebAssembly containers) on Sailfish OS consumes prohibitive memory, drains battery, and complicates RPM packaging.

#### Decision
Implement a zero-external-dependency embedded HTTP/1.1 server in `notesplusplus-core` using standard library TCP networking (`std::net::TcpListener`, `std::net::TcpStream`) and a worker thread pool.

Key server features:
1. **REST APIs & Static Asset Delivery**: Serves notes (`/api/notes`, `/api/pages`), renders HTML previews (`/api/render`), handles authentication (`/api/auth`), and serves embedded web assets (Vue 3 ESM app, stylesheets, icons).
2. **Server-Sent Events (SSE)**: Streams real-time token chunks from the AI assistant endpoint (`/api/agent/chat`) using HTTP chunked transfer encoding (`text/event-stream`).
3. **Zero Idle CPU Drain**: The TCP listener utilizes blocking OS kernel socket polling (`accept()`), maintaining 0% CPU consumption and zero battery drain when idle.
4. **Session Authentication & Security**: Enforces cookie-based session management (`fishdoc_session`) with persistent session tokens surviving application restarts.

#### Consequences
##### Positive / Utility Delivered
- **Zero External Runtime Dependencies**: Compiles directly into the native Sailfish application binary with zero third-party web framework dependencies.
- **Negligible Power Footprint**: Blocks on OS socket interrupts with no busy loops or polling timers on the host device.
- **Full Desktop Companion**: Delivers full note editing, split preview, document search, and slide presentations over local Wi-Fi.

##### Trade-offs / Mitigations
- Raw HTTP/1.1 parsing requires careful handling of request boundaries, header casing, path traversal sanitization, and CORS preflight headers.
