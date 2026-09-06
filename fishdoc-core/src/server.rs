//! Embedded HTTP server and REST / SSE API for Fishdoc Web Editor & AI Assistant.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use rusqlite::Connection;
use serde_json::json;

/// Strip characters that could inject HTTP headers (quotes, CR, LF).
fn sanitize_header_value(s: &str) -> String {
    s.chars().filter(|c| *c != '"' && *c != '\r' && *c != '\n').collect()
}

use crate::agent::{
    build_template_instruction, AgentSession, AgentStepResult, LlmClient, LlmConfig, LlmProvider,
    PermissionConfig, PermissionManager,
};
use crate::block::Block;
use crate::db;
use crate::html::{adoc_to_html5, adoc_to_html_body, blocks_to_html_body};
use crate::page;
use crate::parser;

pub mod auth;
pub mod tls;
pub mod web_assets;
use web_assets::{APP_JS, INDEX_HTML, STYLE_CSS};

/// Server configuration holding paths, networking, security, and AI client parameters.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub notes_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub port: u16,
    pub llm_config: LlmConfig,
    pub permission_config: PermissionConfig,
    pub auth_config: auth::AuthConfig,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
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
            auth_config: auth::AuthConfig::default(),
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
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
    pub auth_config: Arc<Mutex<auth::AuthConfig>>,
    pub session_store: auth::SessionStore,
    pub oidc_flow_mgr: auth::OidcFlowManager,
    pub tls_status: Arc<Mutex<Option<tls::TlsStatusInfo>>>,
    pub is_tls: bool,
}

impl ServerContext {
    pub fn new(config: ServerConfig) -> Self {
        Self::new_with_tls(config, false)
    }

    pub fn new_with_tls(config: ServerConfig, is_tls: bool) -> Self {
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
            auth_config: Arc::new(Mutex::new(config.auth_config)),
            session_store: auth::SessionStore::new(),
            oidc_flow_mgr: auth::OidcFlowManager::new(),
            tls_status: Arc::new(Mutex::new(None)),
            is_tls,
        }
    }

    /// Updates the active LLM client and permission configurations.
    pub fn update_llm_config(&self, config: LlmConfig, perm_config: Option<PermissionConfig>) {
        let mut cfg_guard = self.llm_config.lock().unwrap_or_else(|e| e.into_inner());
        *cfg_guard = config.clone();
        if let Some(p) = perm_config {
            let mut perm_guard = self.perm_config.lock().unwrap_or_else(|e| e.into_inner());
            *perm_guard = p;
        }
        let new_client = LlmClient::new(config);
        let perm_mgr = PermissionManager::new(self.perm_config.lock().unwrap_or_else(|e| e.into_inner()).clone());
        self.session.lock().unwrap_or_else(|e| e.into_inner()).update_config(perm_mgr, new_client);
    }

    /// Updates the active authentication configuration.
    pub fn update_auth_config(&self, config: auth::AuthConfig) {
        let mut cfg_guard = self.auth_config.lock().unwrap_or_else(|e| e.into_inner());
        *cfg_guard = config;
    }

    /// Updates TLS status information.
    pub fn update_tls_status(&self, status: Option<tls::TlsStatusInfo>) {
        let mut s_guard = self.tls_status.lock().unwrap_or_else(|e| e.into_inner());
        *s_guard = status;
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
            .unwrap_or_else(|| {
                let scheme = if self.context.is_tls { "https" } else { "http" };
                format!("{}://127.0.0.1:{}", scheme, self.port)
            })
    }

    pub fn is_tls(&self) -> bool {
        self.context.is_tls
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
    let db_path = notes_path.parent().unwrap_or(&notes_path).join("fishdoc.db");
    let backup_dir = notes_path.parent().unwrap_or(&notes_path).join("backups");
    let config = ServerConfig {
        notes_dir: notes_path,
        db_path,
        backup_dir,
        port: requested_port,
        ..Default::default()
    };
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
        auth_config: auth::AuthConfig::default(),
        enable_tls: false,
        tls_cert_path: None,
        tls_key_path: None,
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

    let scheme = if config.enable_tls { "https" } else { "http" };
    let local_ips = get_local_ip_addresses();
    let mut local_urls = Vec::new();
    for ip in &local_ips {
        local_urls.push(format!("{}://{}:{}", scheme, ip, actual_port));
    }
    if local_urls.is_empty() {
        local_urls.push(format!("{}://127.0.0.1:{}", scheme, actual_port));
    }

    let rustls_config = if config.enable_tls {
        let parent_dir = config.notes_dir.parent().unwrap_or(&config.notes_dir);
        let cert_dir = parent_dir.join("tls");
        let cert_path = config
            .tls_cert_path
            .clone()
            .unwrap_or_else(|| cert_dir.join("server.crt"));
        let key_path = config
            .tls_key_path
            .clone()
            .unwrap_or_else(|| cert_dir.join("server.key"));

        let cert = tls::get_or_create_tls_cert(&cert_path, &key_path, None)?;
        let r_cfg = tls::create_rustls_server_config(&cert)?;
        Some((r_cfg, cert_path, key_path))
    } else {
        None
    };

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    let is_tls = rustls_config.is_some();
    let r_cfg_arc = rustls_config.as_ref().map(|(c, _, _)| c.clone());

    let context = ServerContext::new_with_tls(config, is_tls);
    if let Some((_, cert_p, key_p)) = rustls_config {
        context.update_tls_status(Some(tls::TlsStatusInfo {
            is_tls: true,
            is_custom: tls::is_custom_cert_installed(&cert_p),
            cert_path: cert_p.to_string_lossy().to_string(),
            key_path: key_p.to_string_lossy().to_string(),
            subject: "Fishdoc Web Server".to_string(),
        }));
    }

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
                    if let Some(ref r_cfg) = r_cfg_arc {
                        if let Ok(conn) = rustls::ServerConnection::new(r_cfg.clone()) {
                            let tls_stream = Box::new(rustls::StreamOwned::new(conn, stream));
                            thread::spawn(move || {
                                handle_http_client(StreamWrapper::Tls(tls_stream), ctx);
                            });
                        }
                    } else {
                        thread::spawn(move || {
                            handle_http_client(StreamWrapper::Plain(stream), ctx);
                        });
                    }
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

/// Unified stream wrapper for plain TCP or TLS encrypted connections.
pub enum StreamWrapper {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ServerConnection, TcpStream>>),
}

