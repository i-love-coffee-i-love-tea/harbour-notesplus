//! QMetaObject bridge exposing Speech-to-Text (STT) and model management to Sailfish OS QML.

use qmetaobject::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use notesplusplus_core::paths::AppPaths;
use notesplusplus_core::stt::{
    find_model_by_id, get_model_catalog, model_file_path, ModelDownloader, SttError, SttModelInfo,
    WhisperEngine,
};

use crate::audio_recorder::AudioRecorder;

const DEFAULT_WAVEFORM_JSON: &str = "[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]";

#[derive(QObject)]
pub struct SpeechBridge {
    base: qt_base_class!(trait QObject),

    // Properties
    is_recording: qt_property!(bool; NOTIFY recording_changed),
    is_transcribing: qt_property!(bool; NOTIFY transcribing_changed),
    is_downloading: qt_property!(bool; NOTIFY downloading_changed),
    download_progress: qt_property!(f64; NOTIFY download_progress_changed),
    downloading_model_id: qt_property!(String; NOTIFY downloading_changed),
    active_model_id: qt_property!(String; NOTIFY active_model_changed),
    available_models_json: qt_property!(String; NOTIFY models_changed),
    installed_models_json: qt_property!(String; NOTIFY models_changed),
    last_transcription: qt_property!(String; NOTIFY transcription_completed),
    error_message: qt_property!(String; NOTIFY error_occurred),
    has_installed_models: qt_property!(bool; NOTIFY models_changed),
    /// Absolute path of the last completed (or in-progress) recording WAV file.
    recording_file_path: qt_property!(String; NOTIFY recording_changed),
    /// Live microphone volume level (0.0 to 1.0) during active recording.
    audio_level: qt_property!(f64; NOTIFY audio_level_changed),
    /// JSON-encoded array of recent audio levels for waveform bar rendering.
    waveform_json: qt_property!(String; NOTIFY audio_level_changed),

    // Signals
    recording_changed: qt_signal!(),
    transcribing_changed: qt_signal!(),
    downloading_changed: qt_signal!(),
    download_progress_changed: qt_signal!(),
    active_model_changed: qt_signal!(),
    models_changed: qt_signal!(),
    audio_level_changed: qt_signal!(),
    transcription_completed: qt_signal!(text: String),
    download_completed: qt_signal!(model_id: String),
    error_occurred: qt_signal!(message: String),

    // Methods
    transcribe_file: qt_method!(fn(&mut self, path: String)),
    download_model: qt_method!(fn(&mut self, model_id: String)),
    cancel_download: qt_method!(fn(&mut self)),
    delete_model: qt_method!(fn(&mut self, model_id: String) -> bool),
    set_active_model: qt_method!(fn(&mut self, model_id: String)),
    refresh_models: qt_method!(fn(&mut self)),
    poll_worker: qt_method!(fn(&mut self) -> bool),
    /// Start microphone capture to a 16 kHz mono WAV file.
    start_recording: qt_method!(fn(&mut self) -> bool),
    /// Stop capture without transcribing. Returns the WAV path or empty on failure/cancel.
    stop_recording: qt_method!(fn(&mut self) -> String),
    /// Stop capture and immediately queue offline transcription of the recording.
    stop_recording_and_transcribe: qt_method!(fn(&mut self)),
    /// Abort capture and delete the partial file (no transcription).
    cancel_recording: qt_method!(fn(&mut self)),
    is_model_installed: qt_method!(fn(&self, model_id: String) -> bool),
    get_model_info_json: qt_method!(fn(&self, model_id: String) -> String),
    get_last_transcription: qt_method!(fn(&self) -> String),
    get_error_message: qt_method!(fn(&self) -> String),

    // Internal state
    models_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    progress_slot: Arc<Mutex<f64>>,
    download_worker_result: Arc<Mutex<Option<Result<String, String>>>>,
    transcribe_worker_result: Arc<Mutex<Option<Result<String, String>>>>,
    cached_engine: Arc<Mutex<Option<(String, WhisperEngine)>>>,
    recorder: AudioRecorder,
}

