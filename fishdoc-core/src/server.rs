//! Embedded HTTP server and REST / SSE API for Fishdoc Web Editor & AI Assistant.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use rusqlite::Connection;
use serde_json::json;

use crate::agent::{
    build_template_instruction, AgentSession, AgentStepResult, LlmClient, LlmConfig, LlmProvider,
    PermissionConfig, PermissionManager,
};
use crate::block::Block;
use crate::db;
use crate::html::{adoc_to_html5, adoc_to_html_body, blocks_to_html_body};
use crate::page;
use crate::parser;

pub mod web_assets;
use web_assets::{APP_JS, INDEX_HTML, STYLE_CSS};

/// Server configuration holding paths, networking, and AI client parameters.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub notes_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub port: u16,
    pub llm_config: LlmConfig,
    pub permission_config: PermissionConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let data_dir = PathBuf::from(&home).join(".local").join("share").join("harbour-fishdoc");
        let notes_dir = data_dir.join("notes");
        let db_path = data_dir.join("fishdoc.db");
        let backup_dir = data_dir.join("backups");
        Self {
            notes_dir,
            db_path,
            backup_dir,
            port: 8080,
            llm_config: LlmConfig::default(),
            permission_config: PermissionConfig::default(),
        }
    }
}

/// Shared runtime context passed across request worker threads.
#[derive(Clone)]
pub struct ServerContext {
    pub notes_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub session: Arc<Mutex<AgentSession>>,
    pub llm_config: Arc<Mutex<LlmConfig>>,
    pub perm_config: Arc<Mutex<PermissionConfig>>,
}

impl ServerContext {
    pub fn new(config: ServerConfig) -> Self {
        let _ = fs::create_dir_all(&config.notes_dir);
        let _ = fs::create_dir_all(&config.backup_dir);

        let perm_mgr = PermissionManager::new(config.permission_config.clone());
        let client = LlmClient::new(config.llm_config.clone());
        let mut session = AgentSession::new(
            &config.notes_dir,
            &config.db_path,
            &config.backup_dir,
            perm_mgr,
            client,
        );
        session.reset_session(None, None);

        Self {
            notes_dir: config.notes_dir,
            db_path: config.db_path,
            backup_dir: config.backup_dir,
            session: Arc::new(Mutex::new(session)),
            llm_config: Arc::new(Mutex::new(config.llm_config)),
            perm_config: Arc::new(Mutex::new(config.permission_config)),
        }
    }

    /// Updates the active LLM client and permission configurations.
    pub fn update_llm_config(&self, config: LlmConfig, perm_config: Option<PermissionConfig>) {
        let mut cfg_guard = self.llm_config.lock().unwrap();
        *cfg_guard = config.clone();
        if let Some(p) = perm_config {
            let mut perm_guard = self.perm_config.lock().unwrap();
            *perm_guard = p;
        }
        let new_client = LlmClient::new(config);
        let perm_mgr = PermissionManager::new(self.perm_config.lock().unwrap().clone());
        self.session.lock().unwrap().update_config(perm_mgr, new_client);
    }
}

/// Handle for controlling the running embedded HTTP server.
pub struct HttpServerHandle {
    is_running: Arc<AtomicBool>,
    port: u16,
    local_urls: Vec<String>,
    context: ServerContext,
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

    pub fn context(&self) -> &ServerContext {
        &self.context
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        let _ = TcpStream::connect(format!("127.0.0.1:{}", self.port));
    }
}

/// Start the embedded documentation HTTP server with default configuration.
pub fn start_server(notes_path: PathBuf, requested_port: u16) -> Result<HttpServerHandle, String> {
    let mut config = ServerConfig::default();
    config.notes_dir = notes_path.clone();
    config.db_path = notes_path.parent().unwrap_or(&notes_path).join("fishdoc.db");
    config.backup_dir = notes_path.parent().unwrap_or(&notes_path).join("backups");
    config.port = requested_port;
    start_server_with_config(config)
}

