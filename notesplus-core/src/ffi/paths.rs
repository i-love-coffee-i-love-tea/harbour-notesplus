use std::os::raw::c_char;
use crate::paths::AppPaths;
use super::common::{cstr_to_path, string_to_c};

/// Create a new AppPaths with default locations.
#[no_mangle]
pub extern "C" fn notes_core_app_paths_new() -> *mut AppPaths {
    Box::into_raw(Box::new(AppPaths::new()))
}

/// Create a new AppPaths with explicitly provided data_dir and notes_dir.
/// If either argument is null or empty, falls back to the respective default directory.
#[no_mangle]
pub extern "C" fn notes_core_app_paths_new_with_dirs(
    data_dir: *const c_char,
    notes_dir: *const c_char,
) -> *mut AppPaths {
    let data_path = unsafe { cstr_to_path(data_dir) };
    let notes_path = unsafe { cstr_to_path(notes_dir) };

    let resolved_data = if data_path.as_os_str().is_empty() {
        AppPaths::default_data_dir()
    } else {
        data_path
    };

    let resolved_notes = if notes_path.as_os_str().is_empty() {
        AppPaths::default_notes_dir()
    } else {
        notes_path
    };

    Box::into_raw(Box::new(AppPaths::with_dirs(resolved_data, resolved_notes)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use crate::ffi::common::cstr_to_string;

    #[test]
    fn test_ffi_app_paths_with_dirs() {
        let c_data = CString::new("/tmp/test_data").unwrap();
        let c_notes = CString::new("/tmp/test_notes").unwrap();

        let paths = notes_core_app_paths_new_with_dirs(c_data.as_ptr(), c_notes.as_ptr());
        assert!(!paths.is_null());

        let data_ret = notes_core_app_paths_data_dir(paths);
        let notes_ret = notes_core_app_paths_notes_dir(paths);
        let db_ret = notes_core_app_paths_db_path(paths);

        assert_eq!(unsafe { cstr_to_string(data_ret) }, "/tmp/test_data");
        assert_eq!(unsafe { cstr_to_string(notes_ret) }, "/tmp/test_notes");
        assert_eq!(unsafe { cstr_to_string(db_ret) }, "/tmp/test_data/notesplus.db");

        crate::ffi::common::notes_core_free_string(data_ret);
        crate::ffi::common::notes_core_free_string(notes_ret);
        crate::ffi::common::notes_core_free_string(db_ret);
        notes_core_app_paths_free(paths);
    }

    #[test]
    fn test_ffi_app_paths_new_with_null_dirs_falls_back() {
        let paths = notes_core_app_paths_new_with_dirs(std::ptr::null(), std::ptr::null());
        assert!(!paths.is_null());

        let data_ret = notes_core_app_paths_data_dir(paths);
        let notes_ret = notes_core_app_paths_notes_dir(paths);

        assert_eq!(unsafe { cstr_to_string(data_ret) }, AppPaths::default_data_dir().to_str().unwrap());
        assert_eq!(unsafe { cstr_to_string(notes_ret) }, AppPaths::default_notes_dir().to_str().unwrap());

        crate::ffi::common::notes_core_free_string(data_ret);
        crate::ffi::common::notes_core_free_string(notes_ret);
        notes_core_app_paths_free(paths);
    }
}
