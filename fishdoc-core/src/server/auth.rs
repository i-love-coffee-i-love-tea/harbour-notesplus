//! Authentication module for Fishdoc Embedded Server:
//! Supports HTTP Basic Authentication and OpenID Connect (OIDC) / OAuth 2.0 Single Sign-On.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use ring::digest::{digest, SHA256};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

/// Generates a cryptographically secure random hex string of given byte length.
pub fn generate_secure_token(byte_len: usize) -> Result<String, String> {
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; byte_len];
    rng.fill(&mut bytes).map_err(|e| format!("Failed to generate random bytes: {}", e))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Compute SHA-256 hash of `salt + ":" + password` in hex.
pub fn hash_password(password: &str, salt: &str) -> String {
    let payload = format!("{}:{}", salt, password);
    let hash = digest(&SHA256, payload.as_bytes());
    hash.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time password hash verification.
pub fn verify_password_hash(password: &str, salt: &str, expected_hash: &str) -> bool {
    if expected_hash.is_empty() {
        return false;
    }
    let actual_hash = hash_password(password, salt);
    let a = actual_hash.as_bytes();
    let b = expected_hash.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Authentication configuration stored in server state / persistent settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub basic_enabled: bool,
    pub basic_username: String,
    pub basic_password_hash: String,
    pub basic_password_salt: String,
    pub oauth_enabled: bool,
    pub oauth_provider_name: String,
    pub oauth_issuer_url: String,
    pub oauth_client_id: String,
    pub oauth_client_secret: Option<String>,
    pub oauth_allowed_emails: Vec<String>,
    pub oauth_redirect_uri: Option<String>,
    pub oauth_scopes: Vec<String>,
    pub allow_self_signed_oidc: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            basic_enabled: true,
            basic_username: "admin".to_string(),
            basic_password_hash: String::new(),
            basic_password_salt: String::new(),
            oauth_enabled: false,
            oauth_provider_name: "Authentik".to_string(),
            oauth_issuer_url: String::new(),
            oauth_client_id: String::new(),
            oauth_client_secret: None,
            oauth_allowed_emails: Vec::new(),
            oauth_redirect_uri: None,
            oauth_scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
            allow_self_signed_oidc: false,
        }
    }
}

impl AuthConfig {
    /// Sets a new password, generating a new random salt and SHA-256 hash.
    pub fn set_password(&mut self, password: &str) -> Result<(), String> {
        let salt = generate_secure_token(16)?;
        let hash = hash_password(password, &salt);
        self.basic_password_salt = salt;
        self.basic_password_hash = hash;
        Ok(())
    }

    /// Verifies username and password against configured basic auth.
    pub fn verify_basic_credentials(&self, username: &str, password: &str) -> bool {
        if !self.basic_enabled || self.basic_username.is_empty() {
            return false;
        }
        if self.basic_username != username {
            return false;
        }
        verify_password_hash(password, &self.basic_password_salt, &self.basic_password_hash)
    }

    /// Parses and verifies standard HTTP `Authorization: Basic <base64>` header.
    pub fn verify_basic_auth_header(&self, header_val: &str) -> Option<String> {
        let trimmed = header_val.trim();
        if !trimmed.to_ascii_lowercase().starts_with("basic ") {
            return None;
        }
        let b64 = trimmed[6..].trim();
        let decoded = base64_decode(b64)?;
        let credentials = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(_) => return None,
        };
        let (user, pass) = match credentials.split_once(':') {
            Some((u, p)) => (u, p),
            None => return None,
        };
        if self.verify_basic_credentials(user, pass) {
            Some(user.to_string())
        } else {
            None
        }
    }
}