/// Start the embedded documentation HTTP server with full explicit configuration.
pub fn start_server_full(
    notes_dir: PathBuf,
    db_path: PathBuf,
    backup_dir: PathBuf,
    requested_port: u16,
    llm_config: Option<LlmConfig>,
    permission_config: Option<PermissionConfig>,
) -> Result<HttpServerHandle, String> {
    let config = ServerConfig {
        notes_dir,
        db_path,
        backup_dir,
        port: requested_port,
        llm_config: llm_config.unwrap_or_default(),
        permission_config: permission_config.unwrap_or_default(),
    };
    start_server_with_config(config)
}

/// Start the embedded documentation HTTP server with a `ServerConfig`.
pub fn start_server_with_config(config: ServerConfig) -> Result<HttpServerHandle, String> {
    let port = if config.port == 0 { 8080 } else { config.port };
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
    let context = ServerContext::new(config);
    let context_clone = context.clone();

    thread::spawn(move || {
        let _ = listener.set_nonblocking(false);

        while is_running_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if !is_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let ctx = context_clone.clone();
                    thread::spawn(move || {
                        handle_http_client(stream, ctx);
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
        context,
    })
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    query: Option<String>,
    _headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn parse_http_request(stream: &mut TcpStream) -> Result<ParsedHttpRequest, String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| e.to_string())?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request line".to_string());
    }

    let method = parts[0].to_uppercase();
    let raw_path = parts[1];
    let (path, query) = match raw_path.split_once('?') {
        Some((p, q)) => (url_decode(p), Some(url_decode(q))),
        None => (url_decode(raw_path), None),
    };

    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }

    Ok(ParsedHttpRequest {
        method,
        path,
        query,
        _headers: headers,
        body,
    })
}