impl Read for StreamWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            StreamWrapper::Plain(s) => s.read(buf),
            StreamWrapper::Tls(s) => s.read(buf),
        }
    }
}

impl Write for StreamWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            StreamWrapper::Plain(s) => s.write(buf),
            StreamWrapper::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            StreamWrapper::Plain(s) => s.flush(),
            StreamWrapper::Tls(s) => s.flush(),
        }
    }
}

impl StreamWrapper {
    pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> std::io::Result<()> {
        match self {
            StreamWrapper::Plain(s) => s.set_read_timeout(timeout),
            StreamWrapper::Tls(s) => s.sock.set_read_timeout(timeout),
        }
    }

    pub fn set_write_timeout(&self, timeout: Option<std::time::Duration>) -> std::io::Result<()> {
        match self {
            StreamWrapper::Plain(s) => s.set_write_timeout(timeout),
            StreamWrapper::Tls(s) => s.sock.set_write_timeout(timeout),
        }
    }
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    query: Option<String>,
    _headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn parse_http_request(stream: &mut StreamWrapper) -> Result<ParsedHttpRequest, String> {
    let mut header_bytes = Vec::new();
    let mut one_byte = [0u8; 1];

    loop {
        let n = stream.read(&mut one_byte).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Unexpected EOF while reading request headers".to_string());
        }
        header_bytes.push(one_byte[0]);
        if header_bytes.ends_with(b"\r\n\r\n") || header_bytes.ends_with(b"\n\n") {
            break;
        }
        if header_bytes.len() > 65536 {
            return Err("Request headers too large".to_string());
        }
    }

    let header_str = String::from_utf8_lossy(&header_bytes);
    let mut lines = header_str.lines();
    let request_line = lines.next().ok_or_else(|| "Empty request".to_string())?;

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
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    if content_length > MAX_BODY_SIZE {
        return Err(format!(
            "Request body too large: {} bytes (max {})",
            content_length, MAX_BODY_SIZE
        ));
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        stream.read_exact(&mut body).map_err(|e| e.to_string())?;
    }

    Ok(ParsedHttpRequest {
        method,
        path,
        query,
        _headers: headers,
        body,
    })
}


