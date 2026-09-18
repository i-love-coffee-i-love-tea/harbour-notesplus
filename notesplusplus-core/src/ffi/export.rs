use std::os::raw::c_char;

use crate::html;
use super::common::{cstr_to_path, cstr_to_string, ffi_err, string_to_c};

/// Render a single page to HTML5 in memory. Returns allocated C string (caller frees) or error.
#[no_mangle]
pub extern "C" fn notes_core_render_page_html5(
    notes_dir: *const c_char,
    rel_path: *const c_char,
) -> *mut c_char {
    let ndir = unsafe { cstr_to_path(notes_dir) };
    let rel = unsafe { cstr_to_string(rel_path) };
    let assets = ndir.join("assets");
    match html::render_page_to_html5_string(&ndir, &assets, &rel) {
        Ok(html_str) => string_to_c(html_str),
        Err(e) => ffi_err!(e),
    }
}

/// Export a single page to HTML5. Returns allocated output path or error.
#[no_mangle]
pub extern "C" fn notes_core_export_html5(
    notes_dir: *const c_char,
    _rel_path: *const c_char,
    full_path: *const c_char,
    output_path: *const c_char,
) -> *mut c_char {
    let ndir = unsafe { cstr_to_path(notes_dir) };
    let full = unsafe { cstr_to_path(full_path) };
    let out = unsafe { cstr_to_path(output_path) };
    let assets = ndir.join("assets");
    match html::export_page_to_html5(&ndir, &assets, &full.to_string_lossy(), &out) {
        Ok(p) => string_to_c(p.to_string_lossy().into_owned()),
        Err(e) => ffi_err!(e),
    }
}

/// Export all pages to HTML5. Returns allocated JSON array of output paths.
#[no_mangle]
pub extern "C" fn notes_core_export_all_html5(
    notes_dir: *const c_char,
    output_dir: *const c_char,
) -> *mut c_char {
    let ndir = unsafe { cstr_to_path(notes_dir) };
    let out = unsafe { cstr_to_path(output_dir) };
    let assets = ndir.join("assets");
    match html::export_all_pages_to_html5(&ndir, &assets, &out) {
        Ok(paths) => {
            let strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            string_to_c(serde_json::to_string(&strs).unwrap_or_else(|_| "[]".to_string()))
        }
        Err(e) => ffi_err!(e),
    }
}