/// Simple base64 decoder for Basic Auth.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: [i8; 256] = {
        let mut t = [-1i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < chars.len() {
            t[chars[i] as usize] = i as i8;
            i += 1;
        }
        t
    };

    let bytes = input.trim_end_matches('=').as_bytes();
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in bytes {
        let val = TABLE[b as usize];
        if val < 0 {
            if b == b' ' || b == b'\r' || b == b'\n' || b == b'\t' {
                continue;
            }
            return None;
        }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Active user session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user: String,
    pub auth_method: String, // "basic" | "oauth"
    pub created_at: u64,
    pub expires_at: u64,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

/// Thread-safe in-memory session store.
#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a new session valid for `duration_secs`.
    pub fn create_session(&self, user: &str, auth_method: &str, duration_secs: u64) -> Result<Session, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let session = Session {
            id: generate_secure_token(32)?,
            user: user.to_string(),
            auth_method: auth_method.to_string(),
            created_at: now,
            expires_at: now + duration_secs,
        };
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    /// Validates session token and returns session if active.
    pub fn validate_session(&self, token: &str) -> Option<Session> {
        let mut map = self.sessions.lock().unwrap();
        if let Some(session) = map.get(token) {
            if session.is_expired() {
                map.remove(token);
                None
            } else {
                Some(session.clone())
            }
        } else {
            None
        }
    }

    /// Invalidate/remove session on logout.
    pub fn remove_session(&self, token: &str) {
        let mut map = self.sessions.lock().unwrap();
        map.remove(token);
    }
}

/// Pending state during OIDC authorization code flow.
#[derive(Clone, Debug)]
pub struct OidcPendingState {
    pub pkce_verifier: String,
    pub nonce: String,
    pub expires_at: u64,
}

/// Manages OIDC state challenges and tokens.
#[derive(Clone, Default)]
pub struct OidcFlowManager {
    states: Arc<Mutex<HashMap<String, OidcPendingState>>>,
}

impl OidcFlowManager {
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert_state(&self, state: String, pkce_verifier: String, nonce: String, ttl_secs: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut map = self.states.lock().unwrap();
        map.insert(
            state,
            OidcPendingState {
                pkce_verifier,
                nonce,
                expires_at: now + ttl_secs,
            },
        );
    }

    pub fn take_state(&self, state: &str) -> Option<OidcPendingState> {
        let mut map = self.states.lock().unwrap();
        let pending = map.remove(state)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now < pending.expires_at {
            Some(pending)
        } else {
            None
        }
    }
}

/// Custom synchronous HTTP client dispatcher for openidconnect with optional self-signed TLS support.
pub fn oidc_http_client(
    req: openidconnect::HttpRequest,
    allow_self_signed: bool,
) -> Result<openidconnect::HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(30));

    if allow_self_signed {
        builder = builder.tls_config(std::sync::Arc::new(crate::agent::client::build_insecure_tls_client_config()));
    }

    let agent = builder.build();
    let mut ureq_req = agent.request_url(req.method.as_str(), &req.url);

    for (k, v) in req.headers.iter() {
        if let Ok(val_str) = v.to_str() {
            ureq_req = ureq_req.set(k.as_str(), val_str);
        }
    }

    let res = if req.body.is_empty() {
        ureq_req.call()
    } else {
        ureq_req.send_bytes(&req.body)
    };

    match res {
        Ok(response) => {
            let status = openidconnect::http::StatusCode::from_u16(response.status())?;
            let mut headers = openidconnect::http::HeaderMap::new();
            for name in response.headers_names() {
                if let Some(val) = response.header(&name) {
                    if let (Ok(hname), Ok(hval)) = (
                        openidconnect::http::header::HeaderName::from_bytes(name.as_bytes()),
                        openidconnect::http::header::HeaderValue::from_str(val),
                    ) {
                        headers.insert(hname, hval);
                    }
                }
            }

            let mut body = Vec::new();
            std::io::Read::read_to_end(&mut response.into_reader(), &mut body)?;

            Ok(openidconnect::HttpResponse {
                status_code: status,
                headers,
                body,
            })
        }
        Err(ureq::Error::Status(code, response)) => {
            let status = openidconnect::http::StatusCode::from_u16(code)?;
            let mut headers = openidconnect::http::HeaderMap::new();
            for name in response.headers_names() {
                if let Some(val) = response.header(&name) {
                    if let (Ok(hname), Ok(hval)) = (
                        openidconnect::http::header::HeaderName::from_bytes(name.as_bytes()),
                        openidconnect::http::header::HeaderValue::from_str(val),
                    ) {
                        headers.insert(hname, hval);
                    }
                }
            }
            let mut body = Vec::new();
            std::io::Read::read_to_end(&mut response.into_reader(), &mut body)?;
            Ok(openidconnect::HttpResponse {
                status_code: status,
                headers,
                body,
            })
        }
        Err(e) => Err(Box::new(e)),
    }
}

