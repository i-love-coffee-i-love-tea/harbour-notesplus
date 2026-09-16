use std::os::raw::c_char;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::agent::{
    AgentSession, AgentStepResult, LlmClient, LlmConfig, PendingConfirmation, PermissionConfig,
    PermissionManager,
};
use super::common::{cstr_to_path, cstr_to_string, ffi_err, string_to_c};

pub struct FfiAgentSession {
    pub session: Arc<Mutex<AgentSession>>,
    pub result: Arc<Mutex<Option<WorkerResult>>>,
    pub streaming_buffer: Arc<Mutex<String>>,
}

pub type FfiTokenCallback = unsafe extern "C" fn(user_data: *mut std::os::raw::c_void, token: *const c_char, is_done: bool);

struct SendCallback {
    callback: Option<FfiTokenCallback>,
    user_data_addr: usize,
}
unsafe impl Send for SendCallback {}

pub struct WorkerResult {
    pub step_result: Result<AgentStepResult, String>,
    pub messages_json: String,
    pub pending_action: Option<PendingConfirmation>,
    pub can_undo: bool,
    pub last_snapshot_id: Option<String>,
    pub last_created_note: Option<String>,
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
    let perm_config: PermissionConfig = serde_json::from_value(cfg.clone()).unwrap_or_default();
    let llm_config: LlmConfig = serde_json::from_value(cfg).unwrap_or_default();

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
///
/// # Safety
///
/// `ffi` must point to a valid `FfiAgentSession`. `prompt` must be a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn notes_core_agent_send(
    ffi: *mut FfiAgentSession,
    prompt: *const c_char,
) -> i32 {
    notes_core_agent_send_streaming(ffi, prompt, None, std::ptr::null_mut())
}