/// Validate CORS origin against localhost variants and known LAN IPs.
/// Returns the origin string if allowed, empty string otherwise.
fn validate_cors_origin(origin: Option<&str>, allowed_ips: &[String]) -> String {
    let origin = match origin {
        Some(o) => o,
        None => return String::new(),
    };

    // Extract host from origin URL (e.g., "http://localhost:3000" -> "localhost")
    let host = origin
        .split("://")
        .nth(1)
        .unwrap_or(origin)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();

    // Allow localhost variants
    if host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1" {
        return origin.to_string();
    }

    // Allow IPs from the local interface list
    if allowed_ips.contains(&host) {
        return origin.to_string();
    }

    // Allow private LAN ranges
    if host.starts_with("192.168.") || host.starts_with("10.") {
        return origin.to_string();
    }
    // 172.16.0.0/12
    if host.starts_with("172.") {
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() >= 2 {
            if let Ok(second) = parts[1].parse::<u8>() {
                if (16..=31).contains(&second) {
                    return origin.to_string();
                }
            }
        }
    }

    String::new()
}

/// Build CORS header lines when origin is non-empty, empty string otherwise.
fn build_cors_headers(origin: &str) -> String {
    if origin.is_empty() {
        return String::new();
    }
    format!(
        "Access-Control-Allow-Origin: {}\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization, Cookie\r\n",
        origin
    )
}

fn make_session_cookie(session_id: &str, is_tls: bool, max_age: Option<u64>) -> String {
    let mut cookie = format!("fishdoc_session={}; Path=/; HttpOnly; SameSite=Lax", session_id);
    if let Some(age) = max_age {
        cookie.push_str(&format!("; Max-Age={}", age));
    }
    if is_tls {
        cookie.push_str("; Secure");
    }
    cookie
}