impl Default for SpeechBridge {
    fn default() -> Self {
        let paths = AppPaths::new();
        let models_dir = paths.stt_models_dir();
        let _ = std::fs::create_dir_all(&models_dir);

        let catalog = get_model_catalog(&models_dir, None);
        let active_id = catalog
            .iter()
            .find(|m| m.is_installed)
            .map(|m| m.id.clone())
            .unwrap_or_default();

        let updated_catalog = get_model_catalog(
            &models_dir,
            if active_id.is_empty() {
                None
            } else {
                Some(&active_id)
            },
        );

        let available_json =
            serde_json::to_string(&updated_catalog).unwrap_or_else(|_| "[]".to_string());
        let installed_models: Vec<&SttModelInfo> =
            updated_catalog.iter().filter(|m| m.is_installed).collect();
        let installed_json =
            serde_json::to_string(&installed_models).unwrap_or_else(|_| "[]".to_string());
        let has_installed = !installed_models.is_empty();

        Self {
            base: Default::default(),
            is_recording: false,
            is_transcribing: false,
            is_downloading: false,
            download_progress: 0.0,
            downloading_model_id: String::new(),
            active_model_id: active_id,
            available_models_json: available_json,
            installed_models_json: installed_json,
            last_transcription: String::new(),
            error_message: String::new(),
            has_installed_models: has_installed,
            recording_file_path: String::new(),
            audio_level: 0.0,
            waveform_json: DEFAULT_WAVEFORM_JSON.to_string(),

            recording_changed: Default::default(),
            transcribing_changed: Default::default(),
            downloading_changed: Default::default(),
            download_progress_changed: Default::default(),
            active_model_changed: Default::default(),
            models_changed: Default::default(),
            audio_level_changed: Default::default(),
            transcription_completed: Default::default(),
            download_completed: Default::default(),
            error_occurred: Default::default(),

            transcribe_file: Default::default(),
            download_model: Default::default(),
            cancel_download: Default::default(),
            delete_model: Default::default(),
            set_active_model: Default::default(),
            refresh_models: Default::default(),
            poll_worker: Default::default(),
            start_recording: Default::default(),
            stop_recording: Default::default(),
            stop_recording_and_transcribe: Default::default(),
            cancel_recording: Default::default(),
            is_model_installed: Default::default(),
            get_model_info_json: Default::default(),
            get_last_transcription: Default::default(),
            get_error_message: Default::default(),

            models_dir,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
            download_worker_result: Arc::new(Mutex::new(None)),
            transcribe_worker_result: Arc::new(Mutex::new(None)),
            cached_engine: Arc::new(Mutex::new(None)),
            recorder: AudioRecorder::new(),
        }
    }
}

impl SpeechBridge {
    fn report_error(&mut self, msg: String) {
        self.error_message = msg;
        self.error_occurred(self.error_message.clone());
    }

    pub fn refresh_models(&mut self) {
        let catalog = get_model_catalog(
            &self.models_dir,
            if self.active_model_id.is_empty() {
                None
            } else {
                Some(&self.active_model_id)
            },
        );

        // If current active model is no longer installed (or wasn't set), pick first installed if available
        let active_installed = catalog
            .iter()
            .any(|m| m.id == self.active_model_id && m.is_installed);

        let final_catalog = if !active_installed {
            let first_installed = catalog
                .iter()
                .find(|m| m.is_installed)
                .map(|m| m.id.clone());
            if let Some(first_id) = first_installed {
                if self.active_model_id != first_id {
                    self.active_model_id = first_id;
                    self.active_model_changed();
                }
                get_model_catalog(&self.models_dir, Some(&self.active_model_id))
            } else {
                if !self.active_model_id.is_empty() {
                    self.active_model_id = String::new();
                    self.active_model_changed();
                }
                catalog
            }
        } else {
            catalog
        };

        self.available_models_json =
            serde_json::to_string(&final_catalog).unwrap_or_else(|_| "[]".to_string());
        let installed_models: Vec<&SttModelInfo> =
            final_catalog.iter().filter(|m| m.is_installed).collect();
        self.installed_models_json =
            serde_json::to_string(&installed_models).unwrap_or_else(|_| "[]".to_string());
        self.has_installed_models = !installed_models.is_empty();

        self.models_changed();
    }

