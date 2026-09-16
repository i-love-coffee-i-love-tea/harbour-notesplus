use std::os::raw::c_char;

use crate::agent::LlmConfig;
use crate::server;
use super::common::{cstr_to_path, cstr_to_string, string_to_c};

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

    let llm_config: Option<LlmConfig> = serde_json::from_str(&cfg_str).ok();

    match server::start_server_full(ndir, db, backup, port, llm_config, None) {
        Ok(handle) => Box::into_raw(Box::new(handle)),
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
        Err(e) => string_to_c(format!("ERROR: {}", e)),
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
        Err(e) => string_to_c(format!("ERROR: {}", e)),
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
