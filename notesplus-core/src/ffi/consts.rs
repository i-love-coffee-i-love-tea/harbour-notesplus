use std::os::raw::c_char;
use crate::constants;
use super::common::string_to_c;

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
pub extern "C" fn notes_core_const_default_system_prompt() -> *mut c_char {
    string_to_c(crate::agent::prompt::DEFAULT_SYSTEM_PROMPT.to_string())
}

#[no_mangle]
pub extern "C" fn notes_core_const_default_server_port() -> u16 {
    constants::DEFAULT_SERVER_PORT
}

#[no_mangle]
pub extern "C" fn notes_core_const_app_version() -> *mut c_char {
    string_to_c(env!("CARGO_PKG_VERSION").to_string())
}