    pub fn set_active_model(&mut self, model_id: String) {
        if self.active_model_id == model_id {
            return;
        }
        self.active_model_id = model_id;
        self.active_model_changed();
        self.refresh_models();
    }

    pub fn download_model(&mut self, model_id: String) {
        if self.is_downloading {
            self.cancel_download();
        }

        let model_info = match find_model_by_id(&model_id) {
            Some(m) => m,
            None => {
                let err = format!("Unknown model ID: {}", model_id);
                self.report_error(err);
                return;
            }
        };

        self.is_downloading = true;
        self.downloading_model_id = model_id.clone();
        self.download_progress = 0.0;
        self.error_message = String::new();
        self.downloading_changed();
        self.download_progress_changed();

        self.cancel_flag.store(false, Ordering::SeqCst);
        if let Ok(mut p) = self.progress_slot.lock() {
            *p = 0.0;
        }
        if let Ok(mut r) = self.download_worker_result.lock() {
            *r = None;
        }

        let cancel_flag = Arc::clone(&self.cancel_flag);
        let progress_slot = Arc::clone(&self.progress_slot);
        let result_slot = Arc::clone(&self.download_worker_result);
        let models_dir = self.models_dir.clone();
        let target_model_id = model_id;

        thread::spawn(move || {
            let downloader = ModelDownloader::new(&models_dir);
            let res = downloader.download(&model_info, Some(cancel_flag), |progress| {
                if let Ok(mut p) = progress_slot.lock() {
                    *p = progress.percent as f64;
                }
            });

            let output = match res {
                Ok(_) => {
                    if let Ok(mut p) = progress_slot.lock() {
                        *p = 100.0;
                    }
                    Ok(target_model_id)
                }
                Err(SttError::Cancelled) => Err("cancelled".to_string()),
                Err(e) => Err(format!("Download failed: {}", e)),
            };

            if let Ok(mut r) = result_slot.lock() {
                *r = Some(output);
            }
        });
    }

