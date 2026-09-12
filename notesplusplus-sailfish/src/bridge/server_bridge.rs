use notesplusplus_core::agent::DEFAULT_OLLAMA_ENDPOINT;

use super::NotesBridge;

impl NotesBridge {
    pub(crate) fn start_web_server_impl(&mut self) -> String {
        self.ensure_init();
        if self.web_server_running {
            return self.web_server_url.clone();
        }

        let db_path = self.data_dir.join(notesplusplus_core::constants::DB_FILENAME);
        let backup_dir = self.data_dir.join("backups");
        let cert_dir = self.data_dir.join("tls");
        let cert_path = cert_dir.join("server.crt");
        let key_path = cert_dir.join("server.key");

        let config = notesplusplus_core::server::ServerConfig {
            notes_dir: self.notes_path.clone(),
            notes_subdir: self.notes_dir(),
            db_path,
            backup_dir,
            port: 8080,
            bind_address: self.bind_address.clone(),
            llm_config: self.llm_config.clone(),
            permission_config: self.permission_config.clone(),
            auth_config: self.auth_config.clone(),
            enable_tls: true,
            tls_cert_path: Some(cert_path),
            tls_key_path: Some(key_path),
            reject_public_networks: self.reject_public_networks,
        };

        match notesplusplus_core::server::start_server_with_config(config) {
            Ok(handle) => {
                if !self.pending_theme_colors.is_empty() {
                    handle.context().set_theme_colors(self.pending_theme_colors.clone());
                }
                let primary_url = handle.primary_url().replace("0.0.0.0", "localhost");
                self.web_server_url = primary_url.clone();
                self.web_server_running = true;
                self.server_handle = Some(handle);
                self.web_server_status_changed();
                primary_url
            }
            Err(e) => {
                self.report_error(format!("Failed to start web server: {}", e));
                String::new()
            }
        }
    }