/// Initiates OpenID Connect Authorization Code Flow with PKCE.
pub fn build_oidc_authorization_url(
    config: &AuthConfig,
    flow_mgr: &OidcFlowManager,
    redirect_uri_override: Option<&str>,
) -> Result<String, String> {
    if !config.oauth_enabled || config.oauth_issuer_url.trim().is_empty() {
        return Err("OpenID Connect is not enabled or Issuer URL is empty".to_string());
    }

    let issuer_url = IssuerUrl::new(config.oauth_issuer_url.trim().to_string())
        .map_err(|e| format!("Invalid Issuer URL: {}", e))?;

    let allow_self_signed = config.allow_self_signed_oidc;
    let http_fn = move |req| oidc_http_client(req, allow_self_signed).map_err(|e| {
        openidconnect::ureq::Error::Other(e.to_string())
    });

    let provider_metadata = CoreProviderMetadata::discover(&issuer_url, http_fn)
        .map_err(|e| format!("Failed to discover OIDC provider metadata: {}", e))?;

    let redirect_url_str = redirect_uri_override
        .or(config.oauth_redirect_uri.as_deref())
        .unwrap_or("https://127.0.0.1:8080/api/auth/oauth/callback");

    let redirect_url = RedirectUrl::new(redirect_url_str.to_string())
        .map_err(|e| format!("Invalid Redirect URL: {}", e))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.oauth_client_id.trim().to_string()),
        config.oauth_client_secret.as_ref().map(|s| ClientSecret::new(s.trim().to_string())),
    )
    .set_redirect_uri(redirect_url);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut auth_req = client.authorize_url(
        CoreAuthenticationFlow::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );

    for scope_name in &config.oauth_scopes {
        if scope_name != "openid" {
            auth_req = auth_req.add_scope(Scope::new(scope_name.clone()));
        }
    }

    let (auth_url, csrf_token, nonce) = auth_req.set_pkce_challenge(pkce_challenge).url();

    // Store state with 10-minute TTL
    flow_mgr.insert_state(
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
        nonce.secret().clone(),
        600,
    );

    Ok(auth_url.to_string())
}