    pub fn cancel_download(&mut self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn delete_model(&mut self, model_id: String) -> bool {
        if self.is_downloading && self.downloading_model_id == model_id {
            self.cancel_download();
        }

        let downloader = ModelDownloader::new(&self.models_dir);
        let res = downloader.delete(&model_id);

        if let Ok(mut cached) = self.cached_engine.lock() {
            if let Some((cached_id, _)) = cached.as_ref() {
                if cached_id == &model_id {
                    *cached = None;
                }
            }
        }

        self.refresh_models();
        res.is_ok()
    }

    pub fn transcribe_file(&mut self, path: String) {
        if self.is_transcribing {
            return;
        }
        self.is_transcribing = true;
        self.error_message = String::new();
        self.transcribing_changed();
        self.begin_transcription(path);
    }

    fn begin_transcription(&mut self, path: String) {
        let trimmed_path = path.trim();
        let trimmed_path = if trimmed_path.starts_with("file://") {
            &trimmed_path[7..]
        } else {
            trimmed_path
        };
        if trimmed_path.is_empty() {
            let err = "Audio path is empty".to_string();
            eprintln!("[debug:stt] begin_transcription: {}", err);
            self.report_error(err);
            self.is_transcribing = false;
            self.transcribing_changed();
            return;
        }

        let audio_path = PathBuf::from(trimmed_path);
        if !audio_path.is_file() {
            let err = format!("Audio file not found: {}", trimmed_path);
            eprintln!("[debug:stt] begin_transcription: {}", err);
            self.report_error(err);
            self.is_transcribing = false;
            self.transcribing_changed();
            return;
        }

        // Determine which model to use
        let model_id = if !self.active_model_id.is_empty() {
            self.active_model_id.clone()
        } else {
            let catalog = get_model_catalog(&self.models_dir, None);
            match catalog.into_iter().find(|m| m.is_installed) {
                Some(m) => {
                    self.active_model_id = m.id.clone();
                    self.active_model_changed();
                    m.id
                }
                None => {
                    let err =
                        "No speech model installed. Please download a model first.".to_string();
                    eprintln!("[debug:stt] begin_transcription: {}", err);
                    self.report_error(err);
                    self.is_transcribing = false;
                    self.transcribing_changed();
                    return;
                }
            }
        };

        let model_path = model_file_path(&self.models_dir, &model_id);
        if !model_path.is_file() {
            let err = format!(
                "Model file for '{}' not found on disk. Please re-download the model.",
                model_id
            );
            eprintln!("[debug:stt] begin_transcription: {}", err);
            self.report_error(err);
            self.is_transcribing = false;
            self.transcribing_changed();
            return;
        }

        eprintln!(
            "[STT] begin_transcription: spawning thread for model={}, audio={}",
            model_id, trimmed_path
        );

        if let Ok(mut r) = self.transcribe_worker_result.lock() {
            *r = None;
        }

        let result_slot = Arc::clone(&self.transcribe_worker_result);
        let cached_engine_slot = Arc::clone(&self.cached_engine);

        thread::spawn(move || {
            let res = (|| -> Result<String, String> {
                let mut engine_guard = cached_engine_slot
                    .lock()
                    .map_err(|e| format!("Lock error: {}", e))?;

                let needs_load = match engine_guard.as_ref() {
                    Some((cached_id, _)) => cached_id != &model_id,
                    None => true,
                };

                if needs_load {
                    let engine = WhisperEngine::load(&model_path)
                        .map_err(|e| format!("Failed to load Whisper model: {}", e))?;
                    *engine_guard = Some((model_id.clone(), engine));
                }

                if let Some((_, ref engine)) = *engine_guard {
                    let text = engine
                        .transcribe_wav_file(&audio_path)
                        .map_err(|e| format!("Transcription error: {}", e))?;
                    Ok(text)
                } else {
                    Err("Internal error: engine failed to initialize".to_string())
                }
            })();

            if let Ok(mut r) = result_slot.lock() {
                *r = Some(res);
            }
        });
    }

    pub fn poll_worker(&mut self) -> bool {
        let mut state_changed = false;

        // Poll download progress
        if self.is_downloading {
            if let Ok(p) = self.progress_slot.lock() {
                if (*p - self.download_progress).abs() > 0.01 {
                    self.download_progress = *p;
                    self.download_progress_changed();
                    state_changed = true;
                }
            }
        }

        // Poll download completion
        let dl_result = match self.download_worker_result.lock() {
            Ok(mut r) => r.take(),
            Err(_) => None,
        };

        if let Some(res) = dl_result {
            self.is_downloading = false;
            self.downloading_model_id = String::new();
            self.downloading_changed();

            match res {
                Ok(model_id) => {
                    self.download_progress = 100.0;
                    self.download_progress_changed();
                    self.refresh_models();
                    if self.active_model_id.is_empty() {
                        self.set_active_model(model_id.clone());
                    }
                    self.download_completed(model_id);
                }
                Err(err) if err == "cancelled" => {
                    self.download_progress = 0.0;
                    self.download_progress_changed();
                    self.refresh_models();
                }
                Err(err) => {
                    self.report_error(err);
                }
            }
            state_changed = true;
        }

        // Poll transcription completion
        let tr_result = match self.transcribe_worker_result.lock() {
            Ok(mut r) => r.take(),
            Err(_) => None,
        };

        if let Some(res) = tr_result {
            self.is_transcribing = false;
            self.transcribing_changed();

            match res {
                Ok(text) => {
                    let cleaned = text
                        .replace("[BLANK_AUDIO]", "")
                        .replace("[MUSIC]", "")
                        .trim()
                        .to_string();
                    eprintln!("[debug:stt] Transcription completed: '{}' (cleaned: '{}')", text, cleaned);
                    if cleaned.is_empty() {
                        // whisper returned only a placeholder — treat as no speech
                        self.last_transcription = String::new();
                        self.transcription_completed(String::new());
                    } else {
                        self.last_transcription = cleaned.clone();
                        self.transcription_completed(cleaned);
                    }
                }
                Err(err) => {
                    self.report_error(err);
                }
            }
            state_changed = true;
        }

        // Poll live recording volume & waveform
        if self.is_recording {
            let (level, hist) = self.recorder.get_level_and_history();
            let wf_json = serde_json::to_string(&hist).unwrap_or_else(|_| "[]".to_string());
            if (self.audio_level - level).abs() > 0.001 || self.waveform_json != wf_json {
                self.audio_level = level;
                self.waveform_json = wf_json;
                self.audio_level_changed();
                state_changed = true;
            }
        }

        state_changed || self.is_downloading || self.is_transcribing || self.is_recording
    }

    pub fn start_recording(&mut self) -> bool {
        if self.is_recording {
            return true;
        }
        if self.is_transcribing {
            let err = "Cannot start recording while transcription is in progress".to_string();
            self.report_error(err);
            return false;
        }

        match self.recorder.start(None) {
            Ok(path) => {
                self.recording_file_path = path.to_string_lossy().to_string();
                self.is_recording = true;
                self.audio_level = 0.0;
                self.waveform_json = DEFAULT_WAVEFORM_JSON.to_string();
                self.error_message = String::new();
                self.recording_changed();
                self.audio_level_changed();
                true
            }
            Err(err) => {
                self.is_recording = false;
                self.recording_file_path = String::new();
                self.audio_level = 0.0;
                self.waveform_json = DEFAULT_WAVEFORM_JSON.to_string();
                self.report_error(err);
                self.recording_changed();
                self.audio_level_changed();
                false
            }
        }
    }

    pub fn stop_recording(&mut self) -> String {
        if !self.is_recording && !self.recorder.is_recording() {
            return self.recording_file_path.clone();
        }

        self.audio_level = 0.0;
        self.waveform_json = DEFAULT_WAVEFORM_JSON.to_string();
        self.audio_level_changed();

        match self.recorder.stop() {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.recording_file_path = path_str.clone();
                self.is_recording = false;
                self.recording_changed();
                path_str
            }
            Err(err) => {
                self.is_recording = false;
                self.recording_changed();
                self.report_error(err);
                String::new()
            }
        }
    }

