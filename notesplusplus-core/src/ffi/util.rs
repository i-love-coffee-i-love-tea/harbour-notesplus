use std::os::raw::c_char;

use crate::agent;
use crate::server;
use super::common::{cstr_to_string, string_to_c};

/// Get network interfaces as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_get_network_interfaces_json() -> *mut c_char {
    let interfaces = server::get_network_interfaces();
    string_to_c(serde_json::to_string(&interfaces).unwrap_or_else(|_| "[]".to_string()))
}

/// Fetch URL content. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_fetch_url(url: *const c_char) -> *mut c_char {
    let url = unsafe { cstr_to_string(url) };
    match agent::fetch_url(&url) {
        Ok(content) => string_to_c(content),
        Err(e) => string_to_c(format!("ERROR: {}", e)),
    }
}