/// Send a prompt to the agent with a streaming token callback (background). Returns 0 on success.
///
/// # Safety
///
/// `ffi` must point to a valid `FfiAgentSession`. `prompt` must be a valid, null-terminated C string.
/// `callback` must be thread-safe or NULL.
#[no_mangle]
pub unsafe extern "C" fn notes_core_agent_send_streaming(
    ffi: *mut FfiAgentSession,
    prompt: *const c_char,
    callback: Option<FfiTokenCallback>,
    user_data: *mut std::os::raw::c_void,
) -> i32 {
    if ffi.is_null() { return -1; }
    let ffi = &mut *ffi;
    let prompt = cstr_to_string(prompt);

    let session = ffi.session.clone();
    let result_slot = ffi.result.clone();
    let buffer = ffi.streaming_buffer.clone();

    // Clear streaming buffer
    if let Ok(mut b) = buffer.lock() {
        b.clear();
    }

    let cb_wrapper = SendCallback {
        callback,
        user_data_addr: user_data as usize,
    };

    thread::spawn(move || {
        let step_result;
        let messages_json;
        let pending_action;
        let can_undo;
        let last_snapshot_id;
        let last_created_note;

        let SendCallback {
            callback: cb_opt,
            user_data_addr,
        } = cb_wrapper;
        let udata = user_data_addr as *mut std::os::raw::c_void;

        {
            let mut sess = session.lock().unwrap_or_else(|e| e.into_inner());
            let result = sess.send_prompt_streaming(&prompt, |tok| {
                if let Ok(mut b) = buffer.lock() {
                    b.push_str(tok);
                }
                if let Some(cb) = cb_opt {
                    let c_tok = string_to_c(tok.to_string());
                    unsafe {
                        cb(udata, c_tok, false);
                        crate::ffi::common::notes_core_free_string(c_tok);
                    }
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

        if let Some(cb) = cb_opt {
            unsafe {
                cb(udata, std::ptr::null(), true);
            }
        }
    });
    0
}

/// Poll agent streaming buffer. Returns accumulated tokens as allocated string.
#[no_mangle]
pub extern "C" fn notes_core_agent_poll_streaming(
    ffi: *mut FfiAgentSession,
) -> *mut c_char {
    if ffi.is_null() { return std::ptr::null_mut(); }
    let ffi = unsafe { &*ffi };
    let mut buf = ffi.streaming_buffer.lock().unwrap_or_else(|e| e.into_inner());
    string_to_c(std::mem::take(&mut *buf))
}

/// Poll for agent result. Returns: 0 = still running, 1 = ready, -1 = error.
/// If ready, *out_json is set to allocated JSON with result details.
#[no_mangle]
pub extern "C" fn notes_core_agent_poll(
    ffi: *mut FfiAgentSession,
    out_json: *mut *mut c_char,
) -> i32 {
    if ffi.is_null() { return -1; }
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
                "step_result": match &wr.step_result {
                    Ok(AgentStepResult::Finished { content, .. }) => serde_json::json!({
                        "type": "Finished",
                        "content": content,
                    }),
                    Ok(AgentStepResult::RequiresConfirmation(_)) => serde_json::json!({
                        "type": "RequiresConfirmation",
                    }),
                    Ok(AgentStepResult::Error(e)) => serde_json::json!({
                        "type": "Error",
                        "content": e,
                    }),
                    Err(e) => serde_json::json!({
                        "type": "Error",
                        "content": e,
                    }),
                },
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

/// Confirm or deny a pending action with streaming token callback (background).
#[no_mangle]
pub unsafe extern "C" fn notes_core_agent_confirm_streaming(
    ffi: *mut FfiAgentSession,
    approved: i32,
    callback: Option<FfiTokenCallback>,
    user_data: *mut std::os::raw::c_void,
) -> i32 {
    if ffi.is_null() { return -1; }
    let ffi = unsafe { &mut *ffi };
    let session = ffi.session.clone();
    let result_slot = ffi.result.clone();
    let buffer = ffi.streaming_buffer.clone();

    if let Ok(mut b) = buffer.lock() {
        b.clear();
    }

    let cb_wrapper = SendCallback {
        callback,
        user_data_addr: user_data as usize,
    };

    thread::spawn(move || {
        let step_result;
        let messages_json;
        let pending_action;
        let can_undo;
        let last_snapshot_id;
        let last_created_note;

        let SendCallback {
            callback: cb_opt,
            user_data_addr,
        } = cb_wrapper;
        let udata = user_data_addr as *mut std::os::raw::c_void;

        {
            let mut sess = session.lock().unwrap_or_else(|e| e.into_inner());
            let result = sess.confirm_pending_action_streaming(approved != 0, |tok| {
                if let Ok(mut b) = buffer.lock() {
                    b.push_str(tok);
                }
                if let Some(cb) = cb_opt {
                    let c_tok = string_to_c(tok.to_string());
                    unsafe {
                        cb(udata, c_tok, false);
                        crate::ffi::common::notes_core_free_string(c_tok);
                    }
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

        if let Some(cb) = cb_opt {
            unsafe {
                cb(udata, std::ptr::null(), true);
            }
        }
    });
    0
}

/// Confirm or deny a pending action (background).
#[no_mangle]
pub unsafe extern "C" fn notes_core_agent_confirm(
    ffi: *mut FfiAgentSession,
    approved: i32,
) -> i32 {
    notes_core_agent_confirm_streaming(ffi, approved, None, std::ptr::null_mut())
}

/// Undo the last agent action. Returns allocated result string.
#[no_mangle]
pub extern "C" fn notes_core_agent_undo(
    ffi: *mut FfiAgentSession,
) -> *mut c_char {
    if ffi.is_null() { return std::ptr::null_mut(); }
    let ffi = unsafe { &mut *ffi };
    let mut sess = ffi.session.lock().unwrap_or_else(|e| e.into_inner());
    match sess.undo_last_action() {
        Ok(msg) => string_to_c(msg),
        Err(e) => ffi_err!(e),
    }
}

/// Update agent configuration.
#[no_mangle]
pub extern "C" fn notes_core_agent_configure(
    ffi: *mut FfiAgentSession,
    config_json: *const c_char,
) {
    if ffi.is_null() { return; }
    let ffi = unsafe { &mut *ffi };
    let cfg_str = unsafe { cstr_to_string(config_json) };
    let cfg: serde_json::Value = serde_json::from_str(&cfg_str).unwrap_or_default();
    let perm_config: PermissionConfig = serde_json::from_value(cfg.clone()).unwrap_or_default();
    let llm_config: LlmConfig = serde_json::from_value(cfg).unwrap_or_default();

    let client = LlmClient::new(llm_config);
    let perm_mgr = PermissionManager::new(perm_config);
    let mut sess = ffi.session.lock().unwrap_or_else(|e| e.into_inner());
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
    if ffi.is_null() { return; }
    let ffi = unsafe { &mut *ffi };
    let fname = unsafe { cstr_to_string(context_filename) };
    let fcontent = unsafe { cstr_to_string(context_content) };
    let extra = unsafe { cstr_to_string(extra_context) };

    let mut sess = ffi.session.lock().unwrap_or_else(|e| e.into_inner());
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