fn handle_http_client(mut stream: TcpStream, ctx: ServerContext) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(120)));

    let req = match parse_http_request(&mut stream) {
        Ok(r) => r,
        Err(_) => {
            send_response(&mut stream, 400, "Bad Request", "text/plain", b"Bad Request");
            return;
        }
    };

    // Handle CORS preflight requests
    if req.method == "OPTIONS" {
        send_response(&mut stream, 200, "OK", "text/plain", b"");
        return;
    }

    let clean_path = req.path.trim_start_matches('/');

    // Reject path traversal attempts
    if clean_path.contains("..") {
        send_response(&mut stream, 403, "Forbidden", "text/plain", b"Forbidden");
        return;
    }

    // 1. Static Web Application Root & Assets
    if req.method == "GET" && (clean_path.is_empty() || clean_path == "index.html") {
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes());
        return;
    }

    if req.method == "GET" && clean_path == "app.js" {
        send_response(&mut stream, 200, "OK", "application/javascript; charset=utf-8", APP_JS.as_bytes());
        return;
    }

    if req.method == "GET" && clean_path == "style.css" {
        send_response(&mut stream, 200, "OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes());
        return;
    }

    // 2. REST API: Notes Management
    if clean_path == "api/notes" {
        match req.method.as_str() {
            "GET" => {
                let list = list_all_notes_json(&ctx.notes_dir, req.query.as_deref());
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", list.as_bytes());
                return;
            }
            "POST" => {
                let body_str = String::from_utf8_lossy(&req.body);
                let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
                let title = json_body.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Note");
                let content = json_body.get("content").and_then(|v| v.as_str()).unwrap_or("");

                let filename = make_slug_filename(title);
                let file_path = ctx.notes_dir.join(&filename);
                if let Err(e) = fs::write(&file_path, content) {
                    let err = json!({ "error": format!("Failed to create note: {}", e) });
                    send_response(&mut stream, 500, "Internal Server Error", "application/json; charset=utf-8", err.to_string().as_bytes());
                    return;
                }

                // Index in SQLite if DB is present
                if let Ok(conn) = Connection::open(&ctx.db_path) {
                    let _ = db::init_schema(&conn);
                    if let Ok(p) = page::create_page(&conn, &ctx.notes_dir, title, false) {
                        let _ = db::update_fts_content(&conn, p.id, content);
                    }
                }

                let resp = json!({
                    "ok": true,
                    "filename": filename,
                    "title": title
                });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                return;
            }
            _ => {}
        }
    }

    if clean_path.starts_with("api/notes/") {
        let note_name = clean_path.strip_prefix("api/notes/").unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else {
            format!("{}.adoc", note_name)
        };
        let file_path = ctx.notes_dir.join(&filename);

        match req.method.as_str() {
            "GET" => {
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        send_response(&mut stream, 200, "OK", "text/plain; charset=utf-8", content.as_bytes());
                        return;
                    }
                }
                send_response(&mut stream, 404, "Not Found", "application/json; charset=utf-8", b"{\"error\":\"Note not found\"}");
                return;
            }
            "PUT" => {
                let content = if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    json_body.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| String::from_utf8_lossy(&req.body).to_string())
                } else {
                    String::from_utf8_lossy(&req.body).to_string()
                };

                if let Err(e) = fs::write(&file_path, &content) {
                    let err = json!({ "error": format!("Failed to save note: {}", e) });
                    send_response(&mut stream, 500, "Internal Server Error", "application/json; charset=utf-8", err.to_string().as_bytes());
                    return;
                }

                // Update database index
                if let Ok(conn) = Connection::open(&ctx.db_path) {
                    let _ = db::init_schema(&conn);
                    let title = extract_title_from_adoc(&content, &filename);
                    if let Ok(Some(existing_page)) = page::get_page(&conn, &filename) {
                        let _ = db::update_fts_content(&conn, existing_page.id, &content);
                    } else {
                        let now = chrono::Utc::now().to_rfc3339();
                        let _ = conn.execute(
                            "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count) VALUES (?1, ?2, 0, ?3, ?3, 0)",
                            rusqlite::params![filename, title, now],
                        );
                        if let Ok(page_id) = conn.query_row(
                            "SELECT id FROM pages WHERE filename = ?1",
                            rusqlite::params![filename],
                            |row| row.get::<_, i64>(0),
                        ) {
                            let _ = db::update_fts_content(&conn, page_id, &content);
                        }
                    }
                }

                let resp = json!({ "ok": true, "filename": filename });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                return;
            }
            "DELETE" => {
                if file_path.is_file() {
                    let _ = fs::remove_file(&file_path);
                    if let Ok(conn) = Connection::open(&ctx.db_path) {
                        let _ = conn.execute("DELETE FROM pages WHERE filename = ?1", rusqlite::params![filename]);
                    }
                }
                let resp = json!({ "ok": true });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                return;
            }
            _ => {}
        }
    }

    // 2b. REST API: Toggle checklist item in note
    if req.method == "POST" && clean_path.starts_with("api/notes/") && clean_path.ends_with("/toggle") {
        let raw_name = clean_path
            .strip_prefix("api/notes/")
            .unwrap()
            .strip_suffix("/toggle")
            .unwrap();
        let filename = if raw_name.ends_with(".adoc") {
            raw_name.to_string()
        } else {
            format!("{}.adoc", raw_name)
        };
        let file_path = ctx.notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let body_str = String::from_utf8_lossy(&req.body);
                let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
                let block_idx = json_body.get("block_index").and_then(|v| v.as_u64()).map(|v| v as usize);
                let item_idx = json_body.get("item_index").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(0);
                let target_checked = json_body.get("checked").and_then(|v| v.as_bool());

                let mut blocks = parser::parse_blocks(&content);
                let mut updated = false;

                if let Some(b_idx) = block_idx {
                    if b_idx < blocks.len() {
                        if let Block::UnorderedListItem { ref mut checked, ref mut raw, .. } = blocks[b_idx] {
                            let new_val = target_checked.unwrap_or_else(|| !checked.unwrap_or(false));
                            *checked = Some(new_val);
                            if new_val {
                                *raw = raw.replacen("[ ]", "[x]", 1).replacen("[*]", "[x]", 1);
                            } else {
                                *raw = raw.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1).replacen("[*]", "[ ]", 1);
                            }
                            updated = true;
                        }
                    }
                } else {
                    let mut check_count = 0;
                    for b in &mut blocks {
                        if let Block::UnorderedListItem { ref mut checked, ref mut raw, .. } = b {
                            if checked.is_some() {
                                if check_count == item_idx {
                                    let new_val = target_checked.unwrap_or_else(|| !checked.unwrap_or(false));
                                    *checked = Some(new_val);
                                    if new_val {
                                        *raw = raw.replacen("[ ]", "[x]", 1).replacen("[*]", "[x]", 1);
                                    } else {
                                        *raw = raw.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1).replacen("[*]", "[ ]", 1);
                                    }
                                    updated = true;
                                    break;
                                }
                                check_count += 1;
                            }
                        }
                    }
                }

                if updated {
                    let new_content = parser::blocks_to_adoc(&blocks);
                    let _ = fs::write(&file_path, new_content.as_bytes());
                    let html = adoc_to_html_body(&new_content, Some(&ctx.notes_dir));
                    let resp = json!({ "ok": true, "content": new_content, "html": html });
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                    return;
                }
            }
        }
        send_response(&mut stream, 404, "Not Found", "application/json; charset=utf-8", b"{\"error\":\"Note or checklist item not found\"}");
        return;
    }

    // 3. REST API: Render AsciiDoc to HTML
    if clean_path == "api/render" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let (content, is_full) = if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let c = json_body.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let full = json_body.get("full").and_then(|v| v.as_bool()).unwrap_or(false);
            (c, full)
        } else {
            (body_str.to_string(), false)
        };

        let html = if is_full {
            adoc_to_html5(&content, "Rendered Document", Some(&ctx.notes_dir))
        } else {
            adoc_to_html_body(&content, Some(&ctx.notes_dir))
        };
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes());
        return;
    }

    // 3b. REST API: Parse and Render Blocks AST
    if clean_path == "api/blocks/parse" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let content = if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
            json_body.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string()
        } else {
            body_str.to_string()
        };

        let blocks = parser::parse_blocks(&content);
        let block_items: Vec<serde_json::Value> = blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let html = blocks_to_html_body(&[b.clone()], Some(&ctx.notes_dir));
                json!({
                    "index": i,
                    "raw": b.raw_text(),
                    "html": html
                })
            })
            .collect();

        let resp = json!({ "blocks": block_items });
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
        return;
    }

    // 4. REST / SSE API: AI Assistant (Brokered via backend AgentSession & LlmClient)
    if clean_path == "api/ai/config" {
        match req.method.as_str() {
            "GET" => {
                let cfg = ctx.llm_config.lock().unwrap();
                let is_undo_available = ctx.session.lock().unwrap().can_undo();
                let has_pending = ctx.session.lock().unwrap().pending_action().is_some();
                let resp = json!({
                    "provider": match cfg.provider {
                        LlmProvider::Ollama => "ollama",
                        LlmProvider::OpenAiCompatible => "openai",
                    },
                    "endpoint": cfg.endpoint_url,
                    "model": cfg.model,
                    "timeout": cfg.timeout_secs,
                    "has_key": cfg.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
                    "can_undo": is_undo_available,
                    "has_pending": has_pending
                });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                return;
            }
            "POST" => {
                let body_str = String::from_utf8_lossy(&req.body);
                if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let mut cfg_guard = ctx.llm_config.lock().unwrap();
                    if let Some(p) = json_body.get("provider").and_then(|v| v.as_str()) {
                        cfg_guard.provider = match p.to_lowercase().as_str() {
                            "openai" | "mimocode" | "compatible" => LlmProvider::OpenAiCompatible,
                            _ => LlmProvider::Ollama,
                        };
                    }
                    if let Some(ep) = json_body.get("endpoint").and_then(|v| v.as_str()) {
                        cfg_guard.endpoint_url = ep.to_string();
                    }
                    if let Some(m) = json_body.get("model").and_then(|v| v.as_str()) {
                        cfg_guard.model = m.to_string();
                    }
                    if let Some(k) = json_body.get("api_key").and_then(|v| v.as_str()) {
                        cfg_guard.api_key = Some(k.to_string());
                    }
                    if let Some(t) = json_body.get("timeout").and_then(|v| v.as_u64()) {
                        cfg_guard.timeout_secs = t;
                    }

                    let new_client = LlmClient::new(cfg_guard.clone());
                    let perm_mgr = PermissionManager::new(ctx.perm_config.lock().unwrap().clone());
                    ctx.session.lock().unwrap().update_config(perm_mgr, new_client);

                    let resp = json!({ "ok": true });
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
                    return;
                }
            }
            _ => {}
        }
    }

    // POST /api/ai/chat -> SSE Streaming endpoint
    if clean_path == "api/ai/chat" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
        let prompt = json_body.get("prompt").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let context_filename = json_body.get("context_filename").and_then(|v| v.as_str()).map(|s| s.to_string());
        let context_content = json_body.get("context_content").and_then(|v| v.as_str()).map(|s| s.to_string());

        send_sse_header(&mut stream);

        let mut session_guard = ctx.session.lock().unwrap();
        if let Some(ref fname) = context_filename {
            let content = context_content.unwrap_or_else(|| {
                fs::read_to_string(ctx.notes_dir.join(fname)).unwrap_or_default()
            });
            session_guard.reset_session(Some((fname.as_str(), &content)), None);
        }

        let stream_mutex = Arc::new(Mutex::new(stream));
        let stream_for_tokens = stream_mutex.clone();

        let step_result = session_guard.send_prompt_streaming(&prompt, move |token| {
            if let Ok(mut s) = stream_for_tokens.lock() {
                send_sse_event(&mut s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo(),
                        "last_created_note": session_guard.last_created_note()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut s, &json!({
                        "type": "pending_confirmation",
                        "action": {
                            "tool_name": pending.tool_name,
                            "filename": pending.filename,
                            "reason": pending.reason,
                            "diff": pending.diff.lines.iter().map(|l| json!({
                                "diff_type": format!("{:?}", l.line_type).to_lowercase(),
                                "text": l.content
                            })).collect::<Vec<_>>()
                        }
                    }));
                }
                AgentStepResult::Error(err) => {
                    send_sse_event(&mut s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut s);
        }
        return;
    }

    // POST /api/ai/template -> Quick action runner
    if clean_path == "api/ai/template" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
        let template_id = json_body.get("template_id").and_then(|v| v.as_str()).unwrap_or("summarize");
        let content = json_body.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let context_filename = json_body.get("context_filename").and_then(|v| v.as_str()).unwrap_or("note.adoc");

        let instruction = build_template_instruction(template_id, "", Some(context_filename), Some(content));

        send_sse_header(&mut stream);
        let mut session_guard = ctx.session.lock().unwrap();
        session_guard.reset_session(Some((context_filename, content)), None);

        let stream_mutex = Arc::new(Mutex::new(stream));
        let stream_for_tokens = stream_mutex.clone();

        let step_result = session_guard.send_prompt_streaming(&instruction, move |token| {
            if let Ok(mut s) = stream_for_tokens.lock() {
                send_sse_event(&mut s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut s, &json!({
                        "type": "pending_confirmation",
                        "action": {
                            "tool_name": pending.tool_name,
                            "filename": pending.filename,
                            "reason": pending.reason,
                            "diff": pending.diff.lines.iter().map(|l| json!({
                                "diff_type": format!("{:?}", l.line_type).to_lowercase(),
                                "text": l.content
                            })).collect::<Vec<_>>()
                        }
                    }));
                }
                AgentStepResult::Error(err) => {
                    send_sse_event(&mut s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut s);
        }
        return;
    }

    // POST /api/ai/confirm -> Tool call confirmation
    if clean_path == "api/ai/confirm" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
        let approved = json_body.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);

        send_sse_header(&mut stream);
        let mut session_guard = ctx.session.lock().unwrap();

        let stream_mutex = Arc::new(Mutex::new(stream));
        let stream_for_tokens = stream_mutex.clone();

        let step_result = session_guard.confirm_pending_action_streaming(approved, move |token| {
            if let Ok(mut s) = stream_for_tokens.lock() {
                send_sse_event(&mut s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut s, &json!({
                        "type": "pending_confirmation",
                        "action": {
                            "tool_name": pending.tool_name,
                            "filename": pending.filename,
                            "reason": pending.reason,
                            "diff": pending.diff.lines.iter().map(|l| json!({
                                "diff_type": format!("{:?}", l.line_type).to_lowercase(),
                                "text": l.content
                            })).collect::<Vec<_>>()
                        }
                    }));
                }
                AgentStepResult::Error(err) => {
                    send_sse_event(&mut s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut s);
        }
        return;
    }

    // POST /api/ai/undo -> Revert last AI snapshot
    if clean_path == "api/ai/undo" && req.method == "POST" {
        let mut session_guard = ctx.session.lock().unwrap();
        match session_guard.undo_last_action() {
            Ok(msg) => {
                let resp = json!({ "ok": true, "message": msg, "can_undo": session_guard.can_undo() });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes());
            }
            Err(err) => {
                let resp = json!({ "ok": false, "error": err, "can_undo": session_guard.can_undo() });
                send_response(&mut stream, 400, "Bad Request", "application/json; charset=utf-8", resp.to_string().as_bytes());
            }
        }
        return;
    }

    // 5. Page URLs: /page/{name}, /notes/{name}, /edit/{name}
    if clean_path.starts_with("page/") || clean_path.starts_with("notes/") || clean_path.starts_with("edit/") {
        let note_name = clean_path
            .strip_prefix("page/")
            .or_else(|| clean_path.strip_prefix("notes/"))
            .or_else(|| clean_path.strip_prefix("edit/"))
            .unwrap_or("");

        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else {
            format!("{}.adoc", note_name)
        };

        // If explicitly requested standalone export/view
        if let Some(q) = req.query.as_deref() {
            if q.contains("export=1") || q.contains("download=1") {
                let file_path = ctx.notes_dir.join(&filename);
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                        let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
                        send_attachment_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &format!("{}.html", title));
                        return;
                    }
                }
            } else if q.contains("view=rendered") {
                let file_path = ctx.notes_dir.join(&filename);
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                        let html = render_web_page_html(&content, title, &ctx.notes_dir, &filename);
                        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes());
                        return;
                    }
                }
            }
        }

        // Default: serve the Vue 3 interactive editor app (app router loads the note)
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes());
        return;
    }

    // 6. Raw Note text endpoint
    if clean_path.starts_with("raw/") {
        let note_name = clean_path.strip_prefix("raw/").unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else {
            format!("{}.adoc", note_name)
        };

        let file_path = ctx.notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                send_response(&mut stream, 200, "OK", "text/plain; charset=utf-8", content.as_bytes());
                return;
            }
        }
    }

    // 7. Standalone HTML5 export download
    if clean_path.starts_with("export/") {
        let note_name = clean_path.strip_prefix("export/").unwrap_or("");
        let filename = if note_name.ends_with(".adoc") {
            note_name.to_string()
        } else if note_name.ends_with(".html") {
            format!("{}.adoc", note_name.strip_suffix(".html").unwrap_or(note_name))
        } else {
            format!("{}.adoc", note_name)
        };

        let file_path = ctx.notes_dir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
                send_attachment_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &format!("{}.html", title));
                return;
            }
        }
    }

    // 8. Static asset from notes directory (images, svgs, stylesheets, etc.)
    let asset_path = if clean_path.starts_with("assets/") {
        ctx.notes_dir.join(clean_path.strip_prefix("assets/").unwrap_or(clean_path))
    } else {
        ctx.notes_dir.join(clean_path)
    };

    if asset_path.is_file() {
        if let Ok(bytes) = fs::read(&asset_path) {
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

    send_response(&mut stream, 404, "Not Found", "text/html; charset=utf-8", b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Fishdoc Editor</a></p>");
}

fn send_response(stream: &mut TcpStream, status_code: u16, status_text: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\n\r\n",
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
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=\"{}\"\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\n\r\n",
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

fn send_sse_header(stream: &mut TcpStream) {
    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\n\r\n";
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.flush();
}

fn send_sse_event(stream: &mut TcpStream, data: &serde_json::Value) {
    let payload = format!("data: {}\n\n", data);
    let _ = stream.write_all(payload.as_bytes());
    let _ = stream.flush();
}

fn send_sse_done(stream: &mut TcpStream) {
    let _ = stream.write_all(b"data: [DONE]\n\n");
    let _ = stream.flush();
}

fn list_all_notes_json(notes_dir: &Path, search_query: Option<&str>) -> String {
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(notes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let mut title = name.strip_suffix(".adoc").unwrap_or(name).to_string();
                    let mut snippet = String::new();
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("= ") {
                                title = trimmed.trim_start_matches("= ").trim().to_string();
                            } else if snippet.is_empty() && !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with(':') {
                                snippet = trimmed.chars().take(120).collect();
                            }
                        }
                    }
                    notes.push((title, name.to_string(), snippet));
                }
            }
        }
    }

    notes.sort_by_key(|a| a.0.to_lowercase());

    let query_str = search_query.unwrap_or("").trim().to_lowercase();
    let filtered: Vec<_> = if query_str.is_empty() {
        notes
    } else {
        notes
            .into_iter()
            .filter(|(t, fn_name, snip)| {
                t.to_lowercase().contains(&query_str)
                    || fn_name.to_lowercase().contains(&query_str)
                    || snip.to_lowercase().contains(&query_str)
            })
            .collect()
    };

    let json_items: Vec<_> = filtered
        .into_iter()
        .map(|(t, fn_name, snip)| {
            json!({
                "title": t,
                "filename": fn_name,
                "snippet": snip
            })
        })
        .collect();

    serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string())
}

