use std::os::raw::c_char;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::search as search_mod;
use super::common::{cstr_to_path, cstr_to_string, string_to_c};

pub struct FfiSearchEngine {
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
