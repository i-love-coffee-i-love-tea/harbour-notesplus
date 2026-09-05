use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::html::adoc_to_html5;

/// Handle for controlling the running embedded HTTP server.
pub struct HttpServerHandle {
    is_running: Arc<AtomicBool>,
    port: u16,
    local_urls: Vec<String>,
}

impl HttpServerHandle {
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn urls(&self) -> &[String] {
        &self.local_urls
    }

    pub fn primary_url(&self) -> String {
        self.local_urls
            .first()
            .cloned()
            .unwrap_or_else(|| format!("http://127.0.0.1:{}", self.port))
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        // Ping the server to unblock the listener accept loop
        let _ = TcpStream::connect(format!("127.0.0.1:{}", self.port));
    }
}

/// Start the embedded documentation HTTP server on a background thread.
pub fn start_server(notes_path: PathBuf, requested_port: u16) -> Result<HttpServerHandle, String> {
    let port = if requested_port == 0 { 8080 } else { requested_port };
    let mut listener = None;

    for p in port..(port + 20) {
        if let Ok(l) = TcpListener::bind(("0.0.0.0", p)) {
            listener = Some((l, p));
            break;
        }
    }

    let (listener, actual_port) = match listener {
        Some(res) => res,
        None => return Err(format!("Failed to bind to port {} or nearby ports", port)),
    };

    let local_ips = get_local_ip_addresses();
    let mut local_urls = Vec::new();
    for ip in &local_ips {
        local_urls.push(format!("http://{}:{}", ip, actual_port));
    }
    if local_urls.is_empty() {
        local_urls.push(format!("http://127.0.0.1:{}", actual_port));
    }

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    let notes_path_clone = notes_path.clone();

    thread::spawn(move || {
        // Set timeout on accept so thread can check running flag periodically
        let _ = listener.set_nonblocking(false);

        while is_running_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if !is_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let notes_dir = notes_path_clone.clone();
                    thread::spawn(move || {
                        handle_http_client(stream, &notes_dir);
                    });
                }
                Err(_) => {
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    });

    Ok(HttpServerHandle {
        is_running,
        port: actual_port,
        local_urls,
    })
}

fn handle_http_client(mut stream: TcpStream, notes_dir: &Path) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));

    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_response(&mut stream, 400, "Bad Request", "text/plain", b"Bad Request");
        return;
    }

    let method = parts[0];
    let raw_path = parts[1];

    if method != "GET" && method != "HEAD" {
        send_response(&mut stream, 405, "Method Not Allowed", "text/plain", b"Method Not Allowed");
        return;
    }

    let (path, query) = match raw_path.split_once('?') {
        Some((p, q)) => (url_decode(p), Some(url_decode(q))),
        None => (url_decode(raw_path), None),
    };

    let clean_path = path.trim_start_matches('/');

    if clean_path.is_empty() || clean_path == "index.html" {
        // Render Notes Index / Dashboard
        let html = render_dashboard_html(notes_dir, query.as_deref());
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes());
        return;
    }

    if clean_path.starts_with("page/") || clean_path.starts_with("notes/") {
        let note_name = clean_path.strip_prefix("page/").or_else(|| clean_path.strip_prefix("notes/")).unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else {
            format!("{}.adoc", note_name)
        };

        let file_path = notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                let html = render_web_page_html(&content, title, notes_dir, &filename);
                send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes());
                return;
            }
        }
    }

    if clean_path.starts_with("raw/") {
        let note_name = clean_path.strip_prefix("raw/").unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else {
            format!("{}.adoc", note_name)
        };

        let file_path = notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                send_response(&mut stream, 200, "OK", "text/plain; charset=utf-8", content.as_bytes());
                return;
            }
        }
    }

    if clean_path.starts_with("export/") {
        let note_name = clean_path.strip_prefix("export/").unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else if note_name.ends_with(".html") {
            format!("{}.adoc", note_name.strip_suffix(".html").unwrap_or(note_name))
        } else {
            format!("{}.adoc", note_name)
        };

        let file_path = notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                let html = adoc_to_html5(&content, title, Some(notes_dir));
                send_attachment_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &format!("{}.html", title));
                return;
            }
        }
    }

    // Static asset from notes directory (images, svgs, etc.)
    let asset_path = if clean_path.starts_with("assets/") {
        notes_dir.join(clean_path.strip_prefix("assets/").unwrap_or(clean_path))
    } else {
        notes_dir.join(clean_path)
    };

    if asset_path.is_file() {
        if let Ok(bytes) = std::fs::read(&asset_path) {
            let ext = asset_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            let mime = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "svg" => "image/svg+xml",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "css" => "text/css; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                _ => "application/octet-stream",
            };
            send_response(&mut stream, 200, "OK", mime, &bytes);
            return;
        }
    }

    send_response(&mut stream, 404, "Not Found", "text/html; charset=utf-8", b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes Index</a></p>");
}

