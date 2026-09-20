use std::collections::HashMap;
use std::os::raw::c_char;

use crate::group;
use crate::html::qt_html::{QtRenderOptions, QtThemeColors};
use crate::page;
use crate::tree;
use super::common::{
    cstr_to_path, cstr_to_string, ffi_err, parse_qt_render_options, string_to_c, with_conn,
    with_conn_mut,
};

/// Get flat group list as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_groups_flat_json(
    conn: *mut rusqlite::Connection,
) -> *mut c_char {
    unsafe {
        with_conn(conn, ffi_err!("null connection"), |conn| {
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
        })
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
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let dir = cstr_to_path(notes_dir);
            let parent = cstr_to_string(parent);
            let name = cstr_to_string(name);
            match group::create_group(conn, &dir, &parent, &name) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        })
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
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let dir = cstr_to_path(notes_dir);
            let old = cstr_to_string(old_path);
            let new = cstr_to_string(new_name);
            match group::rename_group(conn, &dir, &old, &new) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        })
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
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let dir = cstr_to_path(notes_dir);
            let path = cstr_to_string(path);
            match group::delete_group(conn, &dir, &path, recursive != 0) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        })
    }
}

/// Toggle group collapsed state. Returns new state (0=expanded, 1=collapsed), -1 on error.
#[no_mangle]
pub extern "C" fn notes_core_group_toggle_collapsed(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
) -> i32 {
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let path = cstr_to_string(path);
            match group::toggle_group_collapsed(conn, &path) {
                Ok(collapsed) => if collapsed { 1 } else { 0 },
                Err(_) => -1,
            }
        })
    }
}

/// Set group note sort order. Returns 0 on success.
#[no_mangle]
pub extern "C" fn notes_core_group_set_note_sort(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
    sort_order: i32,
) -> i32 {
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let path = cstr_to_string(path);
            let sort = match sort_order {
                0 => group::NoteSortOrder::NewestFirst,
                _ => group::NoteSortOrder::ByName,
            };
            match group::set_group_note_sort(conn, &path, sort) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        })
    }
}

/// Get group note sort order. Returns sort order (0=newest, 1=byname), -1 on error.
#[no_mangle]
pub extern "C" fn notes_core_group_get_note_sort(
    conn: *mut rusqlite::Connection,
    path: *const c_char,
) -> i32 {
    unsafe {
        with_conn_mut(conn, -1, |conn| {
            let path = cstr_to_string(path);
            match group::get_group_note_sort(conn, &path) {
                Ok(group::NoteSortOrder::NewestFirst) => 0,
                Ok(group::NoteSortOrder::ByName) => 1,
                Err(_) => -1,
            }
        })
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
    unsafe {
        with_conn(conn, ffi_err!("null connection"), |conn| {
            let dir = cstr_to_path(notes_dir);
            let theme_str = cstr_to_string(theme_json);
            let opts_str = cstr_to_string(options_json);

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
                Some(QtThemeColors::default())
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
        })
    }
}

/// Load main page data JSON (recent pages + tree). Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_load_main_page_data_json(
    conn: *mut rusqlite::Connection,
    notes_dir: *const c_char,
    depth: i32,
    drop_comments: i32,
) -> *mut c_char {
    unsafe {
        with_conn(conn, ffi_err!("null connection"), |conn| {
            let dir = cstr_to_path(notes_dir);

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
            let default_theme = QtThemeColors::default();
            let default_opts = QtRenderOptions::default();
            let tree_json = crate::tree::build_group_tree(
                &pages, &groups, depth, Some(&dir), drop_comments != 0, Some(&default_theme), Some(&default_opts),
            );

            let result = serde_json::json!({
                "recent_pages": recent_jsons,
                "grouped_tree": tree_json,
            });
            string_to_c(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::ffi::db::{notes_core_db_close, notes_core_db_open};
    use std::ffi::CString;

    #[test]
    fn test_ffi_group_note_sort() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let notes_dir = dir.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();

        let c_db_path = CString::new(db_path.to_str().unwrap()).unwrap();
        let conn_ptr = notes_core_db_open(c_db_path.as_ptr());
        assert!(!conn_ptr.is_null());

        let c_notes_dir = CString::new(notes_dir.to_str().unwrap()).unwrap();
        let c_parent = CString::new("").unwrap();
        let c_name = CString::new("MyGroup").unwrap();
        let rc = notes_core_group_create(conn_ptr, c_notes_dir.as_ptr(), c_parent.as_ptr(), c_name.as_ptr());
        assert_eq!(rc, 0);

        let c_group_path = CString::new("MyGroup").unwrap();
        // Default sort should be 0 (NewestFirst)
        let initial_sort = notes_core_group_get_note_sort(conn_ptr, c_group_path.as_ptr());
        assert_eq!(initial_sort, 0);

        // Set to 1 (ByName)
        let rc = notes_core_group_set_note_sort(conn_ptr, c_group_path.as_ptr(), 1);
        assert_eq!(rc, 0);
        let updated_sort = notes_core_group_get_note_sort(conn_ptr, c_group_path.as_ptr());
        assert_eq!(updated_sort, 1);

        // Set back to 0 (NewestFirst)
        let rc = notes_core_group_set_note_sort(conn_ptr, c_group_path.as_ptr(), 0);
        assert_eq!(rc, 0);
        let reset_sort = notes_core_group_get_note_sort(conn_ptr, c_group_path.as_ptr());
        assert_eq!(reset_sort, 0);

        notes_core_db_close(conn_ptr);
    }
}