    pub fn stop_recording_and_transcribe(&mut self) {
        eprintln!("[debug:stt] stop_recording_and_transcribe called");
        // Set transcribing flag before stopping recording so the poll timer
        // never sees both is_recording and is_transcribing as false.
        self.is_transcribing = true;
        self.error_message = String::new();
        self.transcribing_changed();
        let path = self.stop_recording();
        eprintln!("[debug:stt] stop_recording returned path: '{}'", path);
        if path.is_empty() {
            self.is_transcribing = false;
            self.transcribing_changed();
            return;
        }
        self.begin_transcription(path);
    }

    pub fn cancel_recording(&mut self) {
        self.recorder.cancel();
        self.audio_level = 0.0;
        self.waveform_json = DEFAULT_WAVEFORM_JSON.to_string();
        self.audio_level_changed();
        if self.is_recording || !self.recording_file_path.is_empty() {
            self.is_recording = false;
            self.recording_file_path = String::new();
            self.recording_changed();
        }
    }

    pub fn is_model_installed(&self, model_id: String) -> bool {
        let path = model_file_path(&self.models_dir, &model_id);
        path.is_file()
    }

    pub fn get_model_info_json(&self, model_id: String) -> String {
        let catalog = get_model_catalog(
            &self.models_dir,
            if self.active_model_id.is_empty() {
                None
            } else {
                Some(&self.active_model_id)
            },
        );
        if let Some(m) = catalog.into_iter().find(|m| m.id == model_id) {
            serde_json::to_string(&m).unwrap_or_default()
        } else {
            String::new()
        }
    }

    pub fn get_last_transcription(&self) -> String {
        self.last_transcription.clone()
    }

    pub fn get_error_message(&self) -> String {
        self.error_message.clone()
    }
}