fn send_response(stream: &mut TcpStream, status_code: u16, status_text: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status_code,
        status_text,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn send_attachment_response(stream: &mut TcpStream, status_code: u16, status_text: &str, content_type: &str, body: &[u8], filename: &str) {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=\"{}\"\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status_code,
        status_text,
        content_type,
        body.len(),
        filename
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn render_dashboard_html(notes_dir: &Path, search_query: Option<&str>) -> String {
    let mut notes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(notes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let title = name.strip_suffix(".adoc").unwrap_or(name);
                    let mut snippet = String::new();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() && !trimmed.starts_with('=') && !trimmed.starts_with("//") {
                                snippet = trimmed.chars().take(120).collect();
                                break;
                            }
                        }
                    }
                    notes.push((title.to_string(), name.to_string(), snippet));
                }
            }
        }
    }

    notes.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let query_str = search_query.unwrap_or("").trim();
    let filtered_notes: Vec<_> = if query_str.is_empty() {
        notes
    } else {
        let q = query_str.to_lowercase();
        notes
            .into_iter()
            .filter(|(title, filename, snippet)| {
                title.to_lowercase().contains(&q)
                    || filename.to_lowercase().contains(&q)
                    || snippet.to_lowercase().contains(&q)
            })
            .collect()
    };

    let mut cards_html = String::new();
    for (title, filename, snippet) in &filtered_notes {
        cards_html.push_str(&format!(
            r#"<a class="note-card" href="/page/{filename}">
                <div class="note-card-title">{title}</div>
                <div class="note-card-snippet">{snippet}</div>
                <div class="note-card-footer">
                    <span class="view-link">View Page &rarr;</span>
                    <span class="export-link" onclick="event.preventDefault(); window.location.href='/export/{filename}';">HTML5</span>
                </div>
            </a>"#,
            filename = filename,
            title = escape_html(title),
            snippet = escape_html(snippet)
        ));
    }

    if cards_html.is_empty() {
        cards_html = "<div class=\"empty-state\"><p>No notes found matching your search.</p></div>".to_string();
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Notes++ Documentation Library</title>
    <style>
{css}
    </style>
</head>
<body class="dashboard-body">
    <header class="top-nav">
        <div class="nav-container">
            <div class="brand">
                <span class="logo">&#128214;</span> <strong>Notes++</strong> <span class="badge">Web Portal</span>
            </div>
            <form class="search-form" method="GET" action="/">
                <input type="text" name="q" placeholder="Search documentation..." value="{query}">
                <button type="submit">Search</button>
            </form>
        </div>
    </header>
    <main class="dashboard-main">
        <div class="dashboard-header">
            <h2>Your Documentation ({count} notes)</h2>
            <p>Live documentation server running on Sailfish OS. Click any note to read or download as standalone HTML5.</p>
        </div>
        <div class="notes-grid">
            {cards}
        </div>
    </main>
    <footer class="dashboard-footer">
        <p>Served live by <strong>Notes++</strong> on Sailfish OS</p>
    </footer>
</body>
</html>"#,
        css = DASHBOARD_CSS,
        query = escape_html(query_str),
        count = filtered_notes.len(),
        cards = cards_html
    )
}

fn render_web_page_html(adoc_content: &str, title: &str, notes_dir: &Path, filename: &str) -> String {
    let standalone = adoc_to_html5(adoc_content, title, Some(notes_dir));

    let top_bar = format!(
        r#"<div class="web-page-topbar">
            <div class="topbar-left">
                <a class="nav-btn" href="/">&larr; Notes Index</a>
                <span class="page-current-title">{}</span>
            </div>
            <div class="topbar-right">
                <a class="action-btn" href="/raw/{}" target="_blank">Raw AsciiDoc</a>
                <a class="action-btn primary" href="/export/{}">Download HTML5</a>
            </div>
        </div>"#,
        escape_html(title),
        filename,
        filename
    );

    standalone.replacen(
        "<body class=\"fishdoc-body\">",
        &format!("<body class=\"fishdoc-body with-topbar\">\n{}", top_bar),
        1
    )
}

