use std::io::Write;
use std::time::Duration;
use serde_json::json;

use crate::constants::{AUTH_CHALLENGE_TTL_SECS, MIME_JSON, SESSION_COOKIE_NAME};
use crate::server::auth::{current_epoch_secs, Session, SessionStore};
use crate::server::http::{
    extract_cookie_value, make_session_cookie, send_json_ok, send_response, send_response_full,
    ParsedHttpRequest,
};
use crate::server::ServerContext;

pub fn authenticate_request(
    req: &ParsedHttpRequest,
    session_store: &SessionStore,
) -> Option<Session> {
    // 1. Check Session Cookie
    if let Some(cookie_hdr) = req.headers.get("cookie") {
        if let Some(session_id) = extract_cookie_value(cookie_hdr, SESSION_COOKIE_NAME) {
            if let Some(sess) = session_store.validate_session(&session_id) {
                return Some(sess);
            }
        }
    }

    // 2. Check Authorization Header (Bearer token)
    if let Some(auth_hdr) = req.headers.get("authorization") {
        let auth_hdr = auth_hdr.trim();
        if let Some(token) = auth_hdr.strip_prefix("Bearer ") {
            let token = token.trim();
            if let Some(sess) = session_store.validate_session(token) {
                return Some(sess);
            }
        }
    }

    None
}

