//! C FFI boundary for notesplusplus-core.
//!
//! Memory ownership rules:
//! - Every `*mut c_char` returned is Rust-allocated (CString::into_raw).
//!   Caller MUST free via `notes_core_free_string()`.
//! - Opaque handles (*mut T) are created by `_new`/`_open`, freed by `_free`.
//! - Input `*const c_char` parameters are borrowed; FFI copies internally.
//! - Return `c_int`: 0 = success, negative = error code.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::agent::{
    self, AgentSession, AgentStepResult, LlmClient, LlmConfig, PendingConfirmation,
    PermissionConfig, PermissionManager,
};
use crate::block::{Block, collect_footnotes};
use crate::constants;
use crate::db;
use crate::group;
use crate::html;
use crate::html::qt_html::{QtRenderOptions, QtThemeColors};
use crate::journal;
use crate::page;
use crate::parser;
use crate::paths::AppPaths;
use crate::search as search_mod;
use crate::server;
use crate::stt;
use crate::tree;

// ---------------------------------------------------------------------------
// Block rendering helpers
// ---------------------------------------------------------------------------

/// Render blocks to JSON array with pre-rendered HTML, matching the Rust bridge behavior.
fn blocks_to_json_with_html(
    blocks: &[Block],
    theme: &QtThemeColors,
    options: &QtRenderOptions,
) -> Vec<serde_json::Value> {
    let mut headings_vec = Vec::new();
    for (idx, block) in blocks.iter().enumerate() {
        if let Block::Heading { level, spans, .. } = block {
            if *level >= 1 && *level <= 5 {
                let text = spans.iter().map(|s| s.plain_text()).collect::<Vec<_>>().join("");
                let mut h_map = serde_json::Map::new();
                h_map.insert("level".into(), serde_json::Value::Number((*level).into()));
                h_map.insert("text".into(), serde_json::Value::String(text));
                h_map.insert("index".into(), serde_json::Value::Number(idx.into()));
                headings_vec.push(serde_json::Value::Object(h_map));
            }
        }
    }

    let mut list = Vec::new();
    for (idx, block) in blocks.iter().enumerate() {
        let mut json = block.to_qvariant_map();
        if let Block::Toc { .. } = block {
            if let serde_json::Value::Object(ref mut map) = json {
                map.insert("headings".into(), serde_json::Value::Array(headings_vec.clone()));
            }
        }
        match block {
            Block::Heading { .. } | Block::Paragraph { .. } |
            Block::OrderedListItem { .. } | Block::UnorderedListItem { .. } |
            Block::DescriptionListItem { .. } | Block::CalloutListItem { .. } |
            Block::CodeBlock { .. } | Block::LiteralBlock { .. } |
            Block::Blockquote { .. } | Block::Verse { .. } |
            Block::Admonition { .. } | Block::Sidebar { .. } |
            Block::Example { .. } | Block::Open { .. } |
            Block::Table { .. } | Block::Image { .. } => {
                let html = html::qt_html::render_qt_block(block, idx, theme, options);
                if let serde_json::Value::Object(ref mut map) = json {
                    map.insert("html".into(), serde_json::Value::String(html));
                }
            }
            _ => {}
        }
        list.push(json);
    }

    // Collect footnotes and append synthetic footnotes block
    let footnotes = collect_footnotes(blocks);
    if !footnotes.is_empty() {
        let mut fn_html = String::from(
            "<hr/><p style='margin:4px 8px;font-weight:bold;color:__LINK_COLOR__;'>Footnotes</p>",
        );
        for (i, (id, text)) in footnotes.iter().enumerate() {
            let num_label = (i + 1).to_string();
            let label = id.as_deref().unwrap_or(&num_label);
            let content = if text.is_empty() { label } else { text.as_str() };
            use std::fmt::Write;
            write!(
                fn_html,
                "<p style='margin:2px 8px;'>[{}] {}</p>",
                html::qt_html::escape_html_for_footnote(label),
                html::qt_html::escape_html_for_footnote(content)
            )
            .ok();
        }
        let mut fn_json = serde_json::Map::new();
        fn_json.insert("type".into(), serde_json::Value::String("footnotes".into()));
        fn_json.insert("html".into(), serde_json::Value::String(fn_html));
        list.push(serde_json::Value::Object(fn_json));
    }

    list
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse QtRenderOptions from JSON string (since QtRenderOptions doesn't impl Deserialize).
fn parse_qt_render_options(json_str: &str, notes_dir: Option<String>) -> QtRenderOptions {
    let val: serde_json::Value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let search_terms = val
        .get("search_terms")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let notes_dir = val
        .get("notes_dir")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or(notes_dir);
    let allow_external_images = val
        .get("allow_external_images")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    QtRenderOptions {
        search_terms,
        notes_dir,
        allow_external_images,
    }
}

/// Convert a C string pointer to a Rust &str. Returns None if null.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(ptr).to_str().ok() }
    }
}

