//! Embedded HTTP server and REST / SSE API for Notes++ Web Editor & AI Assistant.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub mod auth;
pub mod http;
pub mod routes;
pub mod tls;
pub mod web_assets;

pub use http::{
    get_local_ip_addresses, is_private_or_local_ip, is_public_ip, sanitize_header_value,
    StreamWrapper,
};
pub use routes::handle_http_client;
pub use routes::pages::{
    extract_title_from_adoc, list_all_notes_json, make_slug_filename, render_web_page_html,
};

use std::collections::HashMap;

use crate::agent::{
    AgentSession, LlmClient, LlmConfig, PermissionConfig, PermissionManager,
};

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
    pub reject_public_networks: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let paths = crate::paths::AppPaths::new();
        let backup_dir = paths.data_dir.join("backups");
        Self {
            notes_dir: paths.notes_dir,
            db_path: paths.db_path,
            backup_dir,
            port: crate::constants::DEFAULT_SERVER_PORT,
            llm_config: LlmConfig::default(),
            permission_config: PermissionConfig::default(),
            auth_config: auth::AuthConfig::default(),
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            reject_public_networks: true,
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
    pub auth_challenges: auth::AuthChallengeStore,
    pub pending_auth_challenge: Arc<Mutex<Option<String>>>,
    pub tls_status: Arc<Mutex<Option<tls::TlsStatusInfo>>>,
    pub reject_public_networks: Arc<AtomicBool>,
    pub theme_colors: Arc<Mutex<HashMap<String, String>>>,
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

        let sessions_path = config
            .db_path
            .parent()
            .map(|p| p.join("sessions.json"))
            .unwrap_or_else(|| config.backup_dir.join("sessions.json"));

        Self {
            notes_dir: config.notes_dir,
            db_path: config.db_path,
            backup_dir: config.backup_dir,
            session: Arc::new(Mutex::new(session)),
            llm_config: Arc::new(Mutex::new(config.llm_config)),
            perm_config: Arc::new(Mutex::new(config.permission_config)),
            auth_config: Arc::new(Mutex::new(config.auth_config)),
            session_store: auth::SessionStore::with_storage(sessions_path),
            auth_challenges: auth::AuthChallengeStore::new(),
            pending_auth_challenge: Arc::new(Mutex::new(None)),
            tls_status: Arc::new(Mutex::new(None)),
            reject_public_networks: Arc::new(AtomicBool::new(config.reject_public_networks)),
            theme_colors: Arc::new(Mutex::new(HashMap::new())),
            is_tls,
        }
    }

    /// Updates whether connections from public networks should be rejected.
    pub fn set_reject_public_networks(&self, reject: bool) {
        self.reject_public_networks.store(reject, Ordering::SeqCst);
    }

    /// Returns whether connections from public networks are currently rejected.
    pub fn reject_public_networks(&self) -> bool {
        self.reject_public_networks.load(Ordering::SeqCst)
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

    /// Updates the Sailfish ambience theme colors for the web UI.
    pub fn set_theme_colors(&self, colors: HashMap<String, String>) {
        let mut guard = self.theme_colors.lock().unwrap_or_else(|e| e.into_inner());
        *guard = colors;
    }

    /// Signals that an authorization challenge is pending, notifying the QML app.
    pub fn signal_auth_challenge(&self, challenge_id: String) {
        let mut guard = self.pending_auth_challenge.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(challenge_id);
    }

    /// Clears the pending authorization challenge signal.
    pub fn clear_auth_challenge(&self) {
        let mut guard = self.pending_auth_challenge.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }

    /// Takes the pending authorization challenge ID (returns and clears it).
    pub fn take_auth_challenge(&self) -> Option<String> {
        let mut guard = self.pending_auth_challenge.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
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
    let db_path = notes_path.parent().unwrap_or(&notes_path).join("notesplusplus.db");
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
        reject_public_networks: true,
    };
    start_server_with_config(config)
}