pub fn handle_auth_config<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let session_store = ctx.session_store.clone();
    let session = authenticate_request(req, &session_store);
    let now = current_epoch_secs();

    let resp = json!({
        "auth_required": true,
        "authenticated": session.is_some(),
        "user": session.as_ref().map(|s| s.user.clone()).unwrap_or_default(),
        "expires_at": session.as_ref().map(|s| s.expires_at).unwrap_or(0),
        "remaining_secs": session.as_ref().map(|s| s.expires_at.saturating_sub(now)).unwrap_or(0)
    });
    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
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
    if let Some(auth_hdr) = req.headers.get("authorization") {
        let auth_hdr = auth_hdr.trim();
        if let Some(token) = auth_hdr.strip_prefix("Bearer ") {
            let token = token.trim();
            ctx.session_store.remove_session(token);
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
    let session = authenticate_request(req, &ctx.session_store);
    let now = current_epoch_secs();

    let resp = json!({
        "authenticated": session.is_some(),
        "user": session.as_ref().map(|s| s.user.clone()).unwrap_or_default(),
        "auth_method": session.as_ref().map(|s| s.auth_method.clone()).unwrap_or_default(),
        "expires_at": session.as_ref().map(|s| s.expires_at).unwrap_or(0),
        "remaining_secs": session.as_ref().map(|s| s.expires_at.saturating_sub(now)).unwrap_or(0)
    });
    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
}

/// POST /api/auth/code/initiate — Browser requests a verification code authorization challenge.
pub fn handle_challenge_initiate<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    // Rate limit: max 10 requests per 60 seconds per client IP
    let client_ip = req.client_ip();
    if let Err(retry_after) = ctx.rate_limiter.check("auth_initiate", client_ip, 10, Duration::from_secs(60)) {
        let retry_after_str = retry_after.to_string();
        let err = json!({
            "ok": false,
            "error": format!("Too many authentication requests. Please try again in {} seconds.", retry_after),
            "retry_after": retry_after
        });
        send_response_full(
            stream,
            429,
            "Too Many Requests",
            MIME_JSON,
            err.to_string().as_bytes(),
            cors_origin,
            &[("Retry-After", &retry_after_str)],
        );
        return;
    }

    match ctx.auth_challenges.create_challenge(AUTH_CHALLENGE_TTL_SECS) {
        Ok(challenge) => {
            ctx.signal_auth_challenge(challenge.challenge_id.clone());
            let resp = json!({
                "ok": true,
                "challenge_id": challenge.challenge_id,
                "verification_code": challenge.verification_code,
                "expires_in": AUTH_CHALLENGE_TTL_SECS
            });
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        Err(e) => {
            log::error!("Auth challenge creation failed: {}", e);
            let err = json!({ "ok": false, "error": "Failed to create authentication challenge" });
            send_response(stream, 500, "Internal Server Error", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        }
    }
}

/// GET /api/auth/code/status?challenge_id=X — Browser polls for challenge approval result.
pub fn handle_challenge_status<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let query_str = req.query.clone().unwrap_or_default();
    let challenge_id = query_str
        .split('&')
        .find_map(|part| {
            let (k, v) = part.split_once('=')?;
            if k == "challenge_id" { Some(v.to_string()) } else { None }
        })
        .unwrap_or_default();

    if challenge_id.is_empty() {
        let err = json!({ "ok": false, "error": "Missing challenge_id parameter" });
        send_response(stream, 400, "Bad Request", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        return;
    }

    match ctx.auth_challenges.get_challenge(&challenge_id) {
        Some(challenge) => {
            if challenge.status == "approved" {
                // Create a session and return it
                let ttl = ctx.session_expiry_secs();
                match ctx.session_store.create_session("web-user", "code", ttl) {
                    Ok(sess) => {
                        // Clean up the challenge
                        ctx.auth_challenges.remove_challenge(&challenge_id);
                        let cookie_str = make_session_cookie(&sess.id, ctx.is_tls, Some(ttl));
                        let now = current_epoch_secs();
                        let remaining_secs = sess.expires_at.saturating_sub(now);
                        let resp = json!({
                            "status": "approved",
                            "session_id": sess.id,
                            "user": sess.user,
                            "expires_at": sess.expires_at,
                            "remaining_secs": remaining_secs
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
                    }
                    Err(e) => {
                        log::error!("Session creation failed: {}", e);
                        let err = json!({ "status": "error", "error": "Failed to create session" });
                        send_response(stream, 500, "Internal Server Error", MIME_JSON, err.to_string().as_bytes(), cors_origin);
                    }
                }
            } else if challenge.status == "denied" {
                ctx.auth_challenges.remove_challenge(&challenge_id);
                let resp = json!({ "status": "denied" });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            } else if challenge.is_expired() {
                ctx.auth_challenges.remove_challenge(&challenge_id);
                let resp = json!({ "status": "expired" });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            } else {
                let resp = json!({ "status": "pending" });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            }
        }
        None => {
            let err = json!({ "ok": false, "error": "Challenge not found or already consumed" });
            send_response(stream, 404, "Not Found", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        }
    }
}

/// POST /api/auth/code/approve — QML app approves an authorization challenge (localhost only).
pub fn handle_challenge_approve<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let challenge_id = json_body.get("challenge_id").and_then(|v| v.as_str()).unwrap_or("");

    if challenge_id.is_empty() {
        let err = json!({ "ok": false, "error": "Missing challenge_id in payload" });
        send_response(stream, 400, "Bad Request", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        return;
    }

    if ctx.auth_challenges.approve_challenge(challenge_id) {
        ctx.clear_auth_challenge();
        let resp = json!({ "ok": true, "message": "Challenge approved successfully" });
        send_json_ok(stream, &resp, cors_origin);
    } else {
        let err = json!({ "ok": false, "error": "Challenge not found, expired, or not pending" });
        send_response(stream, 404, "Not Found", MIME_JSON, err.to_string().as_bytes(), cors_origin);
    }
}

/// POST /api/auth/code/deny — QML app denies an authorization challenge (localhost only).
pub fn handle_challenge_deny<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let challenge_id = json_body.get("challenge_id").and_then(|v| v.as_str()).unwrap_or("");

    if challenge_id.is_empty() {
        let err = json!({ "ok": false, "error": "Missing challenge_id in payload" });
        send_response(stream, 400, "Bad Request", MIME_JSON, err.to_string().as_bytes(), cors_origin);
        return;
    }

    if ctx.auth_challenges.deny_challenge(challenge_id) {
        ctx.clear_auth_challenge();
        let resp = json!({ "ok": true, "message": "Challenge denied" });
        send_json_ok(stream, &resp, cors_origin);
    } else {
        let err = json!({ "ok": false, "error": "Challenge not found or not pending" });
        send_response(stream, 404, "Not Found", MIME_JSON, err.to_string().as_bytes(), cors_origin);
    }
}
