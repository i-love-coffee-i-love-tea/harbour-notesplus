use std::collections::HashMap;
use std::os::raw::c_char;

use crate::html;
use crate::html::qt_html::QtThemeColors;
use crate::page;
use crate::parser;
use super::common::{
    blocks_to_json_with_html, cstr_to_path, cstr_to_string, ffi_err, parse_qt_render_options,
    string_to_c,
};

/// Get page source content. Returns allocated string (caller frees).
#[no_mangle]
pub extern "C" fn notes_core_page_get_source(
    notes_dir: *const c_char,
    name: *const c_char,
) -> *mut c_char {
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    match page::read_page(&dir, &name) {
        Ok(content) => string_to_c(content),
        Err(e) => ffi_err!(e),
    }
}

/// Save page content and index it. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_save_source(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    name: *const c_char,
    content: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    let content = unsafe { cstr_to_string(content) };
    match page::save_and_index_page(conn, &dir, &name, &content) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Create a new page. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_create(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    name: *const c_char,
    color: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    let color_str = if color.is_null() {
        None
    } else {
        let s = unsafe { cstr_to_string(color) };
        if s.is_empty() { None } else { Some(s) }
    };
    match page::create_page(conn, &dir, &name, false, color_str.as_deref()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Delete a page. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_delete(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    name: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    match page::delete_page(conn, &dir, &name) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Rename a page. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_rename(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    name: *const c_char,
    new_title: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    let new_title = unsafe { cstr_to_string(new_title) };
    match page::rename_page(conn, &dir, &name, &new_title) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Move a page to a different group. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_move(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    source_name: *const c_char,
    target_group: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let source = unsafe { cstr_to_string(source_name) };
    let target = unsafe { cstr_to_string(target_group) };
    match page::move_page(conn, &dir, &source, &target) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Set page color. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_set_color(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    name: *const c_char,
    color: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    let color_str = if color.is_null() {
        None
    } else {
        let s = unsafe { cstr_to_string(color) };
        if s.is_empty() { None } else { Some(s) }
    };
    match page::set_page_color(conn, &dir, &name, color_str.as_deref()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Extract title from content. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_page_extract_title(
    content: *const c_char,
    fallback: *const c_char,
) -> *mut c_char {
    let content = unsafe { cstr_to_string(content) };
    let fallback = unsafe { cstr_to_string(fallback) };
    string_to_c(page::extract_doc_title(&content, &fallback))
}

/// Rebuild the full-text search index. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_rebuild_index(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    match page::rebuild_index(conn, &dir) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get list of recent pages as JSON array. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_recent_pages_json(
    conn: *mut rusqlite::Connection,
    limit: i32,
) -> *mut c_char {
    let conn = match unsafe { conn.as_ref() } {
        Some(c) => c,
        None => return ffi_err!("null connection"),
    };
    let limit = if limit > 0 { limit as usize } else { crate::constants::DEFAULT_RECENT_PAGES_LIMIT };
    match page::recent_pages(conn, limit) {
        Ok(pages) => {
            let values: Vec<serde_json::Value> = pages.iter().map(|p| p.to_json_value()).collect();
            string_to_c(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()))
        }
        Err(_) => string_to_c("[]".to_string()),
    }
}

/// Parse AsciiDoc content into block JSON. Returns allocated JSON string.
#[no_mangle]
pub extern "C" fn notes_core_parse_blocks_json(
    adoc: *const c_char,
    drop_comments: i32,
) -> *mut c_char {
    let adoc = unsafe { cstr_to_string(adoc) };
    let drop = drop_comments != 0;
    let blocks = parser::parse_blocks_with_options(&adoc, drop);
    let values: Vec<serde_json::Value> = blocks.iter().map(|b| b.to_qvariant_map()).collect();
    string_to_c(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()))
}

/// Parse and render blocks with Qt HTML. Returns allocated JSON string.
#[no_mangle]
pub extern "C" fn notes_core_page_parse_and_render_blocks_json(
    adoc_content: *const c_char,
    notes_dir: *const c_char,
    drop_comments: i32,
    theme_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let adoc = unsafe { cstr_to_string(adoc_content) };
    let dir = unsafe { cstr_to_string(notes_dir) };
    let drop = drop_comments != 0;
    let theme_str = unsafe { cstr_to_string(theme_json) };
    let opts_str = unsafe { cstr_to_string(options_json) };

    let theme: HashMap<String, String> =
        serde_json::from_str(&theme_str).unwrap_or_default();
    let qt_theme = QtThemeColors::from_map(&theme);
    let qt_options = parse_qt_render_options(&opts_str, Some(dir));

    let blocks = parser::parse_blocks_with_options(&adoc, drop);
    let values = blocks_to_json_with_html(&blocks, &qt_theme, &qt_options);
    string_to_c(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()))
}

/// Render a single block to Qt HTML. Returns allocated JSON string.
#[no_mangle]
pub extern "C" fn notes_core_render_qt_block_json(
    block_json: *const c_char,
    index: i32,
    theme_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let block_str = unsafe { cstr_to_string(block_json) };
    let theme_str = unsafe { cstr_to_string(theme_json) };
    let opts_str = unsafe { cstr_to_string(options_json) };

    let block_val: serde_json::Value = match serde_json::from_str(&block_str) {
        Ok(v) => v,
        Err(_) => return string_to_c(r#"{"error":"invalid block json"}"#.to_string()),
    };

    let theme: HashMap<String, String> =
        serde_json::from_str(&theme_str).unwrap_or_default();
    let qt_theme = QtThemeColors::from_map(&theme);
    let qt_options = parse_qt_render_options(&opts_str, None);

    // Reconstruct block from JSON values
    let adoc = block_val
        .get("raw")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let blocks = parser::parse_blocks(adoc);
    if let Some(block) = blocks.first() {
        let html = html::qt_html::render_qt_block(block, index as usize, &qt_theme, &qt_options);
        string_to_c(html)
    } else {
        string_to_c(String::new())
    }
}

/// Save a block range in a page. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_save_block(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    page_path: *const c_char,
    index: i32,
    count: i32,
    raw_text: *const c_char,
    drop_comments: i32,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let path = unsafe { cstr_to_string(page_path) };
    let text = unsafe { cstr_to_string(raw_text) };
    let drop = drop_comments != 0;

    // Read current page content
    let content = match page::read_page(&dir, &path) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut blocks = parser::parse_blocks_with_options(&content, drop);
    let start = index.max(0) as usize;
    let cnt = count.max(1) as usize;

    if start >= blocks.len() {
        return -1;
    }

    let end = (start + cnt).min(blocks.len());
    // Replace the block range with the new text parsed as blocks
    let new_blocks = parser::parse_blocks_with_options(&text, drop);
    blocks.splice(start..end, new_blocks);

    let new_content = parser::blocks_to_adoc(&blocks);
    match page::save_and_index_page(conn, &dir, &path, &new_content) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Toggle a checkbox in a page. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_page_toggle_checkbox(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    page_path: *const c_char,
    block_index: i32,
    item_path: *const c_char,
) -> i32 {
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let path = unsafe { cstr_to_string(page_path) };
    let ipath = unsafe { cstr_to_string(item_path) };

    let content = match page::read_page(&dir, &path) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let idx = block_index.max(0) as usize;
    let new_content = match parser::toggle_checkbox(&content, idx, &ipath) {
        Some(c) => c,
        None => return -1,
    };

    match page::save_and_index_page(conn, &dir, &path, &new_content) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
