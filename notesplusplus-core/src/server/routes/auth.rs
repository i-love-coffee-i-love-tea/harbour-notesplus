use std::io::Write;
use serde_json::json;

use crate::constants::{MIME_JSON, SESSION_COOKIE_NAME, SESSION_EXPIRY_SECS};
use crate::server::auth::{self, AuthConfig, Session, SessionStore};
use crate::server::http::{
    extract_cookie_value, make_session_cookie, send_redirect, send_response, send_response_full,
    ParsedHttpRequest,
};
use crate::server::ServerContext;

pub fn authenticate_request(
    req: &ParsedHttpRequest,
    auth_config: &AuthConfig,
    session_store: &SessionStore,
) -> Option<Session> {
    if !auth_config.enabled {
        return Some(Session {
            id: "anonymous".to_string(),
            user: "anonymous".to_string(),
            auth_method: "none".to_string(),
            created_at: 0,
            expires_at: u64::MAX,
        });
    }

    // 1. Check Session Cookie
    if let Some(cookie_hdr) = req.headers.get("cookie") {
        if let Some(session_id) = extract_cookie_value(cookie_hdr, SESSION_COOKIE_NAME) {
            if let Some(sess) = session_store.validate_session(&session_id) {
                return Some(sess);
            }
        }
    }

    // 2. Check Authorization Header (Bearer token or Basic Auth)
    if let Some(auth_hdr) = req.headers.get("authorization") {
        let auth_hdr = auth_hdr.trim();
        if let Some(token) = auth_hdr.strip_prefix("Bearer ") {
            let token = token.trim();
            if let Some(sess) = session_store.validate_session(token) {
                return Some(sess);
            }
        } else if let Some(basic) = auth_hdr.strip_prefix("Basic ") {
            if let Ok(decoded) = base64_decode(basic.trim()) {
                if let Ok(cred_str) = String::from_utf8(decoded) {
                    if let Some((user, pass)) = cred_str.split_once(':') {
                        if auth_config.verify_basic_credentials(user, pass) {
                            return Some(Session {
                                id: "basic_header".to_string(),
                                user: user.to_string(),
                                auth_method: "basic".to_string(),
                                created_at: 0,
                                expires_at: u64::MAX,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input.as_bytes() {
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return Err("Invalid base64 character".to_string()),
        } as u32;

        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

pub fn handle_auth_config<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let session_store = ctx.session_store.clone();

    match req.method.as_str() {
        "GET" => {
            let session = authenticate_request(req, &auth_config, &session_store);
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
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        "POST" => {
            if auth_config.enabled {
                let session = authenticate_request(req, &auth_config, &session_store);
                if session.is_none() {
                    let err = json!({ "ok": false, "error": "Unauthorized" });
                    send_response(stream, 401, "Unauthorized", MIME_JSON, err.to_string().as_bytes(), cors_origin);
                    return;
                }
            }
            if let Ok(mut new_cfg) = serde_json::from_slice::<AuthConfig>(&req.body) {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    if let Some(pass) = val.get("password").and_then(|v| v.as_str()) {
                        if !pass.is_empty() {
                            let _ = new_cfg.set_password(pass);
                        }
                    }
                }
                ctx.update_auth_config(new_cfg);
                let resp = json!({ "ok": true });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                return;
            }
            let err = json!({ "ok": false, "error": "Invalid auth config payload" });
            send_response(stream, 400, "Bad Request", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        }
        _ => {}
    }
}

pub fn handle_login<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let body_str = String::from_utf8_lossy(&req.body);
    let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
    let username = json_body.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = json_body.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if auth_config.verify_basic_credentials(username, password) {
        let ttl_secs = 30 * 24 * 3600; // 30 days
        if let Ok(sess) = ctx.session_store.create_session(username, "basic", ttl_secs) {
            let cookie_str = make_session_cookie(&sess.id, ctx.is_tls, Some(ttl_secs));
            let resp = json!({
                "ok": true,
                "session_id": sess.id,
                "user": sess.user
            });
            send_response_full(
                stream,
                200,
                "OK",
                MIME_JSON,
                resp.to_string().as_bytes(),
                cors_origin,
                &[("Set-Cookie", &cookie_str)],
            );
            return;
        }
    }
    let resp = json!({ "ok": false, "error": "Invalid username or password" });
    send_response(stream, 401, "Unauthorized", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
}

pub fn handle_logout<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if let Some(cookie_hdr) = req.headers.get("cookie") {
        if let Some(session_id) = extract_cookie_value(cookie_hdr, SESSION_COOKIE_NAME) {
            ctx.session_store.remove_session(&session_id);
        }
    }
    let cookie_str = make_session_cookie("", ctx.is_tls, Some(0));
    let resp = json!({ "ok": true });
    send_response_full(
        stream,
        200,
        "OK",
        MIME_JSON,
        resp.to_string().as_bytes(),
        cors_origin,
        &[("Set-Cookie", &cookie_str)],
    );
}

pub fn handle_whoami<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let session = authenticate_request(req, &auth_config, &ctx.session_store);
    let resp = json!({
        "authenticated": session.is_some(),
        "user": session.as_ref().map(|s| s.user.clone()).unwrap_or_default(),
        "auth_method": session.as_ref().map(|s| s.auth_method.clone()).unwrap_or_default(),
    });
    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
}

pub fn handle_oauth_start<W: Write>(
    stream: &mut W,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let default_redirect = "/";
    match auth::build_oidc_authorization_url(&auth_config, &ctx.oidc_flow_mgr, Some(&default_redirect)) {
        Ok(auth_url) => {
            if clean_path == "api/auth/oauth/login" {
                send_redirect(stream, &auth_url, cors_origin, &[]);
            } else {
                let resp = json!({ "ok": true, "authorization_url": auth_url });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            }
        }
        Err(err) => {
            let resp = json!({ "ok": false, "error": err });
            send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}

pub fn handle_oauth_callback<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let host = req.headers.get("host").map(|s| s.as_str()).unwrap_or("127.0.0.1");
    let scheme = if ctx.is_tls { "https" } else { "http" };
    let callback_url = format!("{}://{}/api/auth/oauth/callback", scheme, host);

    let query_str = req.query.clone().unwrap_or_default();
    let mut code = String::new();
    let mut state = String::new();
    for part in query_str.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            if k == "code" {
                code = v.to_string();
            } else if k == "state" {
                state = v.to_string();
            }
        }
    }

    if code.is_empty() || state.is_empty() {
        let err = json!({ "ok": false, "error": "Missing code or state in OAuth callback" });
        send_response(stream, 400, "Bad Request", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        return;
    }

    let auth_config = ctx.auth_config.lock().unwrap_or_else(|e| e.into_inner()).clone();
    match auth::handle_oidc_callback(
        &auth_config,
        &ctx.oidc_flow_mgr,
        &code,
        &state,
        Some(&callback_url),
    ) {
        Ok(email) => {
            let ttl = SESSION_EXPIRY_SECS;
            match ctx.session_store.create_session(&email, "oidc", ttl) {
                Ok(sess) => {
                    let cookie_str = make_session_cookie(&sess.id, ctx.is_tls, Some(ttl));
                    let redirect_target = "/";
                    send_redirect(stream, redirect_target, cors_origin, &[("Set-Cookie", &cookie_str)]);
                }
                Err(e) => {
                    let err = json!({ "ok": false, "error": e });
                    send_response(stream, 500, "Internal Server Error", MIME_JSON, err.to_string().as_bytes(), cors_origin);
                }
            }
        }
        Err(err) => {
            let err_json = json!({ "ok": false, "error": err });
            send_response(stream, 400, "Bad Request", MIME_JSON, err_json.to_string().as_bytes(), cors_origin);
        }
    }
}
