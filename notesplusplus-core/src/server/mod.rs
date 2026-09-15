//! Embedded HTTP server and REST / SSE API for Notes++ Web Editor & AI Assistant.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub mod auth;
pub mod cert_gen;
pub mod http;
pub mod rate_limit;
pub mod routes;
pub mod tls;
pub mod web_assets;

pub use http::{
    get_local_ip_addresses, get_network_interfaces, is_private_or_local_ip, is_public_ip,
    sanitize_header_value,
};
pub use routes::handle_http_client;
pub use routes::pages::{
    list_all_notes_json, render_web_page_html,
};

use std::collections::HashMap;

use crate::agent::{
    AgentSession, LlmClient, LlmConfig, PermissionConfig, PermissionManager,
};

/// Server configuration holding paths, networking, security, and AI client parameters.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub notes_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub port: u16,
    pub bind_address: String,
    pub llm_config: LlmConfig,
    pub permission_config: PermissionConfig,
    pub auth_config: auth::AuthConfig,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub reject_public_networks: bool,
}

impl ServerConfig {
    pub fn from_paths(paths: crate::paths::AppPaths) -> Self {
        let backup_dir = paths.backup_dir();
        let assets_dir = paths.assets_dir();
        Self {
            notes_dir: paths.notes_dir,
            assets_dir,
            db_path: paths.db_path,
            backup_dir,
            port: crate::constants::DEFAULT_SERVER_PORT,
            bind_address: "0.0.0.0".to_string(),
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self::from_paths(crate::paths::AppPaths::new())
    }
}

/// Lightweight concurrency limiter for bounding the number of simultaneous active HTTP client threads.
#[derive(Clone)]
pub struct ConcurrencyLimiter {
    state: Arc<(Mutex<usize>, std::sync::Condvar)>,
    max_permits: usize,
}

impl ConcurrencyLimiter {
    pub fn new(max_permits: usize) -> Self {
        Self {
            state: Arc::new((Mutex::new(0), std::sync::Condvar::new())),
            max_permits,
        }
    }

    pub fn acquire(&self) -> PermitGuard {
        let (lock, cvar) = &*self.state;
        let mut count = lock.lock().unwrap_or_else(|e| e.into_inner());
        while *count >= self.max_permits {
            count = cvar.wait(count).unwrap_or_else(|e| e.into_inner());
        }
        *count += 1;
        PermitGuard {
            state: self.state.clone(),
        }
    }
}

pub struct PermitGuard {
    state: Arc<(Mutex<usize>, std::sync::Condvar)>,
}

impl Drop for PermitGuard {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.state;
        let mut count = lock.lock().unwrap_or_else(|e| e.into_inner());
        if *count > 0 {
            *count -= 1;
        }
        cvar.notify_one();
    }
}

/// Shared runtime context passed across request worker threads.
#[derive(Clone)]
pub struct ServerContext {
    pub notes_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub session: Arc<Mutex<AgentSession>>,
    pub llm_config: Arc<Mutex<LlmConfig>>,
    pub perm_config: Arc<Mutex<PermissionConfig>>,
    pub auth_config: Arc<Mutex<auth::AuthConfig>>,
    pub session_store: auth::SessionStore,
    pub auth_challenges: auth::AuthChallengeStore,
    pub pending_auth_challenge: Arc<Mutex<Option<String>>>,
    pub rate_limiter: rate_limit::RateLimiter,
    pub tls_status: Arc<Mutex<Option<tls::TlsStatusInfo>>>,
    pub reject_public_networks: Arc<AtomicBool>,
    pub theme_colors: Arc<Mutex<HashMap<String, String>>>,
    pub repository: Arc<dyn crate::repository::NoteRepository>,
    pub is_tls: bool,
}

impl ServerContext {
    pub fn new(config: ServerConfig) -> Self {
        Self::new_with_tls(config, false)
    }