/// Immediately drops and aborts a rejected TCP connection (sending RST)
/// to avoid TIME_WAIT socket exhaustion and prevent DDoS / resource starvation.
fn drop_rejected_connection(stream: TcpStream) {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let linger = libc::linger {
            l_onoff: 1,
            l_linger: 0,
        };
        unsafe {
            let _ = libc::setsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_LINGER,
                &linger as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::linger>() as libc::socklen_t,
            );
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
    drop(stream);
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

        let mut alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        for ip in &local_ips {
            let addr = ip.to_string();
            if !alt_names.contains(&addr) {
                alt_names.push(addr);
            }
        }
        // Include wildcard DNS and common private ranges so the cert works
        // regardless of which network the phone is on
        for dns in &["*.local", "*.home", "*.lan"] {
            let s = dns.to_string();
            if !alt_names.contains(&s) {
                alt_names.push(s);
            }
        }
        let tls_opts = tls::TlsOptions {
            alt_names,
            ..tls::TlsOptions::default()
        };
        let cert = tls::get_or_create_tls_cert(&cert_path, &key_path, Some(tls_opts))?;
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
            subject: "Notes++ Web Server".to_string(),
        }));
    }

    let context_clone = context.clone();

    thread::spawn(move || {
        let _ = listener.set_nonblocking(false);

        while is_running_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, client_addr)) => {
                    if !is_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    if context_clone.reject_public_networks() && is_public_ip(&client_addr.ip()) {
                        log::warn!(
                            "[SERVER] Rejected connection from public IP address: {}",
                            client_addr.ip()
                        );
                        drop_rejected_connection(stream);
                        continue;
                    }
                    let _ = stream.set_nodelay(true);
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

/// Start the embedded documentation HTTP server with custom certificate and private key.
pub fn start_server_with_cert_and_key(
    notes_path: PathBuf,
    requested_port: u16,
    cert_path: PathBuf,
    key_path: PathBuf,
) -> Result<HttpServerHandle, String> {
    let db_path = notes_path.parent().unwrap_or(&notes_path).join("notesplusplus.db");
    let backup_dir = notes_path.parent().unwrap_or(&notes_path).join("backups");
    let config = ServerConfig {
        notes_dir: notes_path,
        db_path,
        backup_dir,
        port: requested_port,
        enable_tls: true,
        tls_cert_path: Some(cert_path),
        tls_key_path: Some(key_path),
        ..Default::default()
    };
    start_server_with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
        assert!(root_body.contains("Notes++"));

        // Test GET /icon.png
        let res_icon = ureq::get(&format!("http://127.0.0.1:{}/icon.png", port)).call().unwrap();
        assert_eq!(res_icon.status(), 200);
        assert_eq!(res_icon.header("Content-Type").unwrap(), "image/png");

        // Test GET /app.js
        let res_js = ureq::get(&format!("http://127.0.0.1:{}/app.js", port)).call().unwrap();
        assert_eq!(res_js.status(), 200);
        let js_body = res_js.into_string().unwrap();
        assert!(js_body.contains("createApp"));

        // Test GET /vue.esm-browser.prod.js (vendored Vue)
        let res_vue = ureq::get(&format!("http://127.0.0.1:{}/vue.esm-browser.prod.js", port)).call().unwrap();
        assert_eq!(res_vue.status(), 200);
        let vue_body = res_vue.into_string().unwrap();
        assert!(vue_body.contains("vue"));

        // Test GET /vue.js alias
        let res_vue_alias = ureq::get(&format!("http://127.0.0.1:{}/vue.js", port)).call().unwrap();
        assert_eq!(res_vue_alias.status(), 200);

        // Test GET /style.css
        let res_css = ureq::get(&format!("http://127.0.0.1:{}/style.css", port)).call().unwrap();
        assert_eq!(res_css.status(), 200);

        // Authenticate session for API testing
        let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
        let session_cookie = format!("{}={}", crate::constants::SESSION_COOKIE_NAME, sess.id);

        // Test GET /api/notes
        let res_notes = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(res_notes.status(), 200);
        let notes_json: serde_json::Value = res_notes.into_json().unwrap();
        assert!(!notes_json.as_array().unwrap().is_empty());

        // Test GET /api/notes/welcome.adoc
        let res_note = ureq::get(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(res_note.status(), 200);
        assert!(res_note.into_string().unwrap().contains("Test content"));

        // Test PUT /api/notes/welcome.adoc
        let put_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port))
            .set("Cookie", &session_cookie)
            .set("Content-Type", "text/plain")
            .send_string("= Welcome\nUpdated content from PUT test.")
            .unwrap();
        assert_eq!(put_res.status(), 200);
        let updated_file = fs::read_to_string(notes_dir.join("welcome.adoc")).unwrap();
        assert!(updated_file.contains("Updated content from PUT test"));

        // Test POST /api/notes (create new)
        let create_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Cookie", &session_cookie)
            .send_json(json!({
                "title": "New Doc",
                "content": "= New Doc\nCreated via API"
            }))
            .unwrap();
        assert_eq!(create_res.status(), 200);
        assert!(notes_dir.join("new-doc.adoc").exists());

        // Test POST /api/render
        let render_res = ureq::post(&format!("http://127.0.0.1:{}/api/render", port))
            .set("Cookie", &session_cookie)
            .send_json(json!({
                "content": "= Header\n* Bullet item\n"
            }))
            .unwrap();
        assert_eq!(render_res.status(), 200);
        let render_html = render_res.into_string().unwrap();
        assert!(render_html.contains("Header") && render_html.contains("Bullet item"));

        // Test POST /api/blocks/parse
        let parse_res = ureq::post(&format!("http://127.0.0.1:{}/api/blocks/parse", port))
            .set("Cookie", &session_cookie)
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
            .set("Cookie", &session_cookie)
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
            .set("Cookie", &session_cookie)
            .send_json(json!({
                "provider": "openai",
                "model": "mistral",
                "system_prompt": "You are a concise technical writer."
            }))
            .unwrap();
        assert_eq!(ai_update_res.status(), 200);

        // Verify updated config
        let ai_cfg_res2 = ureq::get(&format!("http://127.0.0.1:{}/api/ai/config", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        let ai_cfg_json2: serde_json::Value = ai_cfg_res2.into_json().unwrap();
        assert_eq!(ai_cfg_json2.get("provider").unwrap(), "openai");
        assert_eq!(ai_cfg_json2.get("model").unwrap(), "mistral");
        assert_eq!(ai_cfg_json2.get("system_prompt").unwrap(), "You are a concise technical writer.");
        // Verify endpoint, tokens, keys, or cert options are never exposed in responses to web UI
        assert!(ai_cfg_json2.get("endpoint").is_none());
        assert!(ai_cfg_json2.get("api_key").is_none());
        assert!(ai_cfg_json2.get("has_key").is_none());
        assert!(ai_cfg_json2.get("allow_self_signed").is_none());

        // Test GET /raw/welcome.adoc
        let raw_res = ureq::get(&format!("http://127.0.0.1:{}/raw/welcome.adoc", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(raw_res.status(), 200);
        assert!(raw_res.into_string().unwrap().contains("Task"));

        // Test GET /export/welcome.adoc
        let export_res = ureq::get(&format!("http://127.0.0.1:{}/export/welcome.adoc", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(export_res.status(), 200);
        assert!(export_res.into_string().unwrap().contains("html"));

        // Test DELETE /api/notes/new-doc.adoc
        let del_res = ureq::delete(&format!("http://127.0.0.1:{}/api/notes/new-doc.adoc", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(del_res.status(), 200);
        assert!(!notes_dir.join("new-doc.adoc").exists());

        // Stop server
        server_handle.stop();
    }

    #[test]
    fn test_server_phone_auth_challenge_flow_and_multi_request_superseding() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test_phone_auth.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        fs::write(notes_dir.join("protected.adoc"), "= Protected\nContent only for authenticated users.").unwrap();

        let auth_cfg = auth::AuthConfig::default();

        let config = ServerConfig {
            notes_dir,
            db_path,
            backup_dir,
            port: 18945,
            llm_config: LlmConfig::default(),
            permission_config: PermissionConfig::default(),
            auth_config: auth_cfg,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            reject_public_networks: true,
        };

        let server_handle = start_server_with_config(config).expect("Auth server should start");
        let port = server_handle.port();

        // 1. Initial unauthenticated access fails with 401
        let unauth_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port)).call();
        assert!(unauth_res.is_err());

        // 2. Client 1 initiates phone authorization challenge
        let init_res1 = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
            .send_json(json!({}))
            .unwrap();
        assert_eq!(init_res1.status(), 200);
        let init_json1: serde_json::Value = init_res1.into_json().unwrap();
        assert_eq!(init_json1["ok"], true);
        let c1_id = init_json1["challenge_id"].as_str().unwrap().to_string();
        let c1_code = init_json1["verification_code"].as_str().unwrap().to_string();
        assert!(!c1_id.is_empty());
        assert_eq!(c1_code.len(), 4);

        // Verify pending challenge is set on the server context
        assert_eq!(
            server_handle.context().pending_auth_challenge.lock().unwrap().as_deref(),
            Some(c1_id.as_str())
        );

        // 3. Client 1 polls status -> status: "pending"
        let poll1 = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, c1_id))
            .call()
            .unwrap();
        let poll_json1: serde_json::Value = poll1.into_json().unwrap();
        assert_eq!(poll_json1["status"], "pending");

        // 4. Client 2 (or a repeated burst of requests) initiates another challenge
        let init_res2 = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
            .send_json(json!({}))
            .unwrap();
        assert_eq!(init_res2.status(), 200);
        let init_json2: serde_json::Value = init_res2.into_json().unwrap();
        assert_eq!(init_json2["ok"], true);
        let c2_id = init_json2["challenge_id"].as_str().unwrap().to_string();
        let c2_code = init_json2["verification_code"].as_str().unwrap().to_string();
        assert_ne!(c1_id, c2_id);
        assert_eq!(c2_code.len(), 4);

        // Server context's pending challenge should now point to Challenge 2 (superseding Challenge 1)
        assert_eq!(
            server_handle.context().pending_auth_challenge.lock().unwrap().as_deref(),
            Some(c2_id.as_str())
        );

        // 5. Phone accepts the currently displayed Challenge 2
        server_handle.context().auth_challenges.approve_challenge(&c2_id);
        server_handle.context().clear_auth_challenge();

        // 6. Client 2 polls status -> approved, gets session cookie
        let poll2 = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, c2_id))
            .call()
            .unwrap();
        let session_cookie = poll2
            .header("Set-Cookie")
            .expect("Should set session cookie")
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let poll_json2: serde_json::Value = poll2.into_json().unwrap();
        assert_eq!(poll_json2["status"], "approved");

        // 7. Client 2 uses cookie to access protected notes API
        let notes_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
            .set("Cookie", &session_cookie)
            .call()
            .unwrap();
        assert_eq!(notes_res.status(), 200);

        server_handle.stop();
    }

    #[test]
    fn test_server_tls_initialization() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test_tls.db");
        let backup_dir = tmp.path().join("backups");
        let tls_dir = tmp.path().join("tls");
        fs::create_dir_all(&notes_dir).unwrap();
        fs::create_dir_all(&tls_dir).unwrap();

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
            reject_public_networks: true,
        };

        let server_handle = start_server_with_config(config).expect("TLS server should start");
        assert!(server_handle.is_tls());
        assert!(server_handle.primary_url().starts_with("https://"));
        assert!(cert_path.exists());
        assert!(key_path.exists());

        server_handle.stop();
    }

    #[test]
    fn test_ip_classification_private_and_public() {
        use std::net::IpAddr;

        // IPv4 Loopback
        assert!(is_private_or_local_ip(&"127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"127.255.0.1".parse::<IpAddr>().unwrap()));
        assert!(!is_public_ip(&"127.0.0.1".parse::<IpAddr>().unwrap()));

        // RFC 1918 10.0.0.0/8
        assert!(is_private_or_local_ip(&"10.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"10.255.255.255".parse::<IpAddr>().unwrap()));
        assert!(!is_public_ip(&"10.1.2.3".parse::<IpAddr>().unwrap()));

        // RFC 1918 172.16.0.0/12
        assert!(is_private_or_local_ip(&"172.16.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"172.31.255.254".parse::<IpAddr>().unwrap()));
        assert!(!is_private_or_local_ip(&"172.15.255.255".parse::<IpAddr>().unwrap()));
        assert!(!is_private_or_local_ip(&"172.32.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"172.32.0.1".parse::<IpAddr>().unwrap()));

        // RFC 1918 192.168.0.0/16
        assert!(is_private_or_local_ip(&"192.168.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"192.168.1.100".parse::<IpAddr>().unwrap()));
        assert!(!is_private_or_local_ip(&"192.169.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"192.169.1.1".parse::<IpAddr>().unwrap()));

        // RFC 3927 Link-local 169.254.0.0/16
        assert!(is_private_or_local_ip(&"169.254.1.1".parse::<IpAddr>().unwrap()));

        // RFC 6598 CGNAT 100.64.0.0/10
        assert!(is_private_or_local_ip(&"100.64.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"100.127.255.255".parse::<IpAddr>().unwrap()));
        assert!(!is_private_or_local_ip(&"100.128.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"100.128.0.1".parse::<IpAddr>().unwrap()));

        // Public IPv4 Addresses
        assert!(is_public_ip(&"8.8.8.8".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"1.1.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"93.184.216.34".parse::<IpAddr>().unwrap()));

        // IPv6 Loopback & Unspecified
        assert!(is_private_or_local_ip(&"::1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"::".parse::<IpAddr>().unwrap()));

        // IPv6 Unique Local Addresses (fc00::/7)
        assert!(is_private_or_local_ip(&"fc00::1".parse::<IpAddr>().unwrap()));
        assert!(is_private_or_local_ip(&"fd12:3456:789a::1".parse::<IpAddr>().unwrap()));

        // IPv6 Link-Local (fe80::/10)
        assert!(is_private_or_local_ip(&"fe80::1".parse::<IpAddr>().unwrap()));

        // IPv4-mapped IPv6
        assert!(is_private_or_local_ip(&"::ffff:192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_public_ip(&"::ffff:8.8.8.8".parse::<IpAddr>().unwrap()));

        // Public IPv6
        assert!(is_public_ip(&"2607:f8b0:4005:805::200e".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn test_server_reject_public_networks_toggle() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test_rej.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let config = ServerConfig {
            notes_dir,
            db_path,
            backup_dir,
            port: 18980,
            reject_public_networks: true,
            ..Default::default()
        };

        let server_handle = start_server_with_config(config).expect("Server should start");
        assert!(server_handle.context().reject_public_networks());

        // Dynamic update
        server_handle.context().set_reject_public_networks(false);
        assert!(!server_handle.context().reject_public_networks());

        server_handle.context().set_reject_public_networks(true);
        assert!(server_handle.context().reject_public_networks());

        server_handle.stop();
    }

    #[test]
    fn test_drop_rejected_connection_executes_cleanly() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let client_thread = thread::spawn(move || {
            TcpStream::connect(format!("127.0.0.1:{}", port))
        });
        let (stream, _) = listener.accept().unwrap();
        drop_rejected_connection(stream);
        let client_sock = client_thread.join().unwrap();
        assert!(client_sock.is_ok());
    }

    #[test]
    fn test_link_page_widget_web_assets() {
        use crate::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

        // 1. Verify index.html contains Link buttons and Link Dialog modal overlay
        assert!(INDEX_HTML.contains("openLinkDialog"));
        assert!(INDEX_HTML.contains("link-modal-overlay"));
        assert!(INDEX_HTML.contains("link-search-box"));
        assert!(INDEX_HTML.contains("link-pages-list"));
        assert!(INDEX_HTML.contains("linkDisplayText"));
        assert!(INDEX_HTML.contains("formattedLinkPreview"));
        assert!(INDEX_HTML.contains("confirmLinkInsert"));

        // 2. Verify app.js contains link dialog state, computeds, and shortcut handlers
        assert!(APP_JS.contains("openLinkModal"));
        assert!(APP_JS.contains("linkSearchQuery"));
        assert!(APP_JS.contains("filteredLinkPages"));
        assert!(APP_JS.contains("formattedLinkPreview"));
        assert!(APP_JS.contains("openLinkDialog"));
        assert!(APP_JS.contains("confirmLinkInsert"));
        assert!(APP_JS.contains("handleLinkKeydown"));
        assert!(APP_JS.contains("selectLinkTarget"));
        assert!(APP_JS.contains("isExternalUrl"));
        assert!(APP_JS.contains("computedCustomFilename"));

        // 3. Verify style.css contains link modal classes
        assert!(STYLE_CSS.contains(".link-modal-card"));
        assert!(STYLE_CSS.contains(".link-search-box"));
        assert!(STYLE_CSS.contains(".link-pages-list"));
        assert!(STYLE_CSS.contains(".link-page-item"));
        assert!(STYLE_CSS.contains(".link-preview-container"));
    }

    #[test]
    fn test_ai_model_selection_web_assets() {
        use crate::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

        // 1. Verify index.html contains provider and model selection, and system prompt configuration
        assert!(INDEX_HTML.contains("ai-model-select"));
        assert!(INDEX_HTML.contains("availableModels"));
        assert!(INDEX_HTML.contains("fetchAvailableModels"));
        assert!(INDEX_HTML.contains("onModelSelect"));
        assert!(INDEX_HTML.contains("aiConfig.provider"));
        assert!(INDEX_HTML.contains("aiConfig.system_prompt"));

        // Verify index.html does NOT leak server address, api key, token inputs, or self-signed cert option
        assert!(!INDEX_HTML.contains("aiConfig.endpoint"));
        assert!(!INDEX_HTML.contains("aiConfig.apiKey"));
        assert!(!INDEX_HTML.contains("Endpoint URL"));
        assert!(!INDEX_HTML.contains("API Key"));
        assert!(!INDEX_HTML.contains("allow_self_signed"));
        assert!(!INDEX_HTML.contains("Accept Self-Signed"));

        // 2. Verify app.js contains provider, model fetching, system prompt state & methods
        assert!(APP_JS.contains("availableModels"));
        assert!(APP_JS.contains("fetchAvailableModels"));
        assert!(APP_JS.contains("onProviderChange"));
        assert!(APP_JS.contains("onModelSelect"));
        assert!(APP_JS.contains("/api/ai/models"));
        assert!(APP_JS.contains("isCurrentModelInList"));
        assert!(APP_JS.contains("aiConfig.value.system_prompt"));
        assert!(APP_JS.contains("aiConfig.value.provider"));

        // Verify app.js does not expose or send server endpoints, tokens, or self-signed cert flags
        assert!(!APP_JS.contains("aiConfig.value.endpoint"));
        assert!(!APP_JS.contains("aiConfig.value.apiKey"));
        assert!(!APP_JS.contains("allow_self_signed"));

        // 3. Verify style.css contains styling for the model select
        assert!(STYLE_CSS.contains(".ai-model-select"));
    }

    #[test]
    fn test_list_all_notes_json_metadata_and_filtering() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();

        fs::write(notes_dir.join("architecture.adoc"), "= System Architecture\nHigh level architecture diagram and components.").unwrap();
        fs::write(notes_dir.join("meeting-notes.adoc"), "= Meeting Notes\nDiscussion on deployment and release roadmap.").unwrap();

        // Without query: returns all notes
        let all_json = list_all_notes_json(&notes_dir, None);
        let all_notes: Vec<serde_json::Value> = serde_json::from_str(&all_json).unwrap();
        assert_eq!(all_notes.len(), 2);
        assert_eq!(all_notes[0]["filename"], "meeting-notes.adoc");
        assert_eq!(all_notes[0]["title"], "Meeting Notes");
        assert!(all_notes[0]["snippet"].as_str().unwrap().contains("Discussion"));

        // With query matching title/snippet
        let filtered_json = list_all_notes_json(&notes_dir, Some("Architecture"));
        let filtered_notes: Vec<serde_json::Value> = serde_json::from_str(&filtered_json).unwrap();
        assert_eq!(filtered_notes.len(), 1);
        assert_eq!(filtered_notes[0]["filename"], "architecture.adoc");
        assert_eq!(filtered_notes[0]["title"], "System Architecture");
    }
}
