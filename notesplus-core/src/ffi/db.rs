use std::os::raw::c_char;
use crate::db;
use super::common::{cstr_to_path, DbConn};

/// Open a database connection wrapped in a thread-safe container. Returns NULL on failure.
#[no_mangle]
pub extern "C" fn notes_core_db_open(db_path: *const c_char) -> *mut rusqlite::Connection {
    let path = unsafe { cstr_to_path(db_path) };
    match db::open_db(&path) {
        Ok(conn) => Box::into_raw(Box::new(DbConn::new(conn))) as *mut rusqlite::Connection,
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn notes_core_db_close(c: *mut rusqlite::Connection) {
    if !c.is_null() {
        unsafe { drop(Box::from_raw(c as *mut DbConn)); }
    }
}