    pub fn new_with_tls(config: ServerConfig, is_tls: bool) -> Self {
        let _ = fs::create_dir_all(&config.notes_dir);
        let _ = fs::create_dir_all(&config.backup_dir);

        let conn = match crate::db::open_db(&config.db_path) {
            Ok(c) => c,
            Err(e) => {
                log::error!("[SERVER] Failed to open database at {:?}: {}. Falling back to in-memory DB — data will NOT persist!", config.db_path, e);
                rusqlite::Connection::open_in_memory().expect("Failed to open even in-memory SQLite database")
            }
        };
        let conn_arc = Arc::new(Mutex::new(conn));
        if let Err(e) = crate::page::sync_and_index_pages(&conn_arc.lock().unwrap_or_else(|e| e.into_inner()), &config.notes_dir) {
            log::warn!("[SERVER] Initial index sync failed: {}", e);
        }
        let repository = Arc::new(crate::repository::FsSqliteNoteRepository::new(&config.notes_dir, Arc::clone(&conn_arc)));

        let perm_mgr = PermissionManager::new(config.permission_config.clone());
        let client = LlmClient::new(config.llm_config.clone());
        let mut session = AgentSession::new(
            &config.notes_dir,
            repository.clone(),
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
            assets_dir: config.assets_dir,
            db_path: config.db_path,
            backup_dir: config.backup_dir,
            session: Arc::new(Mutex::new(session)),
            llm_config: Arc::new(Mutex::new(config.llm_config)),
            perm_config: Arc::new(Mutex::new(config.permission_config)),
            auth_config: Arc::new(Mutex::new(config.auth_config)),
            session_store: auth::SessionStore::with_storage(sessions_path),
            auth_challenges: auth::AuthChallengeStore::new(),
            pending_auth_challenge: Arc::new(Mutex::new(None)),
            rate_limiter: rate_limit::RateLimiter::new(),
            tls_status: Arc::new(Mutex::new(None)),
            reject_public_networks: Arc::new(AtomicBool::new(config.reject_public_networks)),
            theme_colors: Arc::new(Mutex::new(HashMap::new())),
            repository,
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

    /// Returns the currently configured session expiration in seconds.
    pub fn session_expiry_secs(&self) -> u64 {
        self.auth_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .session_expiry_secs
    }

    /// Updates the configured session expiration duration in seconds.
    pub fn set_session_expiry_secs(&self, secs: u64) {
        let mut guard = self.auth_config.lock().unwrap_or_else(|e| e.into_inner());
        guard.session_expiry_secs = secs;
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
    bind_address: String,
    context: ServerContext,
    server: Arc<tiny_http::Server>,
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
                format!("{}://{}:{}", scheme, self.bind_address, self.port)
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
        self.server.unblock();
        let connect_addr = if self.bind_address == "0.0.0.0" { "127.0.0.1" } else { self.bind_address.as_str() };
        let _ = TcpStream::connect(format!("{}:{}", connect_addr, self.port));
    }
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
    let assets_dir = notes_dir.parent().unwrap_or(&notes_dir).join(crate::constants::ASSETS_DIR_NAME);
    let config = ServerConfig {
        notes_dir,
        assets_dir,
        db_path,
        backup_dir,
        port: requested_port,
        bind_address: "0.0.0.0".to_string(),
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

/// Start the embedded documentation HTTP server with a `ServerConfig`.
pub fn start_server_with_config(config: ServerConfig) -> Result<HttpServerHandle, String> {
    let port = if config.port == 0 { crate::constants::DEFAULT_SERVER_PORT } else { config.port };
    let mut listener = None;

    let bind_addr = config.bind_address.clone();
    for p in port..(port + crate::constants::PORT_SCAN_RANGE) {
        if let Ok(l) = TcpListener::bind((bind_addr.as_str(), p)) {
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
    if bind_addr == "0.0.0.0" {
        local_urls.push(format!("{}://0.0.0.0:{}", scheme, actual_port));
    }
    for ip in &local_ips {
        local_urls.push(format!("{}://{}:{}", scheme, ip, actual_port));
    }
    if local_urls.is_empty() {
        local_urls.push(format!("{}://127.0.0.1:{}", scheme, actual_port));
    }

    let ssl_info = if config.enable_tls {
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
        let ssl_config = tiny_http::SslConfig {
            certificate: cert.cert_pem.into_bytes(),
            private_key: cert.key_pem.into_bytes(),
        };
        Some((ssl_config, cert_path, key_path))
    } else {
        None
    };

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    let is_tls = ssl_info.is_some();

    let context = ServerContext::new_with_tls(config, is_tls);
    let (tiny_ssl, cert_p, key_p) = match ssl_info {
        Some((ssl, cp, kp)) => (Some(ssl), Some(cp), Some(kp)),
        None => (None, None, None),
    };

    if let (Some(cert_path), Some(key_path)) = (cert_p, key_p) {
        context.update_tls_status(Some(tls::TlsStatusInfo {
            is_tls: true,
            is_custom: tls::is_custom_cert_installed(&cert_path),
            cert_path: cert_path.to_string_lossy().to_string(),
            key_path: key_path.to_string_lossy().to_string(),
            subject: "Notes++ Web Server".to_string(),
        }));
    }

    let server = tiny_http::Server::from_listener(listener, tiny_ssl)
        .map_err(|e| format!("Failed to create HTTP server: {}", e))?;
    let server = Arc::new(server);

    let context_clone = context.clone();
    let server_clone = server.clone();
    let limiter = ConcurrencyLimiter::new(crate::constants::MAX_CONCURRENT_CONNECTIONS);

    thread::spawn(move || {
        while is_running_clone.load(Ordering::SeqCst) {
            match server_clone.recv() {
                Ok(req) => {
                    if !is_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Some(remote) = req.remote_addr() {
                        if context_clone.reject_public_networks() && is_public_ip(&remote.ip()) {
                            log::warn!(
                                "[SERVER] Rejected connection from public IP address: {}",
                                remote.ip()
                            );
                            let _ = req.respond(tiny_http::Response::empty(403));
                            continue;
                        }
                    }
                    let ctx = context_clone.clone();
                    let permit = limiter.acquire();
                    thread::spawn(move || {
                        let _permit = permit;
                        handle_http_client(req, ctx);
                    });
                }
                Err(e) => {
                    if !is_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    log::warn!("[SERVER] Accept error: {}", e);
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    });

    Ok(HttpServerHandle {
        is_running,
        port: actual_port,
        local_urls,
        bind_address: bind_addr,
        context,
        server,
    })
}

