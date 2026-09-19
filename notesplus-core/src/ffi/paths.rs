use std::os::raw::c_char;
use crate::paths::AppPaths;
use super::common::string_to_c;

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
