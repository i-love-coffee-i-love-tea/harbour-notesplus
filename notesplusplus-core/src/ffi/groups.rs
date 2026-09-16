use std::collections::HashMap;
use std::os::raw::c_char;

use crate::group;
use crate::html::qt_html::QtThemeColors;
use crate::page;
use crate::tree;
use super::common::{cstr_to_path, cstr_to_string, ffi_err, parse_qt_render_options, string_to_c};

/// Get flat group list as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_groups_flat_json(
    conn: *mut rusqlite::Connection,
) -> *mut c_char {
    let conn = match unsafe { conn.as_ref() } {
        Some(c) => c,
        None => return ffi_err!("null connection"),
    };
    match group::get_groups_flat(conn) {
        Ok(groups) => {
            let json = serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string());
            string_to_c(json)
        }
        Err(e) => {
            eprintln!("notes_core_groups_flat_json: {}", e);
            string_to_c("[]".to_string())
        }
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
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_mut() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_ref() } {
        Some(c) => c,
        None => return -1,
    };
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
    let conn = match unsafe { conn.as_ref() } {
        Some(c) => c,
        None => return ffi_err!("null connection"),
    };
    let dir = unsafe { cstr_to_path(notes_dir) };
    let theme_str = unsafe { cstr_to_string(theme_json) };
    let opts_str = unsafe { cstr_to_string(options_json) };

    let pages = match page::list_pages(conn) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("notes_core_build_group_tree_json list_pages: {}", e);
            Vec::new()
        }
    };
    let groups = match group::get_groups_flat(conn) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("notes_core_build_group_tree_json get_groups_flat: {}", e);
            Vec::new()
        }
    };

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
    let conn = match unsafe { conn.as_ref() } {
        Some(c) => c,
        None => return ffi_err!("null connection"),
    };
    let dir = unsafe { cstr_to_path(notes_dir) };

    let recent = match page::recent_pages(conn, crate::constants::DEFAULT_RECENT_PAGES_LIMIT) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("notes_core_load_main_page_data_json recent_pages: {}", e);
            Vec::new()
        }
    };
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
