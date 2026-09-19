use std::os::raw::c_char;

use crate::journal;
use super::common::{cstr_to_path, cstr_to_string, string_to_c};

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