fn make_slug_filename(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-');
    let final_slug = if trimmed.is_empty() { "untitled" } else { trimmed };
    format!("{}.adoc", final_slug)
}

fn extract_title_from_adoc(content: &str, fallback_filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("= ") {
            return trimmed.trim_start_matches("= ").trim().to_string();
        }
    }
    fallback_filename.trim_end_matches(".adoc").replace('_', " ")
}

fn render_web_page_html(adoc_content: &str, title: &str, notes_dir: &Path, filename: &str) -> String {
    let standalone = adoc_to_html5(adoc_content, title, Some(notes_dir));

    let top_bar = format!(
        r#"<div class="web-page-topbar">
            <div class="topbar-left">
                <a class="nav-btn" href="/">&larr; Fishdoc Web Editor</a>
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

    if let Ok(arp) = fs::read_to_string("/proc/net/arp") {
        for line in arp.lines().skip(1) {
            if let Some(ip) = line.split_whitespace().next() {
                if ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.") {
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
    let mut result = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = format!("{}{}", c1, c2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte);
                    continue;
                }
            }
            result.push(b'%');
        } else if ch == '+' {
            result.push(b' ');
        } else {
            result.extend_from_slice(ch.to_string().as_bytes());
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_make_slug_filename() {
        assert_eq!(make_slug_filename("My Note!"), "my-note.adoc");
        assert_eq!(make_slug_filename("  "), "untitled.adoc");
        assert_eq!(make_slug_filename("Hello World 123"), "hello-world-123.adoc");
    }

    #[test]
    fn test_list_all_notes_json() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();

        fs::write(notes_dir.join("alpha.adoc"), "= Alpha Note\nFirst line of content.").unwrap();
        fs::write(notes_dir.join("beta.adoc"), "= Beta Note\nSecond line of content.").unwrap();

        let json_str = list_all_notes_json(&notes_dir, None);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_server_lifecycle_and_endpoints() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        fs::write(notes_dir.join("welcome.adoc"), "= Welcome\nTest content for server.").unwrap();

        let server_handle = start_server_full(
            notes_dir.clone(),
            db_path,
            backup_dir,
            18920,
            None,
            None,
        ).expect("Server should start");

        assert!(server_handle.is_running());
        let port = server_handle.port();
        assert!(port >= 18920);

        // Test GET /
        let res_root = ureq::get(&format!("http://127.0.0.1:{}/", port)).call().unwrap();
        assert_eq!(res_root.status(), 200);
        let root_body = res_root.into_string().unwrap();
        assert!(root_body.contains("Fishdoc Web"));

        // Test GET /app.js
        let res_js = ureq::get(&format!("http://127.0.0.1:{}/app.js", port)).call().unwrap();
        assert_eq!(res_js.status(), 200);
        let js_body = res_js.into_string().unwrap();
        assert!(js_body.contains("createApp"));

        // Test GET /style.css
        let res_css = ureq::get(&format!("http://127.0.0.1:{}/style.css", port)).call().unwrap();
        assert_eq!(res_css.status(), 200);

        // Test GET /api/notes
        let res_notes = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port)).call().unwrap();
        assert_eq!(res_notes.status(), 200);
        let notes_json: serde_json::Value = res_notes.into_json().unwrap();
        assert!(!notes_json.as_array().unwrap().is_empty());

        // Test GET /api/notes/welcome.adoc
        let res_note = ureq::get(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port)).call().unwrap();
        assert_eq!(res_note.status(), 200);
        assert!(res_note.into_string().unwrap().contains("Test content"));

        // Test PUT /api/notes/welcome.adoc
        let put_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port))
            .set("Content-Type", "text/plain")
            .send_string("= Welcome\nUpdated content from PUT test.")
            .unwrap();
        assert_eq!(put_res.status(), 200);
        let updated_file = fs::read_to_string(notes_dir.join("welcome.adoc")).unwrap();
        assert!(updated_file.contains("Updated content from PUT test"));

        // Test POST /api/notes (create new)
        let create_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes", port))
            .send_json(json!({
                "title": "New Doc",
                "content": "= New Doc\nCreated via API"
            }))
            .unwrap();
        assert_eq!(create_res.status(), 200);
        assert!(notes_dir.join("new-doc.adoc").exists());

        // Test POST /api/render
        let render_res = ureq::post(&format!("http://127.0.0.1:{}/api/render", port))
            .send_json(json!({
                "content": "= Header\n* Bullet item\n"
            }))
            .unwrap();
        assert_eq!(render_res.status(), 200);
        let render_html = render_res.into_string().unwrap();
        assert!(render_html.contains("Header") && render_html.contains("Bullet item"));

        // Test POST /api/blocks/parse
        let parse_res = ureq::post(&format!("http://127.0.0.1:{}/api/blocks/parse", port))
            .send_json(json!({
                "content": "= Heading 1\n\nParagraph text\n\n* [ ] Task 1"
            }))
            .unwrap();
        assert_eq!(parse_res.status(), 200);
        let parse_json: serde_json::Value = parse_res.into_json().unwrap();
        assert!(parse_json.get("blocks").and_then(|b| b.as_array()).unwrap().len() >= 3);

        // Test POST /api/notes/welcome.adoc/toggle (checklist toggle)
        fs::write(notes_dir.join("welcome.adoc"), "= Tasks\n* [ ] Task 1\n* [x] Task 2").unwrap();
        let toggle_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc/toggle", port))
            .send_json(json!({
                "item_index": 0,
                "checked": true
            }))
            .unwrap();
        assert_eq!(toggle_res.status(), 200);
        let toggled_file = fs::read_to_string(notes_dir.join("welcome.adoc")).unwrap();
        assert!(toggled_file.contains("* [x] Task 1"));

        // Test POST /api/ai/config (update config)
        let ai_update_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/config", port))
            .send_json(json!({
                "provider": "ollama",
                "endpoint": "http://127.0.0.1:11434",
                "model": "llama3.2",
                "timeout": 45
            }))
            .unwrap();
        assert_eq!(ai_update_res.status(), 200);

        // Verify updated config
        let ai_cfg_res2 = ureq::get(&format!("http://127.0.0.1:{}/api/ai/config", port)).call().unwrap();
        let ai_cfg_json2: serde_json::Value = ai_cfg_res2.into_json().unwrap();
        assert_eq!(ai_cfg_json2.get("model").unwrap(), "llama3.2");
        assert_eq!(ai_cfg_json2.get("timeout").unwrap(), 45);

        // Test GET /raw/welcome.adoc
        let raw_res = ureq::get(&format!("http://127.0.0.1:{}/raw/welcome.adoc", port)).call().unwrap();
        assert_eq!(raw_res.status(), 200);
        assert!(raw_res.into_string().unwrap().contains("Task"));

        // Test GET /export/welcome.adoc
        let export_res = ureq::get(&format!("http://127.0.0.1:{}/export/welcome.adoc", port)).call().unwrap();
        assert_eq!(export_res.status(), 200);
        assert!(export_res.into_string().unwrap().contains("html"));

        // Test DELETE /api/notes/new-doc.adoc
        let del_res = ureq::delete(&format!("http://127.0.0.1:{}/api/notes/new-doc.adoc", port)).call().unwrap();
        assert_eq!(del_res.status(), 200);
        assert!(!notes_dir.join("new-doc.adoc").exists());

        // Stop server
        server_handle.stop();
    }
}
