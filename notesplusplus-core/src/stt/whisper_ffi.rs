//! Thin, safe Rust wrapper around the vendored whisper.cpp C ABI shim (`whisper_bridge`).
//!
//! Only a handful of functions with primitive-typed signatures are declared here
//! (see `third_party/whisper.cpp/whisper_bridge.h`); all whisper.cpp struct layouts
//! (`whisper_full_params`, `whisper_context_params`, etc.) are handled entirely on the
//! C++ side, so no bindgen-generated bindings are required and no ABI mismatch risk
//! exists on the Rust side.

use crate::error::NotesError;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int};
use std::path::Path;

/// Opaque handle to a loaded whisper.cpp model context.
#[repr(C)]
struct WhisperBridgeCtx {
    _private: [u8; 0],
}

extern "C" {
    fn whisper_bridge_init(model_path: *const c_char) -> *mut WhisperBridgeCtx;
    fn whisper_bridge_free(ctx: *mut WhisperBridgeCtx);
    fn whisper_bridge_is_multilingual(ctx: *mut WhisperBridgeCtx) -> c_int;
    fn whisper_bridge_transcribe(
        ctx: *mut WhisperBridgeCtx,
        samples: *const c_float,
        n_samples: c_int,
        language: *const c_char,
        n_threads: c_int,
    ) -> *mut c_char;
    fn whisper_bridge_free_string(s: *mut c_char);
}

/// A loaded, ready-to-use Whisper inference context backed by whisper.cpp.
pub struct WhisperContext {
    ctx: *mut WhisperBridgeCtx,
}

// The underlying `whisper_context` is only ever accessed through `&self`/`&mut self`
// methods that we serialize with the FFI calls below; whisper.cpp itself does not use
// any thread-local state tied to the OS thread that created the context.
unsafe impl Send for WhisperContext {}

impl WhisperContext {
    /// Loads a GGML Whisper model from disk via whisper.cpp.
    pub fn load(model_path: &Path) -> Result<Self, NotesError> {
        let path_str = model_path.to_string_lossy();
        let c_path = CString::new(path_str.as_bytes())
            .map_err(|_| NotesError::SttModel("Model path contains an interior NUL byte".to_string()))?;

        let ctx = unsafe { whisper_bridge_init(c_path.as_ptr()) };
        if ctx.is_null() {
            return Err(NotesError::SttModel(format!(
                "whisper.cpp failed to load model at {}",
                model_path.display()
            )));
        }

        Ok(Self { ctx })
    }

    /// Returns true if the loaded model supports languages other than English.
    pub fn is_multilingual(&self) -> bool {
        unsafe { whisper_bridge_is_multilingual(self.ctx) != 0 }
    }

    /// Runs full Whisper inference (mel -> encoder -> decoder) over 16kHz mono `f32`
    /// PCM `samples`, returning the concatenated, trimmed transcription text.
    ///
    /// `language` may be `None`/`"auto"` to let whisper.cpp auto-detect the spoken
    /// language on multilingual models. `n_threads` is capped to a sane range by the
    /// caller; a value `<= 0` falls back to a whisper.cpp-internal default.
    pub fn transcribe(
        &self,
        samples: &[f32],
        language: Option<&str>,
        n_threads: i32,
    ) -> Result<String, NotesError> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        let c_language = match language {
            Some(lang) if !lang.is_empty() && lang != "auto" => Some(
                CString::new(lang)
                    .map_err(|_| NotesError::SttModel("Language code contains a NUL byte".to_string()))?,
            ),
            _ => None,
        };
        let language_ptr = c_language
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());

        let result_ptr = unsafe {
            whisper_bridge_transcribe(
                self.ctx,
                samples.as_ptr(),
                samples.len() as c_int,
                language_ptr,
                n_threads as c_int,
            )
        };

        if result_ptr.is_null() {
            return Err(NotesError::SttInference(
                "whisper.cpp inference failed (whisper_full returned an error)".to_string(),
            ));
        }

        let text = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };
        unsafe { whisper_bridge_free_string(result_ptr) };

        Ok(text)
    }
}

impl Drop for WhisperContext {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { whisper_bridge_free(self.ctx) };
        }
    }
}

impl std::fmt::Debug for WhisperContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhisperContext").finish_non_exhaustive()
    }
}