fn extract_cookie_value(cookie_header: &str, key: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn authenticate_request(
    req: &ParsedHttpRequest,
    auth_config: &auth::AuthConfig,
    session_store: &auth::SessionStore,
) -> Option<auth::Session> {
    if !auth_config.enabled {
        return Some(auth::Session {
            id: "anonymous".to_string(),
            user: "anonymous".to_string(),
            auth_method: "none".to_string(),
            created_at: 0,
            expires_at: u64::MAX,
        });
    }

    // 1. Check Session Cookie
    if let Some(cookie_hdr) = req._headers.get("cookie") {
        if let Some(session_id) = extract_cookie_value(cookie_hdr, "fishdoc_session") {
            if let Some(session) = session_store.validate_session(&session_id) {
                return Some(session);
            }
        }
    }

    // 2. Check HTTP Basic Authorization header
    if let Some(auth_hdr) = req._headers.get("authorization") {
        if let Some(user) = auth_config.verify_basic_auth_header(auth_hdr) {
            return Some(auth::Session {
                id: "basic".to_string(),
                user,
                auth_method: "basic".to_string(),
                created_at: 0,
                expires_at: u64::MAX,
            });
        }
    }

    None
}

fn handle_http_client(mut stream: StreamWrapper, ctx: ServerContext) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(120)));

    let req = match parse_http_request(&mut stream) {
        Ok(r) => r,
        Err(_) => {
            send_response(&mut stream, 400, "Bad Request", "text/plain", b"Bad Request", "");
            return;
        }
    };

    let local_ips = get_local_ip_addresses();
    let cors_origin = validate_cors_origin(req._headers.get("origin").map(|s| s.as_str()), &local_ips);

    // Handle CORS preflight requests
    if req.method == "OPTIONS" {
        send_response(&mut stream, 200, "OK", "text/plain", b"", &cors_origin);
        return;
    }

    let clean_path = req.path.trim_start_matches('/');

    // Reject path traversal attempts
    if clean_path.contains("..") {
        send_response(&mut stream, 403, "Forbidden", "text/plain", b"Forbidden", &cors_origin);
        return;
    }

    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let session_store = ctx.session_store.clone();
    let oidc_flow_mgr = ctx.oidc_flow_mgr.clone();

    // Health check & Ping
    if clean_path == "api/ping" {
        let resp = json!({
            "ok": true,
            "is_tls": ctx.is_tls,
            "auth_enabled": auth_config.enabled,
        });
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    // Public / Auth endpoints
    if clean_path == "api/auth/config" {
        match req.method.as_str() {
            "GET" => {
                let session = authenticate_request(&req, &auth_config, &session_store);
                let resp = json!({
                    "auth_required": auth_config.enabled,
                    "basic_enabled": auth_config.basic_enabled,
                    "basic_username": auth_config.basic_username,
                    "has_password": !auth_config.basic_password_hash.is_empty(),
                    "oauth_enabled": auth_config.oauth_enabled,
                    "oauth_provider_name": auth_config.oauth_provider_name,
                    "oauth_issuer_url": auth_config.oauth_issuer_url,
                    "oauth_client_id": auth_config.oauth_client_id,
                    "oauth_allowed_emails": auth_config.oauth_allowed_emails,
                    "allow_self_signed_oidc": auth_config.allow_self_signed_oidc,
                    "authenticated": session.is_some(),
                    "user": session.as_ref().map(|s| s.user.clone()).unwrap_or_default()
                });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
                return;
            }
            "POST" => {
                if auth_config.enabled {
                    let session = authenticate_request(&req, &auth_config, &session_store);
                    if session.is_none() {
                        let err = json!({ "ok": false, "error": "Unauthorized" });
                        send_response(&mut stream, 401, "Unauthorized", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
                        return;
                    }
                }
                if let Ok(mut new_cfg) = serde_json::from_slice::<auth::AuthConfig>(&req.body) {
                    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                        if let Some(pass) = val.get("password").and_then(|v| v.as_str()) {
                            if !pass.is_empty() {
                                let _ = new_cfg.set_password(pass);
                            }
                        }
                    }
                    ctx.update_auth_config(new_cfg);
                    let resp = json!({ "ok": true });
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
                    return;
                }
                let err = json!({ "ok": false, "error": "Invalid auth config payload" });
                send_response(&mut stream, 400, "Bad Request", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
                return;
            }
            _ => {}
        }
    }

    if clean_path == "api/auth/login" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
        let username = json_body.get("username").and_then(|v| v.as_str()).unwrap_or("");
        let password = json_body.get("password").and_then(|v| v.as_str()).unwrap_or("");

        if auth_config.verify_basic_credentials(username, password) {
            let ttl_secs = 30 * 24 * 3600; // 30 days
            if let Ok(sess) = session_store.create_session(username, "basic", ttl_secs) {
                let cookie_str = make_session_cookie(&sess.id, ctx.is_tls, Some(ttl_secs));
                let resp = json!({
                    "ok": true,
                    "session_id": sess.id,
                    "user": sess.user
                });
                send_response_full(
                    &mut stream,
                    200,
                    "OK",
                    "application/json; charset=utf-8",
                    resp.to_string().as_bytes(),
                    &cors_origin,
                    &[("Set-Cookie", &cookie_str)],
                );
                return;
            }
        }
        let resp = json!({ "ok": false, "error": "Invalid username or password" });
        send_response(&mut stream, 401, "Unauthorized", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    if clean_path == "api/auth/logout" && req.method == "POST" {
        if let Some(cookie_hdr) = req._headers.get("cookie") {
            if let Some(session_id) = extract_cookie_value(cookie_hdr, "fishdoc_session") {
                session_store.remove_session(&session_id);
            }
        }
        let cookie_str = make_session_cookie("", ctx.is_tls, Some(0));
        let resp = json!({ "ok": true });
        send_response_full(
            &mut stream,
            200,
            "OK",
            "application/json; charset=utf-8",
            resp.to_string().as_bytes(),
            &cors_origin,
            &[("Set-Cookie", &cookie_str)],
        );
        return;
    }

    if clean_path == "api/auth/whoami" && req.method == "GET" {
        let session = authenticate_request(&req, &auth_config, &session_store);
        let resp = json!({
            "authenticated": session.is_some(),
            "user": session.as_ref().map(|s| s.user.clone()),
            "auth_type": session.as_ref().map(|s| s.auth_method.clone()),
            "auth_enabled": auth_config.enabled,
            "basic_enabled": auth_config.basic_enabled,
            "oauth_enabled": auth_config.oauth_enabled,
            "oauth_provider_name": auth_config.oauth_provider_name,
        });
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    // OAuth / OIDC endpoints
    if (clean_path == "api/auth/oauth/start" || clean_path == "api/auth/oauth/login")
        && (req.method == "GET" || req.method == "POST")
    {
        let host = req._headers.get("host").map(|s| s.as_str()).unwrap_or("127.0.0.1");
        let scheme = if ctx.is_tls { "https" } else { "http" };
        let default_redirect = format!("{}://{}/api/auth/oauth/callback", scheme, host);

        match auth::build_oidc_authorization_url(&auth_config, &oidc_flow_mgr, Some(&default_redirect)) {
            Ok(auth_url) => {
                if clean_path == "api/auth/oauth/login" {
                    send_redirect(&mut stream, &auth_url, &cors_origin, &[]);
                } else {
                    let resp = json!({ "ok": true, "auth_url": auth_url });
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
                }
                return;
            }
            Err(e) => {
                let err = json!({ "ok": false, "error": e });
                send_response(&mut stream, 500, "Internal Server Error", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
                return;
            }
        }
    }

    if clean_path == "api/auth/oauth/callback" {
        let host = req._headers.get("host").map(|s| s.as_str()).unwrap_or("127.0.0.1");
        let scheme = if ctx.is_tls { "https" } else { "http" };
        let default_redirect = format!("{}://{}/api/auth/oauth/callback", scheme, host);

        let (code, state) = if let Some(ref q) = req.query {
            let mut c = None;
            let mut s = None;
            for pair in q.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    if k == "code" { c = Some(v.to_string()); }
                    if k == "state" { s = Some(v.to_string()); }
                }
            }
            (c, s)
        } else {
            (None, None)
        };

        if let (Some(code), Some(state)) = (code, state) {
            match auth::handle_oidc_callback(&auth_config, &oidc_flow_mgr, &code, &state, Some(&default_redirect)) {
                Ok(user) => {
                    let ttl_secs = 30 * 24 * 3600;
                    if let Ok(sess) = session_store.create_session(&user, "oauth", ttl_secs) {
                        let cookie_str = make_session_cookie(&sess.id, ctx.is_tls, Some(ttl_secs));
                        send_redirect(&mut stream, "/", &cors_origin, &[("Set-Cookie", &cookie_str)]);
                        return;
                    }
                }
                Err(e) => {
                    let err_html = format!("<h1>OAuth Login Failed</h1><p>{}</p><p><a href=\"/\">Back to login</a></p>", escape_html(&e));
                    send_response(&mut stream, 403, "Forbidden", "text/html; charset=utf-8", err_html.as_bytes(), &cors_origin);
                    return;
                }
            }
        }
        let err_html = "<h1>OAuth Login Failed</h1><p>Missing code or state parameters.</p><p><a href=\"/\">Back to login</a></p>";
        send_response(&mut stream, 400, "Bad Request", "text/html; charset=utf-8", err_html.as_bytes(), &cors_origin);
        return;
    }

    // Route Protection Middleware: Protect all data/AI routes when authentication is enabled
    let is_public_path = clean_path.is_empty()
        || clean_path == "index.html"
        || clean_path == "app.js"
        || clean_path == "style.css"
        || clean_path == "favicon.ico"
        || clean_path.starts_with("assets/")
        || clean_path.starts_with("api/auth/");

    if auth_config.enabled && !is_public_path {
        let session = authenticate_request(&req, &auth_config, &session_store);
        if session.is_none() {
            if clean_path.starts_with("api/") {
                let err = json!({ "error": "Unauthorized", "authenticated": false });
                send_response(&mut stream, 401, "Unauthorized", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
                return;
            } else {
                send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes(), &cors_origin);
                return;
            }
        }
    }

    // 1. Static Web Application Root & Assets
    if req.method == "GET" && (clean_path.is_empty() || clean_path == "index.html") {
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes(), &cors_origin);
        return;
    }

    if req.method == "GET" && clean_path == "app.js" {
        send_response(&mut stream, 200, "OK", "application/javascript; charset=utf-8", APP_JS.as_bytes(), &cors_origin);
        return;
    }

    if req.method == "GET" && clean_path == "style.css" {
        send_response(&mut stream, 200, "OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes(), &cors_origin);
        return;
    }

    // 2. REST API: Notes Management
    if clean_path == "api/notes" {
        match req.method.as_str() {
            "GET" => {
                let list = list_all_notes_json(&ctx.notes_dir, req.query.as_deref());
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", list.as_bytes(), &cors_origin);
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
                    send_response(&mut stream, 500, "Internal Server Error", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
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
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
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
                        send_response(&mut stream, 200, "OK", "text/plain; charset=utf-8", content.as_bytes(), &cors_origin);
                        return;
                    }
                }
                send_response(&mut stream, 404, "Not Found", "application/json; charset=utf-8", b"{\"error\":\"Note not found\"}", &cors_origin);
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
                    send_response(&mut stream, 500, "Internal Server Error", "application/json; charset=utf-8", err.to_string().as_bytes(), &cors_origin);
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
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
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
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
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
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
                    return;
                }
            }
        }
        send_response(&mut stream, 404, "Not Found", "application/json; charset=utf-8", b"{\"error\":\"Note or checklist item not found\"}", &cors_origin);
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
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &cors_origin);
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
                let html = blocks_to_html_body(std::slice::from_ref(b), Some(&ctx.notes_dir));
                json!({
                    "index": i,
                    "raw": b.raw_text(),
                    "html": html
                })
            })
            .collect();

        let resp = json!({ "blocks": block_items });
        send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    // 4b. REST API: List available models from configured LLM server
    if clean_path == "api/ai/models" && req.method == "GET" {
        let cfg = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
        let client = LlmClient::new(cfg.clone());
        drop(cfg);

        match client.list_models() {
            Ok(models) => {
                let resp = json!({ "models": models });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
            }
            Err(e) => {
                let resp = json!({ "error": format!("{}", e) });
                send_response(&mut stream, 502, "Bad Gateway", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
            }
        }
        return;
    }

    // 4. REST / SSE API: AI Assistant (Brokered via backend AgentSession & LlmClient)
    if clean_path == "api/ai/config" {
        match req.method.as_str() {
            "GET" => {
                let cfg = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
                let is_undo_available = ctx.session.lock().unwrap_or_else(|e| e.into_inner()).can_undo();
                let has_pending = ctx.session.lock().unwrap_or_else(|e| e.into_inner()).pending_action().is_some();
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
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
                return;
            }
            "POST" => {
                let body_str = String::from_utf8_lossy(&req.body);
                if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let mut cfg_guard = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
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
                        cfg_guard.api_key = if k.is_empty() { None } else { Some(k.to_string()) };
                    }
                    if let Some(t) = json_body.get("timeout").and_then(|v| v.as_u64()) {
                        cfg_guard.timeout_secs = t;
                    }
                    if let Some(a) = json_body.get("allow_self_signed").and_then(|v| v.as_bool()) {
                        cfg_guard.allow_self_signed = a;
                    }

                    let new_client = LlmClient::new(cfg_guard.clone());
                    let perm_mgr = PermissionManager::new(ctx.perm_config.lock().unwrap_or_else(|e| e.into_inner()).clone());
                    ctx.session.lock().unwrap_or_else(|e| e.into_inner()).update_config(perm_mgr, new_client);

                    let resp = json!({ "ok": true });
                    send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
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

        send_sse_header(&mut stream, &cors_origin);

        let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
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
                send_sse_event(&mut *s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut *s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo(),
                        "last_created_note": session_guard.last_created_note()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut *s, &json!({
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
                    send_sse_event(&mut *s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut *s);
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

        send_sse_header(&mut stream, &cors_origin);
        let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
        session_guard.reset_session(Some((context_filename, content)), None);

        let stream_mutex = Arc::new(Mutex::new(stream));
        let stream_for_tokens = stream_mutex.clone();

        let step_result = session_guard.send_prompt_streaming(&instruction, move |token| {
            if let Ok(mut s) = stream_for_tokens.lock() {
                send_sse_event(&mut *s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut *s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut *s, &json!({
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
                    send_sse_event(&mut *s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut *s);
        }
        return;
    }

    // POST /api/ai/confirm -> Tool call confirmation
    if clean_path == "api/ai/confirm" && req.method == "POST" {
        let body_str = String::from_utf8_lossy(&req.body);
        let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
        let approved = json_body.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);

        send_sse_header(&mut stream, &cors_origin);
        let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());

        let stream_mutex = Arc::new(Mutex::new(stream));
        let stream_for_tokens = stream_mutex.clone();

        let step_result = session_guard.confirm_pending_action_streaming(approved, move |token| {
            if let Ok(mut s) = stream_for_tokens.lock() {
                send_sse_event(&mut *s, &json!({
                    "type": "token",
                    "text": token
                }));
            }
        });

        if let Ok(mut s) = stream_mutex.lock() {
            match step_result {
                AgentStepResult::Finished { content, last_snapshot_id } => {
                    send_sse_event(&mut *s, &json!({
                        "type": "finished",
                        "content": content,
                        "last_snapshot_id": last_snapshot_id,
                        "can_undo": session_guard.can_undo()
                    }));
                }
                AgentStepResult::RequiresConfirmation(pending) => {
                    send_sse_event(&mut *s, &json!({
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
                    send_sse_event(&mut *s, &json!({
                        "type": "error",
                        "error": err
                    }));
                }
            }
            send_sse_done(&mut *s);
        }
        return;
    }

    // POST /api/ai/undo -> Revert last AI snapshot
    if clean_path == "api/ai/undo" && req.method == "POST" {
        let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
        match session_guard.undo_last_action() {
            Ok(msg) => {
                let resp = json!({ "ok": true, "message": msg, "can_undo": session_guard.can_undo() });
                send_response(&mut stream, 200, "OK", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
            }
            Err(err) => {
                let resp = json!({ "ok": false, "error": err, "can_undo": session_guard.can_undo() });
                send_response(&mut stream, 400, "Bad Request", "application/json; charset=utf-8", resp.to_string().as_bytes(), &cors_origin);
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
                        send_attachment_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &format!("{}.html", title), &cors_origin);
                        return;
                    }
                }
            } else if q.contains("view=rendered") {
                let file_path = ctx.notes_dir.join(&filename);
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                        let html = render_web_page_html(&content, title, &ctx.notes_dir, &filename);
                        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &cors_origin);
                        return;
                    }
                }
            }
        }

        // Default: serve the Vue 3 interactive editor app (app router loads the note)
        send_response(&mut stream, 200, "OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes(), &cors_origin);
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
                send_response(&mut stream, 200, "OK", "text/plain; charset=utf-8", content.as_bytes(), &cors_origin);
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
                send_attachment_response(&mut stream, 200, "OK", "text/html; charset=utf-8", html.as_bytes(), &format!("{}.html", title), &cors_origin);
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
            send_response(&mut stream, 200, "OK", mime, &bytes, &cors_origin);
            return;
        }
    }

    send_response(&mut stream, 404, "Not Found", "text/html; charset=utf-8", b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Fishdoc Editor</a></p>", &cors_origin);
}

fn send_response_full<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    cors_origin: &str,
    extra_headers: &[(&str, &str)],
) {
    let cors = build_cors_headers(cors_origin);
    let mut extra = String::new();
    for (k, v) in extra_headers {
        let sanitized = sanitize_header_value(v);
        extra.push_str(&format!("{}: {}\r\n", k, sanitized));
    }
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{}{}\r\n",
        status_code,
        status_text,
        content_type,
        body.len(),
        cors,
        extra
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn send_response<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    cors_origin: &str,
) {
    send_response_full(stream, status_code, status_text, content_type, body, cors_origin, &[]);
}

fn send_redirect<W: Write>(
    stream: &mut W,
    location: &str,
    cors_origin: &str,
    extra_headers: &[(&str, &str)],
) {
    let mut headers = vec![("Location", location)];
    headers.extend_from_slice(extra_headers);
    send_response_full(
        stream,
        302,
        "Found",
        "text/plain; charset=utf-8",
        b"Redirecting...",
        cors_origin,
        &headers,
    );
}

fn send_attachment_response<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    filename: &str,
    cors_origin: &str,
) {
    let safe_filename = sanitize_header_value(filename);
    let disposition = format!("attachment; filename=\"{}\"", safe_filename);
    send_response_full(
        stream,
        status_code,
        status_text,
        content_type,
        body,
        cors_origin,
        &[("Content-Disposition", &disposition)],
    );
}

fn send_sse_header<W: Write>(stream: &mut W, cors_origin: &str) {
    let cors = build_cors_headers(cors_origin);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: close\r\n{}\r\n",
        cors
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.flush();
}

fn send_sse_event<W: Write>(stream: &mut W, data: &serde_json::Value) {
    let payload = format!("data: {}\n\n", data);
    let _ = stream.write_all(payload.as_bytes());
    let _ = stream.flush();
}

fn send_sse_done<W: Write>(stream: &mut W) {
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

    #[test]
    fn test_server_basic_auth_and_session_lifecycle() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();
        fs::write(notes_dir.join("secret_doc.adoc"), "= Secret\nConfidential information.").unwrap();

        let mut auth_cfg = auth::AuthConfig::default();
        auth_cfg.enabled = true;
        auth_cfg.basic_enabled = true;
        auth_cfg.basic_username = "admin".to_string();
        auth_cfg.set_password("MySecretPass!").unwrap();

        let config = ServerConfig {
            notes_dir: notes_dir.clone(),
            db_path,
            backup_dir,
            port: 18940,
            llm_config: LlmConfig::default(),
            permission_config: PermissionConfig::default(),
            auth_config: auth_cfg,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
        };

        let server_handle = start_server_with_config(config).expect("Server should start");
        let port = server_handle.port();

        // 1. Unauthenticated request to protected API should fail with 401
        let unauth_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port)).call();
        assert!(unauth_res.is_err());
        if let Err(ureq::Error::Status(code, _)) = unauth_res {
            assert_eq!(code, 401);
        } else {
            panic!("Expected 401 Unauthorized");
        }

        // 2. Unauthenticated request to public endpoint /api/ping should succeed
        let ping_res = ureq::get(&format!("http://127.0.0.1:{}/api/ping", port)).call().unwrap();
        assert_eq!(ping_res.status(), 200);
        let ping_json: serde_json::Value = ping_res.into_json().unwrap();
        assert_eq!(ping_json.get("auth_enabled").unwrap(), true);

        // 3. Login with invalid password should fail
        let bad_login = ureq::post(&format!("http://127.0.0.1:{}/api/auth/login", port))
            .send_json(json!({
                "username": "admin",
                "password": "wrongpassword"
            }));
        assert!(bad_login.is_err());

        // 4. Login with valid credentials
        let login_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/login", port))
            .send_json(json!({
                "username": "admin",
                "password": "MySecretPass!"
            }))
            .unwrap();
        assert_eq!(login_res.status(), 200);
        let cookie_header = login_res.header("Set-Cookie").expect("Should return Set-Cookie header");
        assert!(cookie_header.contains("fishdoc_session="));

        let cookie_val = cookie_header.split(';').next().unwrap().to_string();

        // 5. Authenticated request with session cookie
        let auth_notes_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Cookie", &cookie_val)
            .call()
            .unwrap();
        assert_eq!(auth_notes_res.status(), 200);

        // 6. Test /api/auth/whoami with cookie
        let whoami_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/whoami", port))
            .set("Cookie", &cookie_val)
            .call()
            .unwrap();
        let whoami_json: serde_json::Value = whoami_res.into_json().unwrap();
        assert_eq!(whoami_json.get("authenticated").unwrap(), true);
        assert_eq!(whoami_json.get("user").unwrap(), "admin");

        // 7. Test Basic Auth header directly (admin:MySecretPass! in base64 is YWRtaW46TXlTZWNyZXRQYXNzIQ==)
        let basic_hdr_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Authorization", "Basic YWRtaW46TXlTZWNyZXRQYXNzIQ==")
            .call()
            .unwrap();
        assert_eq!(basic_hdr_res.status(), 200);

        // 8. Logout
        let logout_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/logout", port))
            .set("Cookie", &cookie_val)
            .call()
            .unwrap();
        assert_eq!(logout_res.status(), 200);

        // 9. After logout, session is invalid
        let post_logout = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Cookie", &cookie_val)
            .call();
        assert!(post_logout.is_err());

        server_handle.stop();
    }

    #[test]
    fn test_server_tls_initialization() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test.db");
        let backup_dir = tmp.path().join("backups");
        let tls_dir = tmp.path().join("tls");
        fs::create_dir_all(&notes_dir).unwrap();

        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");

        let config = ServerConfig {
            notes_dir,
            db_path,
            backup_dir,
            port: 18960,
            llm_config: LlmConfig::default(),
            permission_config: PermissionConfig::default(),
            auth_config: auth::AuthConfig::default(),
            enable_tls: true,
            tls_cert_path: Some(cert_path.clone()),
            tls_key_path: Some(key_path.clone()),
        };

        let server_handle = start_server_with_config(config).expect("TLS server should start");
        assert!(server_handle.is_tls());
        assert!(server_handle.primary_url().starts_with("https://"));
        assert!(cert_path.exists());
        assert!(key_path.exists());

        server_handle.stop();
    }
}
