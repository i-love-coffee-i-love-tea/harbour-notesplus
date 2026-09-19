use std::collections::HashMap;
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::agent::LlmConfig;
use crate::server;
use super::common::{cstr_to_path, cstr_to_string, ffi_err, string_to_c};

/// Callback type for external PDF exporter (e.g. Qt QTextDocument + QPdfWriter in C++ bridge).
/// Returns 0 on success, non-zero on error.
pub type PdfExporterCallback = unsafe extern "C" fn(
    note_rel_path: *const c_char,
    out_pdf_path: *const c_char,
) -> i32;

/// Start the web server. Returns opaque handle or NULL on failure.
#[no_mangle]
pub extern "C" fn notes_core_server_start(
    notes_dir: *const c_char,
    db_path: *const c_char,
    backup_dir: *const c_char,
    port: u16,
    config_json: *const c_char,
) -> *mut server::HttpServerHandle {
    let ndir = unsafe { cstr_to_path(notes_dir) };
    let db = unsafe { cstr_to_path(db_path) };
    let backup = unsafe { cstr_to_path(backup_dir) };
    let cfg_str = unsafe { cstr_to_string(config_json) };

    // Parse full config JSON to extract TLS and LLM settings
    let parsed: Option<serde_json::Value> = serde_json::from_str(&cfg_str).ok();
    let llm_config: Option<LlmConfig> = parsed.as_ref()
        .and_then(|v| v.get("llm"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let tls_cert_path = parsed.as_ref()
        .and_then(|v| v.get("tls_cert_path"))
        .and_then(|v| v.as_str())
        .map(|s| PathBuf::from(s));

    let tls_key_path = parsed.as_ref()
        .and_then(|v| v.get("tls_key_path"))
        .and_then(|v| v.as_str())
        .map(|s| PathBuf::from(s));

    // Parse auth config from JSON
    let auth_expiry_secs = parsed.as_ref()
        .and_then(|v| v.get("auth"))
        .and_then(|v| v.get("session_expiry_secs"))
        .and_then(|v| v.as_u64());

    // Parse permissions from JSON
    let permission_config = parsed.as_ref()
        .and_then(|v| v.get("permissions"))
        .and_then(|v| serde_json::from_value::<crate::agent::PermissionConfig>(v.clone()).ok());

    // Parse theme from JSON
    let theme_colors: Option<HashMap<String, String>> = parsed.as_ref()
        .and_then(|v| v.get("theme"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    match server::start_server_full(
        ndir, db, backup, port, llm_config, permission_config,
        tls_cert_path, tls_key_path,
    ) {
        Ok(handle) => {
            // Apply auth config if provided
            if let Some(expiry) = auth_expiry_secs {
                handle.context().set_session_expiry_secs(expiry);
            }
            // Apply theme colors if provided
            if let Some(theme) = theme_colors {
                handle.context().set_theme_colors(theme);
            }
            Box::into_raw(Box::new(handle))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn notes_core_server_stop(handle: *mut server::HttpServerHandle) {
    if !handle.is_null() {
        unsafe {
            let h = Box::from_raw(handle);
            h.stop();
        }
    }
}

#[no_mangle]
pub extern "C" fn notes_core_server_is_running(
    handle: *const server::HttpServerHandle,
) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    if h.is_running() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn notes_core_server_port(
    handle: *const server::HttpServerHandle,
) -> u16 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    h.port()
}

/// Get server URLs as JSON array. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_server_urls_json(
    handle: *const server::HttpServerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_c("[]".to_string());
    }
    let h = unsafe { &*handle };
    let urls: Vec<String> = h.urls().to_vec();
    string_to_c(serde_json::to_string(&urls).unwrap_or_else(|_| "[]".to_string()))
}

/// Install TLS certificate. Returns allocated JSON result or error string.
#[no_mangle]
pub extern "C" fn notes_core_server_tls_install(
    cert_path: *const c_char,
    key_path: *const c_char,
    cert_pem: *const c_char,
    key_pem: *const c_char,
) -> *mut c_char {
    let cp = unsafe { cstr_to_path(cert_path) };
    let kp = unsafe { cstr_to_path(key_path) };
    let cert = unsafe { cstr_to_string(cert_pem) };
    let key = unsafe { cstr_to_string(key_pem) };
    match server::tls::install_custom_tls_cert(&cert, &key, &cp, &kp) {
        Ok(tc) => {
            let info = serde_json::json!({
                "cert_pem": tc.cert_pem,
                "subject": "custom",
            });
            string_to_c(serde_json::to_string(&info).unwrap_or_default())
        }
        Err(e) => ffi_err!(e),
    }
}

/// Reset to self-signed certificate. Returns allocated JSON result.
#[no_mangle]
pub extern "C" fn notes_core_server_tls_reset(
    cert_path: *const c_char,
    key_path: *const c_char,
) -> *mut c_char {
    let cp = unsafe { cstr_to_path(cert_path) };
    let kp = unsafe { cstr_to_path(key_path) };
    match server::tls::reset_to_self_signed_cert(&cp, &kp, None) {
        Ok(tc) => {
            let info = serde_json::json!({
                "cert_pem": tc.cert_pem,
                "subject": "self-signed",
            });
            string_to_c(serde_json::to_string(&info).unwrap_or_default())
        }
        Err(e) => ffi_err!(e),
    }
}

/// Check if custom TLS cert is installed.
#[no_mangle]
pub extern "C" fn notes_core_server_tls_is_custom(
    cert_path: *const c_char,
) -> i32 {
    let cp = unsafe { cstr_to_path(cert_path) };
    if server::tls::is_custom_cert_installed(&cp) { 1 } else { 0 }
}

/// Take the pending auth challenge signal from the server.
/// Returns JSON: {"challenge_id":"...","verification_code":"..."} or {"pending":false}
/// The signal is cleared after taking.
#[no_mangle]
pub extern "C" fn notes_core_server_auth_take_challenge(
    handle: *const server::HttpServerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_c(r#"{"pending":false}"#.to_string());
    }
    let h = unsafe { &*handle };
    let ctx = h.context();
    match ctx.take_auth_challenge() {
        Some(challenge_id) => {
            match ctx.auth_challenges.get_challenge(&challenge_id) {
                Some(challenge) => {
                    let json = serde_json::json!({
                        "pending": true,
                        "challenge_id": challenge.challenge_id,
                        "verification_code": challenge.verification_code,
                    });
                    string_to_c(json.to_string())
                }
                None => string_to_c(r#"{"pending":false}"#.to_string()),
            }
        }
        None => string_to_c(r#"{"pending":false}"#.to_string()),
    }
}

/// Approve a pending auth challenge in the Rust challenge store.
#[no_mangle]
pub extern "C" fn notes_core_server_auth_approve(
    handle: *const server::HttpServerHandle,
    challenge_id: *const c_char,
) -> i32 {
    if handle.is_null() || challenge_id.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    let id = unsafe { cstr_to_string(challenge_id) };
    if h.context().auth_challenges.approve_challenge(&id) { 1 } else { 0 }
}

/// Deny a pending auth challenge in the Rust challenge store.
#[no_mangle]
pub extern "C" fn notes_core_server_auth_deny(
    handle: *const server::HttpServerHandle,
    challenge_id: *const c_char,
) -> i32 {
    if handle.is_null() || challenge_id.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    let id = unsafe { cstr_to_string(challenge_id) };
    if h.context().auth_challenges.deny_challenge(&id) { 1 } else { 0 }
}

/// Register or clear the PDF exporter callback on the running server handle.
#[no_mangle]
pub extern "C" fn notes_core_server_set_pdf_exporter(
    handle: *mut server::HttpServerHandle,
    callback: Option<PdfExporterCallback>,
) {
    if !handle.is_null() {
        let h = unsafe { &mut *handle };
        if let Some(cb) = callback {
            h.context().set_pdf_exporter_callback(cb);
        } else {
            h.context().clear_pdf_exporter();
        }
    }
}