/// Convert a C string pointer to a Rust String. Returns empty string if null.
unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    unsafe { cstr_to_str(ptr).unwrap_or("").to_owned() }
}

/// Convert a C string pointer to a Path. Returns empty path if null.
unsafe fn cstr_to_path(ptr: *const c_char) -> PathBuf {
    PathBuf::from(unsafe { cstr_to_string(ptr) })
}

/// Allocate a C string from a Rust String. Caller must free via notes_core_free_string.
fn string_to_c(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

// ---------------------------------------------------------------------------
// String ownership
// ---------------------------------------------------------------------------

/// Free a string allocated by any notes_core_* function.
///
/// # Safety
/// `s` must have been returned by a `notes_core_*` function, or be NULL.
#[no_mangle]
pub extern "C" fn notes_core_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)); }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn notes_core_const_db_filename() -> *mut c_char {
    string_to_c(constants::DB_FILENAME.to_string())
}

#[no_mangle]
pub extern "C" fn notes_core_const_journal_filename() -> *mut c_char {
    string_to_c(constants::JOURNAL_FILENAME.to_string())
}

#[no_mangle]
pub extern "C" fn notes_core_const_default_ai_endpoint() -> *mut c_char {
    string_to_c(constants::DEFAULT_AI_ENDPOINT.to_string())
}

#[no_mangle]
pub extern "C" fn notes_core_const_default_ai_model() -> *mut c_char {
    string_to_c(constants::DEFAULT_AI_MODEL.to_string())
}

#[no_mangle]
pub extern "C" fn notes_core_const_default_server_port() -> u16 {
    constants::DEFAULT_SERVER_PORT
}

// ---------------------------------------------------------------------------
// AppPaths
// ---------------------------------------------------------------------------

/// Create a new AppPaths with default locations.
#[no_mangle]
pub extern "C" fn notes_core_app_paths_new() -> *mut AppPaths {
    Box::into_raw(Box::new(AppPaths::new()))
}

#[no_mangle]
pub extern "C" fn notes_core_app_paths_free(p: *mut AppPaths) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)); }
    }
}

#[no_mangle]
pub extern "C" fn notes_core_app_paths_data_dir(p: *const AppPaths) -> *mut c_char {
    let p = unsafe { &*p };
    string_to_c(p.data_dir.to_string_lossy().into_owned())
}

#[no_mangle]
pub extern "C" fn notes_core_app_paths_notes_dir(p: *const AppPaths) -> *mut c_char {
    let p = unsafe { &*p };
    string_to_c(p.notes_dir.to_string_lossy().into_owned())
}