/// Exchanges authorization code for tokens and verifies ID token / User claims.
pub fn handle_oidc_callback(
    config: &AuthConfig,
    flow_mgr: &OidcFlowManager,
    code: &str,
    state: &str,
    redirect_uri_override: Option<&str>,
) -> Result<String, String> {
    let pending_state = flow_mgr
        .take_state(state)
        .ok_or_else(|| "Invalid or expired OAuth state parameter".to_string())?;

    let issuer_url = IssuerUrl::new(config.oauth_issuer_url.trim().to_string())
        .map_err(|e| format!("Invalid Issuer URL: {}", e))?;

    let allow_self_signed = config.allow_self_signed_oidc;
    let http_fn = move |req| oidc_http_client(req, allow_self_signed).map_err(|e| {
        openidconnect::ureq::Error::Other(e.to_string())
    });

    let provider_metadata = CoreProviderMetadata::discover(&issuer_url, http_fn)
        .map_err(|e| format!("Failed to discover OIDC provider metadata: {}", e))?;

    let redirect_url_str = redirect_uri_override
        .or(config.oauth_redirect_uri.as_deref())
        .unwrap_or("https://127.0.0.1:8080/api/auth/oauth/callback");

    let redirect_url = RedirectUrl::new(redirect_url_str.to_string())
        .map_err(|e| format!("Invalid Redirect URL: {}", e))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.oauth_client_id.trim().to_string()),
        config.oauth_client_secret.as_ref().map(|s| ClientSecret::new(s.trim().to_string())),
    )
    .set_redirect_uri(redirect_url);

    let pkce_verifier = PkceCodeVerifier::new(pending_state.pkce_verifier);
    let expected_nonce = Nonce::new(pending_state.nonce);

    let token_response = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request(http_fn)
        .map_err(|e| format!("OAuth token exchange failed: {}", e))?;

    // Verify ID Token if present
    let id_token = token_response
        .id_token()
        .ok_or_else(|| "Provider did not return an ID token".to_string())?;

    let claims = id_token
        .claims(&client.id_token_verifier(), &expected_nonce)
        .map_err(|e| format!("ID token verification failed: {}", e))?;

    // Extract user email or subject
    let email = claims
        .email()
        .map(|e| e.as_str().to_string())
        .or_else(|| claims.preferred_username().map(|u| u.as_str().to_string()))
        .unwrap_or_else(|| claims.subject().as_str().to_string());

    // Check allow-list if configured
    if !config.oauth_allowed_emails.is_empty() {
        let allowed = config.oauth_allowed_emails.iter().any(|allowed_entry| {
            let entry = allowed_entry.trim().to_lowercase();
            if entry.is_empty() {
                return false;
            }
            email.to_lowercase() == entry
                || (entry.starts_with('*') && email.to_lowercase().ends_with(&entry[1..]))
        });

        if !allowed {
            return Err(format!("User '{}' is not authorized to access this Fishdoc instance", email));
        }
    }

    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verification() {
        let mut config = AuthConfig::default();
        config.set_password("MySecretPass123!").unwrap();

        assert!(config.verify_basic_credentials("admin", "MySecretPass123!"));
        assert!(!config.verify_basic_credentials("admin", "WrongPass"));
        assert!(!config.verify_basic_credentials("other_user", "MySecretPass123!"));
    }

    #[test]
    fn test_basic_auth_header_parser() {
        let mut config = AuthConfig::default();
        config.basic_username = "alice".to_string();
        config.set_password("wonderland").unwrap();

        // Basic YWxpY2U6d29uZGVybGFuZA== is "alice:wonderland"
        let valid_header = "Basic YWxpY2U6d29uZGVybGFuZA==";
        assert_eq!(config.verify_basic_auth_header(valid_header), Some("alice".to_string()));

        let invalid_header = "Basic d3Jvbmc6Y3JlZHM="; // "wrong:creds"
        assert_eq!(config.verify_basic_auth_header(invalid_header), None);
    }

    #[test]
    fn test_session_store_lifecycle() {
        let store = SessionStore::new();
        let session = store.create_session("alice", "basic", 3600).unwrap();
        assert_eq!(session.user, "alice");

        let validated = store.validate_session(&session.id);
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().user, "alice");

        store.remove_session(&session.id);
        assert!(store.validate_session(&session.id).is_none());
    }

    #[test]
    fn test_oidc_flow_manager_state_expiry() {
        let flow_mgr = OidcFlowManager::new();
        flow_mgr.insert_state("state123".to_string(), "verifier_abc".to_string(), "nonce_xyz".to_string(), 600);

        let retrieved = flow_mgr.take_state("state123");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().pkce_verifier, "verifier_abc");

        // Second take should be None
        assert!(flow_mgr.take_state("state123").is_none());
    }
}
