use std::ffi::{CStr, CString};
use std::fmt::Write;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::block::{collect_footnotes, Block};
use crate::html;
use crate::html::qt_html::{QtRenderOptions, QtThemeColors};

/// Thread-safe wrapper around a SQLite database connection for FFI consumers.
pub struct DbConn {
    pub inner: Mutex<rusqlite::Connection>,
}

impl DbConn {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self {
            inner: Mutex::new(conn),
        }
    }
}

/// Safely execute an FFI closure with shared reference to the connection, guarding against concurrent access and panics.
pub unsafe fn with_conn<F, R>(conn_ptr: *mut rusqlite::Connection, default: R, f: F) -> R
where
    F: FnOnce(&rusqlite::Connection) -> R + std::panic::UnwindSafe,
    R: std::panic::UnwindSafe,
{
    if conn_ptr.is_null() {
        return default;
    }
    let db_conn = &*(conn_ptr as *const DbConn);
    let guard = match db_conn.inner.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&guard))).unwrap_or(default)
}

/// Safely execute an FFI closure with mutable reference to the connection, guarding against concurrent access and panics.
pub unsafe fn with_conn_mut<F, R>(conn_ptr: *mut rusqlite::Connection, default: R, f: F) -> R
where
    F: FnOnce(&mut rusqlite::Connection) -> R + std::panic::UnwindSafe,
    R: std::panic::UnwindSafe,
{
    if conn_ptr.is_null() {
        return default;
    }
    let db_conn = &*(conn_ptr as *const DbConn);
    let mut guard = match db_conn.inner.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut guard))).unwrap_or(default)
}

/// Render blocks to JSON array with pre-rendered HTML, matching the Rust bridge behavior.
pub fn blocks_to_json_with_html(
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
            Block::Heading { .. }
            | Block::Paragraph { .. }
            | Block::OrderedListItem { .. }
            | Block::UnorderedListItem { .. }
            | Block::DescriptionListItem { .. }
            | Block::CalloutListItem { .. }
            | Block::CodeBlock { .. }
            | Block::LiteralBlock { .. }
            | Block::Blockquote { .. }
            | Block::Verse { .. }
            | Block::Admonition { .. }
            | Block::Sidebar { .. }
            | Block::Example { .. }
            | Block::Open { .. }
            | Block::Table { .. }
            | Block::Image { .. } => {
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

/// Parse QtRenderOptions from JSON string (since QtRenderOptions doesn't impl Deserialize).
pub fn parse_qt_render_options(json_str: &str, notes_dir: Option<String>) -> QtRenderOptions {
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
///
/// # Safety
/// If `ptr` is not null, it must point to a valid null-terminated C string.
pub unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        None
    } else {
        CStr::from_ptr(ptr).to_str().ok()
    }
}

/// Convert a C string pointer to a Rust String. Returns empty string if null.
///
/// # Safety
/// If `ptr` is not null, it must point to a valid null-terminated C string.
pub unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    cstr_to_str(ptr).unwrap_or("").to_owned()
}

/// Convert a C string pointer to a Path. Returns empty path if null.
///
/// # Safety
/// If `ptr` is not null, it must point to a valid null-terminated C string.
pub unsafe fn cstr_to_path(ptr: *const c_char) -> PathBuf {
    PathBuf::from(cstr_to_string(ptr))
}

/// Allocate a C string from a Rust String. Caller must free via notes_core_free_string.
/// Interior NUL bytes are stripped to prevent silent truncation.
pub fn string_to_c(s: String) -> *mut c_char {
    let sanitized = s.replace('\0', "");
    CString::new(sanitized).unwrap_or_default().into_raw()
}

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

macro_rules! ffi_err {
    ($e:expr) => { crate::ffi::common::string_to_c(format!("ERROR: {}", $e)) };
}
pub(crate) use ffi_err;