#[no_mangle]
pub extern "C" fn notes_core_app_paths_db_path(p: *const AppPaths) -> *mut c_char {
    let p = unsafe { &*p };
    string_to_c(p.db_path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

/// Open a database connection. Returns NULL on failure.
#[no_mangle]
pub extern "C" fn notes_core_db_open(db_path: *const c_char) -> *mut rusqlite::Connection {
    let path = unsafe { cstr_to_path(db_path) };
    match db::open_db(&path) {
        Ok(conn) => {
            let _ = db::init_schema(&conn);
            Box::into_raw(Box::new(conn))
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn notes_core_db_close(c: *mut rusqlite::Connection) {
    if !c.is_null() {
        unsafe { drop(Box::from_raw(c)); }
    }
}

// ---------------------------------------------------------------------------
// Page CRUD
// ---------------------------------------------------------------------------

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
        Err(e) => string_to_c(format!("ERROR: {}", e)),
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
    let conn = unsafe { &mut *conn };
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
) -> i32 {
    let conn = unsafe { &mut *conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let name = unsafe { cstr_to_string(name) };
    match page::create_page(conn, &dir, &name, false) {
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
    let conn = unsafe { &mut *conn };
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
    let conn = unsafe { &mut *conn };
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
    let conn = unsafe { &mut *conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let source = unsafe { cstr_to_string(source_name) };
    let target = unsafe { cstr_to_string(target_group) };
    match page::move_page(conn, &dir, &source, &target) {
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
    let conn = unsafe { &mut *conn };
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
    let conn = unsafe { &*conn };
    let limit = if limit > 0 { limit as usize } else { 20 };
    match page::recent_pages(conn, limit) {
        Ok(pages) => {
            let values: Vec<serde_json::Value> = pages.iter().map(|p| p.to_json_value()).collect();
            string_to_c(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()))
        }
        Err(_) => string_to_c("[]".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Block parsing & rendering
// ---------------------------------------------------------------------------

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
    let qt_options = parse_qt_render_options(&opts_str, Some(dir.clone()));

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
    let conn = unsafe { &mut *conn };
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
    let conn = unsafe { &mut *conn };
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

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

/// Get flat group list as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_groups_flat_json(
    conn: *mut rusqlite::Connection,
) -> *mut c_char {
    let conn = unsafe { &*conn };
    match group::get_groups_flat(conn) {
        Ok(groups) => {
            let json = serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string());
            string_to_c(json)
        }
        Err(_) => string_to_c("[]".to_string()),
    }
}

/// Create a group. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_group_create(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    parent: *const c_char,
    name: *const c_char,
) -> i32 {
    let conn = unsafe { &mut *conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let parent = unsafe { cstr_to_string(parent) };
    let name = unsafe { cstr_to_string(name) };
    match group::create_group(conn, &dir, &parent, &name) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Rename a group. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_group_rename(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    old_path: *const c_char,
    new_name: *const c_char,
) -> i32 {
    let conn = unsafe { &mut *conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let old = unsafe { cstr_to_string(old_path) };
    let new = unsafe { cstr_to_string(new_name) };
    match group::rename_group(conn, &dir, &old, &new) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Delete a group. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_group_delete(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    path: *const c_char,
    recursive: i32,
) -> i32 {
    let conn = unsafe { &mut *conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let path = unsafe { cstr_to_string(path) };
    match group::delete_group(conn, &dir, &path, recursive != 0) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Toggle group collapsed state. Returns new state (0=expanded, 1=collapsed), -1 on error.
#[no_mangle]
pub extern "C" fn notes_core_group_toggle_collapsed(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
) -> i32 {
    let conn = unsafe { &mut *conn };
    let path = unsafe { cstr_to_string(path) };
    match group::toggle_group_collapsed(conn, &path) {
        Ok(collapsed) => if collapsed { 1 } else { 0 },
        Err(_) => -1,
    }
}

/// Set group note sort order. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_group_set_note_sort(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
    sort_order: i32,
) -> i32 {
    let conn = unsafe { &mut *conn };
    let path = unsafe { cstr_to_string(path) };
    let sort = match sort_order {
        0 => group::NoteSortOrder::NewestFirst,
        _ => group::NoteSortOrder::ByName,
    };
    match group::set_group_note_sort(conn, &path, sort) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get group note sort order. Returns sort order (0=newest, 1=byname), -1 on error.
#[no_mangle]
pub extern "C" fn notes_core_group_get_note_sort(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
) -> i32 {
    let conn = unsafe { &*conn };
    let path = unsafe { cstr_to_string(path) };
    match group::get_group_note_sort(conn, &path) {
        Ok(group::NoteSortOrder::NewestFirst) => 0,
        Ok(group::NoteSortOrder::ByName) => 1,
        Err(_) => -1,
    }
}

/// Build group tree JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_build_group_tree_json(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    depth: i32,
    drop_comments: i32,
    theme_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let conn = unsafe { &*conn };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let theme_str = unsafe { cstr_to_string(theme_json) };
    let opts_str = unsafe { cstr_to_string(options_json) };

    let pages = page::list_pages(conn).unwrap_or_default();
    let groups = group::get_groups_flat(conn).unwrap_or_default();

    let theme: HashMap<String, String> =
        serde_json::from_str(&theme_str).unwrap_or_default();
    let qt_theme = if theme.is_empty() {
        None
    } else {
        Some(QtThemeColors::from_map(&theme))
    };
    let qt_opts = parse_qt_render_options(&opts_str, None);
    let qt_opts_ref = if opts_str.is_empty() { None } else { Some(&qt_opts) };

    string_to_c(tree::build_group_tree(
        &pages,
        &groups,
        depth,
        Some(&dir),
        drop_comments != 0,
        qt_theme.as_ref(),
        qt_opts_ref,
    ))
}

/// Load main page data JSON (recent pages + tree). Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_load_main_page_data_json(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    depth: i32,
    drop_comments: i32,
) -> *mut c_char {
    let conn = unsafe { &*conn };
    let dir = unsafe { cstr_to_path(notes_dir) };

    let recent = page::recent_pages(conn, 20).unwrap_or_default();
    let recent_jsons: Vec<String> = recent
        .iter()
        .map(|p| {
            let preview = page::get_page_preview_json_with_options(
                &dir, &p.filename, 4, drop_comments != 0,
            );
            let mut map = p.to_json_value();
            if let serde_json::Value::Object(ref mut obj) = map {
                obj.insert(
                    "preview_blocks_json".into(),
                    serde_json::Value::String(preview),
                );
            }
            serde_json::to_string(&map).unwrap_or_default()
        })
        .collect();

    let pages = page::list_pages(conn).unwrap_or_default();
    let groups = group::get_groups_flat(conn).unwrap_or_default();
    let tree_json = crate::tree::build_group_tree(
        &pages, &groups, depth, Some(&dir), drop_comments != 0, None, None,
    );

    let result = serde_json::json!({
        "recent_pages": recent_jsons,
        "grouped_tree": tree_json,
    });
    string_to_c(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
}

// ---------------------------------------------------------------------------
// Search (background + poll pattern)
// ---------------------------------------------------------------------------

struct FfiSearchEngine {
    result: Arc<Mutex<Option<Result<Vec<search_mod::SearchResult>, String>>>>,
    _handle: Option<thread::JoinHandle<()>>,
}

/// Create a new search engine.
#[no_mangle]
pub extern "C" fn notes_core_search_new() -> *mut FfiSearchEngine {
    Box::into_raw(Box::new(FfiSearchEngine {
        result: Arc::new(Mutex::new(None)),
        _handle: None,
    }))
}

/// Start a background search. Call poll to get results.
#[no_mangle]
pub extern "C" fn notes_core_search_start(
    engine: *mut FfiSearchEngine,
    db_path: *const c_char,
    query: *const c_char,
) -> i32 {
    let engine = unsafe { &mut *engine };
    let db_path = unsafe { cstr_to_path(db_path) };
    let query = unsafe { cstr_to_string(query) };

    if query.trim().is_empty() {
        return -1;
    }

    // Clear previous result
    if let Ok(mut guard) = engine.result.lock() {
        *guard = None;
    }

    let result_slot = engine.result.clone();
    engine._handle = Some(thread::spawn(move || {
        let res = (|| -> Result<Vec<search_mod::SearchResult>, String> {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .map_err(|e| format!("DB open: {}", e))?;
            search_mod::search_pages(&conn, &query).map_err(|e| format!("Search: {}", e))
        })();
        if let Ok(mut guard) = result_slot.lock() {
            *guard = Some(res);
        }
    }));
    0
}

/// Poll for search results. Returns: 0 = still running, 1 = ready, -1 = error.
/// If ready, *out_results_json is set to allocated JSON string.
#[no_mangle]
pub extern "C" fn notes_core_search_poll(
    engine: *mut FfiSearchEngine,
    out_results_json: *mut *mut c_char,
) -> i32 {
    let engine = unsafe { &*engine };
    let has_result = engine
        .result
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    if !has_result {
        return 0; // still running
    }

    let result = engine.result.lock().ok().and_then(|mut g| g.take());

    match result {
        Some(Ok(results)) => {
            let values: Vec<serde_json::Value> = results.iter().map(|r| {
                serde_json::json!({
                    "title": r.page.title,
                    "filename": r.page.filename,
                    "group_path": r.page.group_path,
                    "full_path": r.page.full_path(),
                    "snippet": r.snippet,
                    "created_at": r.page.created_at,
                    "updated_at": r.page.updated_at,
                    "block_count": r.page.block_count,
                })
            }).collect();
            let json = serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string());
            unsafe { *out_results_json = string_to_c(json) };
            1
        }
        Some(Err(_)) => -1,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn notes_core_search_free(engine: *mut FfiSearchEngine) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine)); }
    }
}

// ---------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------

/// Get today's journal page name. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_journal_today(
    _notes_dir: *const c_char,
) -> *mut c_char {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let title = format!("Journal {}", today);
    string_to_c(title)
}

/// Append a line to today's journal. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_journal_append(
    _conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    text: *const c_char,
    _is_task: i32,
) -> i32 {
    let dir = unsafe { cstr_to_path(notes_dir) };
    let text = unsafe { cstr_to_string(text) };
    match journal::append_to_journal_today(&dir, &text) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get recent journal lines. Returns allocated JSON string.
#[no_mangle]
pub extern "C" fn notes_core_journal_recent_lines(
    notes_dir: *const c_char,
    limit: i32,
) -> *mut c_char {
    let dir = unsafe { cstr_to_path(notes_dir) };
    let limit = if limit > 0 { limit as usize } else { 20 };
    match journal::recent_journal_lines(&dir, limit) {
        Ok(lines) => string_to_c(serde_json::to_string(&lines).unwrap_or_else(|_| "[]".to_string())),
        Err(_) => string_to_c("[]".to_string()),
    }
}

// ---------------------------------------------------------------------------
// HTML export
// ---------------------------------------------------------------------------

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
        Err(e) => string_to_c(format!("ERROR: {}", e)),
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
        Err(e) => string_to_c(format!("ERROR: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// TLS
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Agent session
// ---------------------------------------------------------------------------

struct FfiAgentSession {
    session: Arc<Mutex<AgentSession>>,
    result: Arc<Mutex<Option<WorkerResult>>>,
    streaming_buffer: Arc<Mutex<String>>,
}

struct WorkerResult {
    step_result: Result<AgentStepResult, String>,
    messages_json: String,
    pending_action: Option<PendingConfirmation>,
    can_undo: bool,
    last_snapshot_id: Option<String>,
    last_created_note: Option<String>,
}

/// Create a new agent session. config_json contains LlmConfig fields.
#[no_mangle]
pub extern "C" fn notes_core_agent_new(
    notes_dir: *const c_char,
    db_path: *const c_char,
    backup_dir: *const c_char,
    config_json: *const c_char,
) -> *mut FfiAgentSession {
    let ndir = unsafe { cstr_to_path(notes_dir) };
    let db = unsafe { cstr_to_path(db_path) };
    let backup = unsafe { cstr_to_path(backup_dir) };
    let cfg_str = unsafe { cstr_to_string(config_json) };
    let cfg: serde_json::Value = serde_json::from_str(&cfg_str).unwrap_or_default();
    let llm_config: LlmConfig = serde_json::from_value(cfg.clone()).unwrap_or_default();
    let perm_config: PermissionConfig = serde_json::from_value(cfg).unwrap_or_default();

    let client = LlmClient::new(llm_config);
    let perm_mgr = PermissionManager::new(perm_config);
    let conn = crate::db::open_db(&db).unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap());
    let repository: std::sync::Arc<dyn crate::repository::NoteRepository> = std::sync::Arc::new(
        crate::repository::FsSqliteNoteRepository::new(&ndir, std::sync::Arc::new(std::sync::Mutex::new(conn)))
    );
    let session = AgentSession::new(ndir, repository, backup, perm_mgr, client);

    Box::into_raw(Box::new(FfiAgentSession {
        session: Arc::new(Mutex::new(session)),
        result: Arc::new(Mutex::new(None)),
        streaming_buffer: Arc::new(Mutex::new(String::new())),
    }))
}

/// Send a prompt to the agent (background). Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_agent_send(
    ffi: *mut FfiAgentSession,
    prompt: *const c_char,
) -> i32 {
    let ffi = unsafe { &mut *ffi };
    let prompt = unsafe { cstr_to_string(prompt) };

    let session = ffi.session.clone();
    let result_slot = ffi.result.clone();
    let buffer = ffi.streaming_buffer.clone();

    // Clear streaming buffer
    if let Ok(mut b) = buffer.lock() {
        b.clear();
    }

    thread::spawn(move || {
        let step_result;
        let messages_json;
        let pending_action;
        let can_undo;
        let last_snapshot_id;
        let last_created_note;

        {
            let mut sess = session.lock().unwrap();
            let result = sess.send_prompt_streaming(&prompt, |tok| {
                if let Ok(mut b) = buffer.lock() {
                    b.push_str(tok);
                }
            });
            step_result = result;
            messages_json = serde_json::to_string(&sess.messages())
                .unwrap_or_else(|_| "[]".to_string());
            pending_action = sess.pending_action();
            can_undo = sess.can_undo();
            last_snapshot_id = sess.last_snapshot_id();
            last_created_note = sess.last_created_note();
        }

        if let Ok(mut guard) = result_slot.lock() {
            *guard = Some(WorkerResult {
                step_result: Ok(step_result),
                messages_json,
                pending_action,
                can_undo,
                last_snapshot_id,
                last_created_note,
            });
        }
    });
    0
}

/// Poll agent streaming buffer. Returns accumulated tokens as allocated string.
#[no_mangle]
pub extern "C" fn notes_core_agent_poll_streaming(
    ffi: *mut FfiAgentSession,
) -> *mut c_char {
    let ffi = unsafe { &*ffi };
    let buf = ffi.streaming_buffer.lock().unwrap();
    string_to_c(buf.clone())
}

/// Poll for agent result. Returns: 0 = still running, 1 = ready, -1 = error.
/// If ready, *out_json is set to allocated JSON with result details.
#[no_mangle]
pub extern "C" fn notes_core_agent_poll(
    ffi: *mut FfiAgentSession,
    out_json: *mut *mut c_char,
) -> i32 {
    let ffi = unsafe { &*ffi };
    let has = ffi.result.lock().map(|g| g.is_some()).unwrap_or(false);
    if !has {
        return 0;
    }

    let result = ffi.result.lock().ok().and_then(|mut g| g.take());
    match result {
        Some(wr) => {
            let json = serde_json::json!({
                "messages_json": wr.messages_json,
                "pending_action_json": wr.pending_action.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default()).unwrap_or_default(),
                "has_pending_action": wr.pending_action.is_some(),
                "can_undo": wr.can_undo,
                "last_snapshot_id": wr.last_snapshot_id,
                "last_created_note": wr.last_created_note,
                "error": match &wr.step_result {
                    Err(e) => e.clone(),
                    Ok(AgentStepResult::Error(e)) => e.clone(),
                    _ => String::new(),
                },
                "finished_content": match &wr.step_result {
                    Ok(AgentStepResult::Finished { content, .. }) => content.clone(),
                    _ => String::new(),
                },
                "requires_confirmation": matches!(&wr.step_result, Ok(AgentStepResult::RequiresConfirmation(_))),
            });
            unsafe { *out_json = string_to_c(serde_json::to_string(&json).unwrap_or_default()) };
            1
        }
        None => 0,
    }
}

/// Confirm or deny a pending action (background).
#[no_mangle]
pub extern "C" fn notes_core_agent_confirm(
    ffi: *mut FfiAgentSession,
    approved: i32,
) -> i32 {
    let ffi = unsafe { &mut *ffi };
    let session = ffi.session.clone();
    let result_slot = ffi.result.clone();
    let buffer = ffi.streaming_buffer.clone();

    if let Ok(mut b) = buffer.lock() {
        b.clear();
    }

    thread::spawn(move || {
        let step_result;
        let messages_json;
        let pending_action;
        let can_undo;
        let last_snapshot_id;
        let last_created_note;

        {
            let mut sess = session.lock().unwrap();
            let result = sess.confirm_pending_action_streaming(approved != 0, |tok| {
                if let Ok(mut b) = buffer.lock() {
                    b.push_str(tok);
                }
            });
            step_result = result;
            messages_json = serde_json::to_string(&sess.messages())
                .unwrap_or_else(|_| "[]".to_string());
            pending_action = sess.pending_action();
            can_undo = sess.can_undo();
            last_snapshot_id = sess.last_snapshot_id();
            last_created_note = sess.last_created_note();
        }

        if let Ok(mut guard) = result_slot.lock() {
            *guard = Some(WorkerResult {
                step_result: Ok(step_result),
                messages_json,
                pending_action,
                can_undo,
                last_snapshot_id,
                last_created_note,
            });
        }
    });
    0
}

/// Undo the last agent action. Returns allocated result string.
#[no_mangle]
pub extern "C" fn notes_core_agent_undo(
    ffi: *mut FfiAgentSession,
) -> *mut c_char {
    let ffi = unsafe { &mut *ffi };
    let mut sess = ffi.session.lock().unwrap();
    match sess.undo_last_action() {
        Ok(msg) => string_to_c(msg),
        Err(e) => string_to_c(format!("ERROR: {}", e)),
    }
}

/// Update agent configuration.
#[no_mangle]
pub extern "C" fn notes_core_agent_configure(
    ffi: *mut FfiAgentSession,
    config_json: *const c_char,
) {
    let ffi = unsafe { &mut *ffi };
    let cfg_str = unsafe { cstr_to_string(config_json) };
    let cfg: serde_json::Value = serde_json::from_str(&cfg_str).unwrap_or_default();
    let llm_config: LlmConfig = serde_json::from_value(cfg.clone()).unwrap_or_default();
    let perm_config: PermissionConfig = serde_json::from_value(cfg).unwrap_or_default();

    let client = LlmClient::new(llm_config);
    let perm_mgr = PermissionManager::new(perm_config);
    let mut sess = ffi.session.lock().unwrap();
    sess.update_config(perm_mgr, client);
}

/// Reset agent session context.
#[no_mangle]
pub extern "C" fn notes_core_agent_reset_session(
    ffi: *mut FfiAgentSession,
    context_filename: *const c_char,
    context_content: *const c_char,
    extra_context: *const c_char,
) {
    let ffi = unsafe { &mut *ffi };
    let fname = unsafe { cstr_to_string(context_filename) };
    let fcontent = unsafe { cstr_to_string(context_content) };
    let extra = unsafe { cstr_to_string(extra_context) };

    let mut sess = ffi.session.lock().unwrap();
    let active = if fname.is_empty() {
        None
    } else {
        Some((fname.as_str(), fcontent.as_str()))
    };
    let extra_opt = if extra.is_empty() {
        None
    } else {
        Some(extra.as_str())
    };
    sess.reset_session(active, extra_opt);
}

#[no_mangle]
pub extern "C" fn notes_core_agent_free(ffi: *mut FfiAgentSession) {
    if !ffi.is_null() {
        unsafe { drop(Box::from_raw(ffi)); }
    }
}

// ---------------------------------------------------------------------------
// STT (Speech-to-Text)
// ---------------------------------------------------------------------------

/// Transcribe a WAV file. Returns allocated text or error string.
#[no_mangle]
pub extern "C" fn notes_core_stt_transcribe(
    model_path: *const c_char,
    wav_path: *const c_char,
) -> *mut c_char {
    let mp = unsafe { cstr_to_path(model_path) };
    let wp = unsafe { cstr_to_path(wav_path) };

    match stt::WhisperEngine::load(&mp) {
        Ok(engine) => match engine.transcribe_wav_file(&wp) {
            Ok(text) => string_to_c(text),
            Err(e) => string_to_c(format!("ERROR: {}", e)),
        },
        Err(e) => string_to_c(format!("ERROR: {}", e)),
    }
}

/// Get STT model catalog as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_stt_model_catalog_json(
    models_dir: *const c_char,
    active_model_id: *const c_char,
) -> *mut c_char {
    let dir = unsafe { cstr_to_path(models_dir) };
    let active = unsafe { cstr_to_string(active_model_id) };
    let active_opt = if active.is_empty() {
        None
    } else {
        Some(active.as_str())
    };
    let catalog = stt::get_model_catalog(&dir, active_opt);
    string_to_c(serde_json::to_string(&catalog).unwrap_or_else(|_| "[]".to_string()))
}

/// Delete an STT model. Returns 1 if deleted, 0 if not found.
#[no_mangle]
pub extern "C" fn notes_core_stt_model_delete(
    models_dir: *const c_char,
    model_id: *const c_char,
) -> i32 {
    let dir = unsafe { cstr_to_path(models_dir) };
    let id = unsafe { cstr_to_string(model_id) };
    let downloader = stt::ModelDownloader::new(&dir);
    match downloader.delete(&id) {
        Ok(deleted) => if deleted { 1 } else { 0 },
        Err(_) => 0,
    }
}

struct FfiSttDownload {
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<f64>>,
    result: Arc<Mutex<Option<Result<String, String>>>>,
    _handle: Option<thread::JoinHandle<()>>,
}

/// Start downloading an STT model (background). Returns handle.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_start(
    models_dir: *const c_char,
    model_id: *const c_char,
) -> *mut FfiSttDownload {
    let dir = unsafe { cstr_to_path(models_dir) };
    let id = unsafe { cstr_to_string(model_id) };

    let model = match stt::find_model_by_id(&id) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(Mutex::new(0.0_f64));
    let result = Arc::new(Mutex::new(None));

    let cancel_c = cancel.clone();
    let progress_c = progress.clone();
    let result_c = result.clone();

    let handle = thread::spawn(move || {
        let downloader = stt::ModelDownloader::new(&dir);
        let res = downloader.download(&model, Some(cancel_c), |prog| {
            if let Ok(mut p) = progress_c.lock() {
                *p = prog.percent as f64;
            }
        });
        if let Ok(mut guard) = result_c.lock() {
            *guard = Some(res.map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string()));
        }
    });

    Box::into_raw(Box::new(FfiSttDownload {
        cancel,
        progress,
        result,
        _handle: Some(handle),
    }))
}

/// Poll download progress. Returns: 0 = running, 1 = done, -1 = error.
/// *out_progress is set to 0.0-1.0, *out_result to allocated string on completion.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_poll(
    handle: *mut FfiSttDownload,
    out_progress: *mut f64,
    out_result: *mut *mut c_char,
) -> i32 {
    let h = unsafe { &*handle };

    let prog = h.progress.lock().map(|p| *p).unwrap_or(0.0);
    unsafe { *out_progress = prog };

    let has = h.result.lock().map(|g| g.is_some()).unwrap_or(false);
    if !has {
        return 0;
    }

    let res = h.result.lock().ok().and_then(|mut g| g.take());
    match res {
        Some(Ok(path)) => {
            unsafe { *out_result = string_to_c(path) };
            1
        }
        Some(Err(e)) => {
            unsafe { *out_result = string_to_c(e) };
            -1
        }
        None => 0,
    }
}

/// Cancel a download.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_cancel(handle: *mut FfiSttDownload) {
    if !handle.is_null() {
        let h = unsafe { &*handle };
        h.cancel.store(true, Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "C" fn notes_core_stt_download_free(handle: *mut FfiSttDownload) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}

// ---------------------------------------------------------------------------
// Network / utilities
// ---------------------------------------------------------------------------

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
