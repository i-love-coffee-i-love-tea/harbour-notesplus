use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::stt;
use super::common::{cstr_to_path, cstr_to_string, ffi_err, string_to_c};

/// Transcribe a WAV file. Returns allocated text or error string.
#[no_mangle]
pub extern "C" fn notes_core_stt_transcribe(
    model_path: *const c_char,
    wav_path: *const c_char,
) -> *mut c_char {
    let mp = unsafe { cstr_to_path(model_path) };
    let wp = unsafe { cstr_to_path(wav_path) };

    match stt::WhisperEngine::load(&mp) {
        Ok(engine) => match engine.transcribe_wav_file(&wp) {
            Ok(text) => string_to_c(text),
            Err(e) => ffi_err!(e),
        },
        Err(e) => ffi_err!(e),
    }
}

/// Get STT model catalog as JSON. Returns allocated string.
#[no_mangle]
pub extern "C" fn notes_core_stt_model_catalog_json(
    models_dir: *const c_char,
    active_model_id: *const c_char,
) -> *mut c_char {
    let dir = unsafe { cstr_to_path(models_dir) };
    let active = unsafe { cstr_to_string(active_model_id) };
    let active_opt = if active.is_empty() {
        None
    } else {
        Some(active.as_str())
    };
    let catalog = stt::get_model_catalog(&dir, active_opt);
    string_to_c(serde_json::to_string(&catalog).unwrap_or_else(|_| "[]".to_string()))
}

/// Delete an STT model. Returns 1 if deleted, 0 if not found.
#[no_mangle]
pub extern "C" fn notes_core_stt_model_delete(
    models_dir: *const c_char,
    model_id: *const c_char,
) -> i32 {
    let dir = unsafe { cstr_to_path(models_dir) };
    let id = unsafe { cstr_to_string(model_id) };
    let downloader = stt::ModelDownloader::new(&dir);
    match downloader.delete(&id) {
        Ok(deleted) => if deleted { 1 } else { 0 },
        Err(_) => 0,
    }
}

pub struct FfiSttDownload {
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<Mutex<f64>>,
    pub result: Arc<Mutex<Option<Result<String, String>>>>,
    pub _handle: Option<thread::JoinHandle<()>>,
}

/// Start downloading an STT model (background). Returns handle.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_start(
    models_dir: *const c_char,
    model_id: *const c_char,
) -> *mut FfiSttDownload {
    let dir = unsafe { cstr_to_path(models_dir) };
    let id = unsafe { cstr_to_string(model_id) };

    let model = match stt::find_model_by_id(&id) {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(Mutex::new(0.0_f64));
    let result = Arc::new(Mutex::new(None));

    let cancel_c = cancel.clone();
    let progress_c = progress.clone();
    let result_c = result.clone();

    let handle = thread::spawn(move || {
        let downloader = stt::ModelDownloader::new(&dir);
        let res = downloader.download(&model, Some(cancel_c), |prog| {
            if let Ok(mut p) = progress_c.lock() {
                *p = prog.percent as f64;
            }
        });
        if let Ok(mut guard) = result_c.lock() {
            *guard = Some(res.map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string()));
        }
    });

    Box::into_raw(Box::new(FfiSttDownload {
        cancel,
        progress,
        result,
        _handle: Some(handle),
    }))
}

/// Poll download progress. Returns: 0 = running, 1 = done, -1 = error.
/// *out_progress is set to 0.0-1.0, *out_result to allocated string on completion.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_poll(
    handle: *mut FfiSttDownload,
    out_progress: *mut f64,
    out_result: *mut *mut c_char,
) -> i32 {
    let h = unsafe { &*handle };

    let prog = h.progress.lock().map(|p| *p).unwrap_or(0.0);
    unsafe { *out_progress = prog };

    let has = h.result.lock().map(|g| g.is_some()).unwrap_or(false);
    if !has {
        return 0;
    }

    let res = h.result.lock().ok().and_then(|mut g| g.take());
    match res {
        Some(Ok(path)) => {
            unsafe { *out_result = string_to_c(path) };
            1
        }
        Some(Err(e)) => {
            unsafe { *out_result = string_to_c(e) };
            -1
        }
        None => 0,
    }
}

/// Cancel a download.
#[no_mangle]
pub extern "C" fn notes_core_stt_download_cancel(handle: *mut FfiSttDownload) {
    if !handle.is_null() {
        let h = unsafe { &*handle };
        h.cancel.store(true, Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "C" fn notes_core_stt_download_free(handle: *mut FfiSttDownload) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}