fn get_local_ip_addresses() -> Vec<String> {
    let mut ips = Vec::new();

    // 1. Try finding local IP via UDP probe (zero traffic, binds routing table)
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip_str = addr.ip().to_string();
                if ip_str != "0.0.0.0" && ip_str != "127.0.0.1" {
                    ips.push(ip_str);
                }
            }
        }
    }

    // 2. Read Linux /proc/net/arp or ifconfig if available
    if let Ok(arp) = std::fs::read_to_string("/proc/net/arp") {
        for line in arp.lines().skip(1) {
            if let Some(ip) = line.split_whitespace().next() {
                if ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.") {
                    // Try to probe that network segment
                    if let Ok(probe_sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                        if probe_sock.connect(format!("{}:80", ip)).is_ok() {
                            if let Ok(addr) = probe_sock.local_addr() {
                                let local = addr.ip().to_string();
                                if !ips.contains(&local) && local != "0.0.0.0" && local != "127.0.0.1" {
                                    ips.push(local);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }

    ips
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next().unwrap_or(0);
            let h2 = bytes.next().unwrap_or(0);
            if let (Some(d1), Some(d2)) = (hex_val(h1), hex_val(h2)) {
                result.push(((d1 << 4) | d2) as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

const DASHBOARD_CSS: &str = r#"
:root {
    --bg-color: #f8fafc;
    --card-bg: #ffffff;
    --text-color: #1e293b;
    --text-muted: #64748b;
    --border-color: #e2e8f0;
    --accent: #2563eb;
    --accent-hover: #1d4ed8;
    --header-bg: #ffffff;
    --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05), 0 2px 4px -2px rgba(0, 0, 0, 0.05);
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-color: #0f172a;
        --card-bg: #1e293b;
        --text-color: #f1f5f9;
        --text-muted: #94a3b8;
        --border-color: #334155;
        --accent: #38bdf8;
        --accent-hover: #0284c7;
        --header-bg: #1e293b;
        --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
    }
}

* { box-sizing: border-box; }

body.dashboard-body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.5;
}

.top-nav {
    background-color: var(--header-bg);
    border-bottom: 1px solid var(--border-color);
    padding: 12px 24px;
    position: sticky;
    top: 0;
    z-index: 100;
}

.nav-container {
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
}

.brand {
    font-size: 1.25rem;
    display: flex;
    align-items: center;
    gap: 8px;
}

.badge {
    font-size: 0.75rem;
    background: var(--accent);
    color: #fff;
    padding: 2px 8px;
    border-radius: 12px;
    font-weight: 600;
}

.search-form {
    display: flex;
    gap: 8px;
}

.search-form input {
    padding: 8px 14px;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background-color: var(--bg-color);
    color: var(--text-color);
    font-size: 0.9rem;
    min-width: 240px;
}

.search-form button {
    padding: 8px 16px;
    background-color: var(--accent);
    color: #fff;
    border: none;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
}

.search-form button:hover {
    background-color: var(--accent-hover);
}

.dashboard-main {
    max-width: 1100px;
    margin: 32px auto;
    padding: 0 24px;
}

.dashboard-header {
    margin-bottom: 28px;
}

.dashboard-header h2 {
    margin: 0 0 6px 0;
    font-size: 1.75rem;
}

.dashboard-header p {
    margin: 0;
    color: var(--text-muted);
}

.notes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 20px;
}

.note-card {
    background-color: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 20px;
    text-decoration: none;
    color: inherit;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: transform 0.15s ease, border-color 0.15s ease;
}

.note-card:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
}

.note-card-title {
    font-size: 1.2rem;
    font-weight: 700;
    margin-bottom: 8px;
    color: var(--text-color);
}

.note-card-snippet {
    font-size: 0.92rem;
    color: var(--text-muted);
    margin-bottom: 16px;
    line-height: 1.45;
    flex-grow: 1;
}

.note-card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--border-color);
    padding-top: 12px;
    font-size: 0.85rem;
}

.view-link {
    color: var(--accent);
    font-weight: 600;
}

.export-link {
    background-color: var(--bg-color);
    border: 1px solid var(--border-color);
    padding: 4px 10px;
    border-radius: 4px;
    font-weight: 500;
    cursor: pointer;
}

.export-link:hover {
    border-color: var(--accent);
    color: var(--accent);
}

.empty-state {
    grid-column: 1 / -1;
    text-align: center;
    padding: 48px;
    color: var(--text-muted);
}

.dashboard-footer {
    text-align: center;
    padding: 48px 24px;
    color: var(--text-muted);
    font-size: 0.85rem;
}

/* Standalone Page Topbar */
.web-page-topbar {
    background-color: var(--header-bg);
    border-bottom: 1px solid var(--border-color);
    padding: 10px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    position: sticky;
    top: 0;
    z-index: 1000;
    box-shadow: var(--shadow);
}

.topbar-left {
    display: flex;
    align-items: center;
    gap: 16px;
}

.page-current-title {
    font-weight: 700;
    font-size: 1.05rem;
}

.nav-btn, .action-btn {
    text-decoration: none;
    font-size: 0.88rem;
    font-weight: 600;
    padding: 6px 14px;
    border-radius: 6px;
    display: inline-block;
    border: 1px solid var(--border-color);
    background-color: var(--card-bg);
    color: var(--text-color);
}

.nav-btn:hover, .action-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
}

.action-btn.primary {
    background-color: var(--accent);
    color: #ffffff;
    border-color: var(--accent);
}

.action-btn.primary:hover {
    background-color: var(--accent-hover);
}
"#;