    pub(crate) fn stop_web_server_impl(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.stop();
        }
        self.web_server_running = false;
        self.web_server_url = String::new();
        self.web_server_status_changed();
    }

    pub(crate) fn toggle_web_server_impl(&mut self) -> bool {
        if self.web_server_running {
            self.stop_web_server_impl();
            false
        } else {
            !self.start_web_server_impl().is_empty()
        }
    }

    pub(crate) fn configure_ai_impl(
        &mut self,
        provider: String,
        url: String,
        model: String,
        key: String,
        timeout: i32,
        auto_read: bool,
        auto_create: bool,
        require_edit: bool,
        allow_self_signed: bool,
        allow_fetch: bool,
    ) {
        let p: notesplusplus_core::agent::LlmProvider = provider.parse().unwrap_or_default();
        self.llm_config.provider = p;
        self.llm_config.endpoint_url = if url.trim().is_empty() {
            match p {
                notesplusplus_core::agent::LlmProvider::Ollama => DEFAULT_OLLAMA_ENDPOINT.to_string(),
                notesplusplus_core::agent::LlmProvider::OpenAiCompatible => "https://api.mimocode.com".to_string(),
            }
        } else {
            url.trim().to_string()
        };
        self.llm_config.model = if model.trim().is_empty() {
            notesplusplus_core::constants::DEFAULT_AI_MODEL.to_string()
        } else {
            model.trim().to_string()
        };
        self.llm_config.api_key = if key.trim().is_empty() {
            None
        } else {
            Some(key.trim().to_string())
        };
        self.llm_config.timeout_secs = if timeout > 0 { timeout as u64 } else { 90 };
        self.llm_config.allow_self_signed = allow_self_signed;

        self.permission_config.auto_allow_read = auto_read;
        self.permission_config.auto_allow_create = auto_create;
        self.permission_config.require_confirm_edit = require_edit;
        self.permission_config.allow_fetch_url = allow_fetch;

        if let Some(ref handle) = self.server_handle {
            handle.context().update_llm_config(self.llm_config.clone(), Some(self.permission_config.clone()));
        }
    }

    pub(crate) fn install_tls_certificate_impl(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String {
        self.ensure_init();
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");

        match notesplusplus_core::server::tls::install_custom_tls_cert(
            &cert_pem_or_path,
            &key_pem_or_path,
            &cert_path,
            &key_path,
        ) {
            Ok(_) => {
                if self.web_server_running {
                    self.stop_web_server_impl();
                    self.start_web_server_impl();
                }
                String::new()
            }
            Err(e) => {
                self.report_error(format!("Failed to install SSL certificate: {}", e));
                e
            }
        }
    }

    pub(crate) fn reset_tls_certificate_impl(&mut self) -> String {
        self.ensure_init();
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");

        match notesplusplus_core::server::tls::reset_to_self_signed_cert(&cert_path, &key_path, None) {
            Ok(_) => {
                if self.web_server_running {
                    self.stop_web_server_impl();
                    self.start_web_server_impl();
                }
                String::new()
            }
            Err(e) => {
                self.report_error(format!("Failed to reset SSL certificate: {}", e));
                e
            }
        }
    }

    pub(crate) fn is_custom_tls_certificate_impl(&self) -> bool {
        let cert_path = self.data_dir.join("tls").join("server.crt");
        notesplusplus_core::server::tls::is_custom_cert_installed(&cert_path)
    }

    pub(crate) fn get_tls_certificate_info_json_impl(&self) -> String {
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");
        let is_custom = notesplusplus_core::server::tls::is_custom_cert_installed(&cert_path);
        serde_json::json!({
            "is_custom": is_custom,
            "cert_path": cert_path.to_string_lossy(),
            "key_path": key_path.to_string_lossy(),
            "exists": cert_path.exists() && key_path.exists(),
        }).to_string()
    }

    // QML wrappers for server management
    pub fn configure_ai(&mut self, provider: String, url: String, model: String, key: String, timeout: i32, auto_read: bool, auto_create: bool, require_edit: bool, allow_self_signed: bool, allow_fetch: bool) {
        self.configure_ai_impl(provider, url, model, key, timeout, auto_read, auto_create, require_edit, allow_self_signed, allow_fetch);
    }
    pub fn install_tls_certificate(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String {
        self.install_tls_certificate_impl(cert_pem_or_path, key_pem_or_path)
    }
    pub fn reset_tls_certificate(&mut self) -> String {
        self.reset_tls_certificate_impl()
    }
    pub fn is_custom_tls_certificate(&mut self) -> bool {
        self.is_custom_tls_certificate_impl()
    }
    pub fn get_tls_certificate_info_json(&mut self) -> String {
        self.get_tls_certificate_info_json_impl()
    }
    pub fn get_server_urls_json(&self) -> String { self.get_server_urls_json_impl() }
    pub fn start_web_server(&mut self) -> String { self.start_web_server_impl() }
    pub fn stop_web_server(&mut self) { self.stop_web_server_impl(); }
    pub fn toggle_web_server(&mut self) -> bool { self.toggle_web_server_impl() }
    pub fn set_theme(&mut self, colors_json: String) { self.set_theme_impl(colors_json); }
    pub fn set_session_expiry_hours(&mut self, hours: i32) { self.set_session_expiry_hours_impl(hours); }

    /// Polls the server context for a pending authorization challenge.
    pub fn check_auth_challenge(&mut self) -> bool {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            let guard = ctx.pending_auth_challenge.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(challenge_id) = guard.as_ref() {
                let mut changed = false;
                if self.auth_challenge_id != *challenge_id {
                    self.auth_challenge_id = challenge_id.clone();
                    changed = true;
                }
                if let Some(challenge) = ctx.auth_challenges.get_challenge(challenge_id) {
                    if self.auth_verification_code != challenge.verification_code {
                        self.auth_verification_code = challenge.verification_code.clone();
                        changed = true;
                    }
                }
                if !self.auth_challenge_pending {
                    self.auth_challenge_pending = true;
                    changed = true;
                }
                if changed {
                    self.auth_challenge_changed();
                }
                return true;
            }
        }
        if self.auth_challenge_pending {
            self.auth_challenge_pending = false;
            self.auth_challenge_id = String::new();
            self.auth_verification_code = String::new();
            self.auth_challenge_changed();
        }
        false
    }

    /// Approves an authorization challenge.
    pub fn approve_auth_challenge(&mut self, challenge_id: String) {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            ctx.auth_challenges.approve_challenge(&challenge_id);
            ctx.clear_auth_challenge();
        }
        self.auth_challenge_pending = false;
        self.auth_challenge_id = String::new();
        self.auth_verification_code = String::new();
        self.auth_challenge_changed();
    }

    /// Denies/cancels an authorization challenge.
    pub fn deny_auth_challenge(&mut self, challenge_id: String) {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            ctx.auth_challenges.deny_challenge(&challenge_id);
            ctx.clear_auth_challenge();
        }
        self.auth_challenge_pending = false;
        self.auth_challenge_id = String::new();
        self.auth_verification_code = String::new();
        self.auth_challenge_changed();
    }
}
