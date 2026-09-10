//! Microphone capture for offline STT.
//!
//! Produces 16-bit PCM mono WAV at 16 kHz using:
//! 1. In-process PulseAudio recording via the PulseAudio Simple API (`libpulse-simple.so.0`),
//!    which runs directly on any standard Sailfish OS installation with zero extra packages.
//! 2. Fallback to platform CLI capture tools (`parecord` / `arecord`) if PulseAudio in-process
//!    is not available.

use std::fs;
use std::io::{self, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use libc::{c_char, c_int, c_void, dlclose, dlopen, dlsym, RTLD_LAZY, RTLD_LOCAL};

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u16 = 1;
const RECORDING_FILE_NAME: &str = "stt-recording.wav";

// ============================================================================
// PulseAudio Simple API FFI (dynamically loaded via dlopen)
// ============================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaSampleSpec {
    pub format: c_int, // PA_SAMPLE_S16LE = 3
    pub rate: u32,     // 16000
    pub channels: u8,  // 1
}

pub type PaSimple = c_void;

type PaSimpleNewFn = unsafe extern "C" fn(
    server: *const c_char,
    name: *const c_char,
    dir: c_int, // PA_STREAM_RECORD = 2
    dev: *const c_char,
    stream_name: *const c_char,
    ss: *const PaSampleSpec,
    map: *const c_void,
    attr: *const c_void,
    error: *mut c_int,
) -> *mut PaSimple;

type PaSimpleReadFn = unsafe extern "C" fn(
    s: *mut PaSimple,
    data: *mut c_void,
    bytes: usize,
    error: *mut c_int,
) -> c_int;

type PaSimpleFreeFn = unsafe extern "C" fn(s: *mut PaSimple);

type PaStrerrorFn = unsafe extern "C" fn(error: c_int) -> *const c_char;

struct PulseAudioLib {
    handle: usize,
    pulse_handle: Option<usize>,
    pa_simple_new: PaSimpleNewFn,
    pa_simple_read: PaSimpleReadFn,
    pa_simple_free: PaSimpleFreeFn,
    pa_strerror: Option<PaStrerrorFn>,
}

unsafe impl Send for PulseAudioLib {}
unsafe impl Sync for PulseAudioLib {}

impl PulseAudioLib {
    fn load() -> Option<Arc<Self>> {
        unsafe {
            let names = [
                b"libpulse-simple.so.0\0".as_ptr() as *const c_char,
                b"libpulse-simple.so\0".as_ptr() as *const c_char,
            ];
            let mut handle: *mut c_void = std::ptr::null_mut();
            for name in &names {
                handle = dlopen(*name, RTLD_LAZY | RTLD_LOCAL);
                if !handle.is_null() {
                    break;
                }
            }
            if handle.is_null() {
                return None;
            }

            let new_sym = dlsym(handle, b"pa_simple_new\0".as_ptr() as *const c_char);
            let read_sym = dlsym(handle, b"pa_simple_read\0".as_ptr() as *const c_char);
            let free_sym = dlsym(handle, b"pa_simple_free\0".as_ptr() as *const c_char);

            if new_sym.is_null() || read_sym.is_null() || free_sym.is_null() {
                dlclose(handle);
                return None;
            }

            let pulse_names = [
                b"libpulse.so.0\0".as_ptr() as *const c_char,
                b"libpulse.so\0".as_ptr() as *const c_char,
            ];
            let mut pulse_handle: *mut c_void = std::ptr::null_mut();
            for name in &pulse_names {
                pulse_handle = dlopen(*name, RTLD_LAZY | RTLD_LOCAL);
                if !pulse_handle.is_null() {
                    break;
                }
            }
            let strerror_sym = if !pulse_handle.is_null() {
                dlsym(pulse_handle, b"pa_strerror\0".as_ptr() as *const c_char)
            } else {
                dlsym(handle, b"pa_strerror\0".as_ptr() as *const c_char)
            };

            let pa_strerror = if !strerror_sym.is_null() {
                Some(std::mem::transmute::<*mut c_void, PaStrerrorFn>(strerror_sym))
            } else {
                None
            };

            Some(Arc::new(Self {
                handle: handle as usize,
                pulse_handle: if !pulse_handle.is_null() {
                    Some(pulse_handle as usize)
                } else {
                    None
                },
                pa_simple_new: std::mem::transmute(new_sym),
                pa_simple_read: std::mem::transmute(read_sym),
                pa_simple_free: std::mem::transmute(free_sym),
                pa_strerror,
            }))
        }
    }

    fn strerror(&self, err: c_int) -> String {
        if let Some(f) = self.pa_strerror {
            unsafe {
                let ptr = f(err);
                if !ptr.is_null() {
                    return std::ffi::CStr::from_ptr(ptr)
                        .to_string_lossy()
                        .to_string();
                }
            }
        }
        format!("PulseAudio error code {}", err)
    }
}

impl Drop for PulseAudioLib {
    fn drop(&mut self) {
        unsafe {
            if self.handle != 0 {
                dlclose(self.handle as *mut c_void);
            }
            if let Some(h) = self.pulse_handle {
                if h != 0 {
                    dlclose(h as *mut c_void);
                }
            }
        }
    }
}

// ============================================================================
// WAV Format Encoding Helpers
// ============================================================================

pub fn write_wav_header<W: Write>(writer: &mut W, data_len: u32) -> io::Result<()> {
    let sample_rate: u32 = SAMPLE_RATE;
    let num_channels: u16 = CHANNELS;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * (num_channels as u32) * ((bits_per_sample / 8) as u32);
    let block_align: u16 = num_channels * (bits_per_sample / 8);
    let riff_chunk_size: u32 = 36 + data_len;

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_chunk_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    writer.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 = PCM)
    writer.write_all(&num_channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_len.to_le_bytes())?;
    Ok(())
}

pub fn finalize_wav_header(file: &mut fs::File, data_len: u32) -> io::Result<()> {
    let riff_chunk_size: u32 = 36 + data_len;
    file.seek(io::SeekFrom::Start(4))?;
    file.write_all(&riff_chunk_size.to_le_bytes())?;
    file.seek(io::SeekFrom::Start(40))?;
    file.write_all(&data_len.to_le_bytes())?;
    file.flush()?;
    Ok(())
}

// ============================================================================
// Active Recording Session
// ============================================================================

#[derive(Debug, Clone)]
pub struct AudioLevelState {
    pub level: f64,
    pub peak: f64,
    pub history: Vec<f64>,
}

impl Default for AudioLevelState {
    fn default() -> Self {
        Self {
            level: 0.0,
            peak: 0.0,
            history: vec![0.0; 7],
        }
    }
}

impl AudioLevelState {
    pub fn push(&mut self, val: f64) {
        if self.history.len() >= 7 {
            self.history.remove(0);
        }
        self.history.push(val);
    }
}

enum ActiveSession {
    InProcessPulse {
        stop_flag: Arc<AtomicBool>,
        thread_handle: std::thread::JoinHandle<Result<u64, String>>,
        path: PathBuf,
    },
    Subprocess {
        child: Child,
        path: PathBuf,
        backend: &'static str,
    },
}

struct ActiveRecording {
    session: ActiveSession,
    path: PathBuf,
    backend: &'static str,
}

/// Thread-safe microphone recorder that writes Whisper-compatible WAV files.
#[derive(Clone, Default)]
pub struct AudioRecorder {
    inner: Arc<Mutex<RecorderState>>,
    level_slot: Arc<Mutex<AudioLevelState>>,
}

#[derive(Default)]
struct RecorderState {
    active: Option<ActiveRecording>,
    last_path: Option<PathBuf>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Default output path under the app cache directory.
    pub fn default_output_path() -> PathBuf {
        let cache_root = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
                PathBuf::from(home).join(".cache")
            });
        let dir = cache_root.join("harbour-notesplusplus");
        let _ = fs::create_dir_all(&dir);
        dir.join(RECORDING_FILE_NAME)
    }

    pub fn is_recording(&self) -> bool {
        self.inner
            .lock()
            .map(|s| s.active.is_some())
            .unwrap_or(false)
    }


    /// Retrieve the current normalized audio volume level (0.0 .. 1.0) and waveform history.
    pub fn get_level_and_history(&self) -> (f64, Vec<f64>) {
        if let Ok(lvl) = self.level_slot.lock() {
            (lvl.level, lvl.history.clone())
        } else {
            (0.0, vec![0.0; 7])
        }
    }

    /// Start capturing audio to `path` (or the default cache path when `None`).
    pub fn start(&self, path: Option<PathBuf>) -> Result<PathBuf, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|e| format!("Recorder lock error: {}", e))?;

        if state.active.is_some() {
            return Err("Already recording".to_string());
        }

        if let Ok(mut lvl) = self.level_slot.lock() {
            *lvl = AudioLevelState::default();
        }

        let output = path.unwrap_or_else(Self::default_output_path);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create recording directory {}: {}",
                    parent.display(),
                    e
                )
            })?;
        }
        // Remove any stale file so consumers don't read a previous take.
        let _ = fs::remove_file(&output);

        // 1. Prefer in-process PulseAudio Simple API capture (zero extra CLI packages needed).
        let (session, backend) = match try_start_pulseaudio(&output, Arc::clone(&self.level_slot)) {
            Ok((sess, b)) => (sess, b),
            Err(pulse_err) => {
                log::warn!(
                    "PulseAudio in-process capture unavailable ({}); falling back to CLI tools",
                    pulse_err
                );
                // 2. Fall back to external parecord / arecord processes.
                let (child, b) = spawn_subprocess_recorder(&output)?;
                (
                    ActiveSession::Subprocess {
                        child,
                        path: output.clone(),
                        backend: b,
                    },
                    b,
                )
            }
        };

        let path_clone = output.clone();
        state.active = Some(ActiveRecording {
            session,
            path: path_clone.clone(),
            backend,
        });
        state.last_path = Some(path_clone.clone());
        log::info!(
            "STT recording started via {} -> {}",
            backend,
            path_clone.display()
        );
        Ok(path_clone)
    }

    /// Stop capture gracefully and return the finalized WAV path.
    pub fn stop(&self) -> Result<PathBuf, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|e| format!("Recorder lock error: {}", e))?;

        let active = state
            .active
            .take()
            .ok_or_else(|| "Not currently recording".to_string())?;

        match active.session {
            ActiveSession::InProcessPulse {
                stop_flag,
                thread_handle,
                path,
            } => {
                stop_flag.store(true, Ordering::SeqCst);
                let _ = thread_handle
                    .join()
                    .map_err(|_| "Recording thread panicked".to_string())??;

                if !path.is_file() {
                    return Err(format!(
                        "Recording finished but file is missing: {}",
                        path.display()
                    ));
                }

                let meta = fs::metadata(&path).map_err(|e| {
                    format!("Failed to stat recording {}: {}", path.display(), e)
                })?;
                if meta.len() < 64 {
                    let _ = fs::remove_file(&path);
                    return Err("Recording produced no audio data".to_string());
                }

                if let Ok(mut lvl) = self.level_slot.lock() {
                    *lvl = AudioLevelState::default();
                }

                state.last_path = Some(path.clone());
                log::info!(
                    "STT in-process recording stopped ({} bytes) -> {}",
                    meta.len(),
                    path.display()
                );
                Ok(path)
            }
            ActiveSession::Subprocess {
                mut child,
                path,
                backend,
            } => {
                finalize_child(&mut child, backend)?;

                if !path.is_file() {
                    return Err(format!(
                        "Recording finished but file is missing: {}",
                        path.display()
                    ));
                }

                let meta = fs::metadata(&path).map_err(|e| {
                    format!("Failed to stat recording {}: {}", path.display(), e)
                })?;
                if meta.len() < 64 {
                    let _ = fs::remove_file(&path);
                    return Err("Recording produced no audio data".to_string());
                }

                if let Ok(mut lvl) = self.level_slot.lock() {
                    *lvl = AudioLevelState::default();
                }

                state.last_path = Some(path.clone());
                log::info!(
                    "STT subprocess recording stopped ({} bytes) -> {}",
                    meta.len(),
                    path.display()
                );
                Ok(path)
            }
        }
    }

    /// Cancel capture and delete the partial file. Idempotent when idle.
    pub fn cancel(&self) {
        if let Ok(mut lvl) = self.level_slot.lock() {
            *lvl = AudioLevelState::default();
        }

        let mut state = match self.inner.lock() {
            Ok(s) => s,
            Err(_) => return,
        };

        if let Some(active) = state.active.take() {
            match active.session {
                ActiveSession::InProcessPulse {
                    stop_flag,
                    thread_handle,
                    path,
                } => {
                    stop_flag.store(true, Ordering::SeqCst);
                    let _ = thread_handle.join();
                    let _ = fs::remove_file(&path);
                    log::info!("STT in-process recording cancelled");
                }
                ActiveSession::Subprocess {
                    mut child,
                    path,
                    ..
                } => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(&path);
                    log::info!("STT subprocess recording cancelled");
                }
            }
        }
    }
}

// ============================================================================
// In-Process PulseAudio Recording Implementation
// ============================================================================

fn try_start_pulseaudio(
    output: &Path,
    level_slot: Arc<Mutex<AudioLevelState>>,
) -> Result<(ActiveSession, &'static str), String> {
    let pa_lib = PulseAudioLib::load().ok_or_else(|| {
        "PulseAudio Simple API (libpulse-simple.so.0) could not be loaded".to_string()
    })?;

    let spec = PaSampleSpec {
        format: 3, // PA_SAMPLE_S16LE
        rate: SAMPLE_RATE,
        channels: CHANNELS as u8,
    };

    let mut pa_err: c_int = 0;
    let app_name = std::ffi::CString::new("Notes++").unwrap();
    let stream_name = std::ffi::CString::new("STT Voice Input").unwrap();

    let simple = unsafe {
        (pa_lib.pa_simple_new)(
            std::ptr::null(),
            app_name.as_ptr(),
            2, // PA_STREAM_RECORD
            std::ptr::null(),
            stream_name.as_ptr(),
            &spec,
            std::ptr::null(),
            std::ptr::null(),
            &mut pa_err,
        )
    };

    if simple.is_null() {
        return Err(format!(
            "Failed to connect to PulseAudio recording stream: {}",
            pa_lib.strerror(pa_err)
        ));
    }

    let mut file = fs::File::create(output)
        .map_err(|e| {
            unsafe { (pa_lib.pa_simple_free)(simple) };
            format!("Failed to create recording file: {}", e)
        })?;

    if let Err(e) = write_wav_header(&mut file, 0) {
        unsafe { (pa_lib.pa_simple_free)(simple) };
        return Err(format!("Failed to write initial WAV header: {}", e));
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);
    let path_clone = output.to_path_buf();
    let s_addr = simple as usize;
    let pa_lib_clone = Arc::clone(&pa_lib);

    let thread_handle = std::thread::spawn(move || -> Result<u64, String> {
        let s = s_addr as *mut PaSimple;
        let mut buffer = [0u8; 2048]; // ~64ms buffer chunk at 16kHz 16-bit mono
        let mut total_bytes: u64 = 0;
        let mut err_code: c_int = 0;

        while !stop_flag_clone.load(Ordering::Relaxed) {
            let rc = unsafe {
                (pa_lib_clone.pa_simple_read)(
                    s,
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len(),
                    &mut err_code,
                )
            };
            if rc < 0 {
                unsafe { (pa_lib_clone.pa_simple_free)(s) };
                return Err(format!("PulseAudio read error: {}", pa_lib_clone.strerror(err_code)));
            }
            if let Err(e) = file.write_all(&buffer) {
                unsafe { (pa_lib_clone.pa_simple_free)(s) };
                return Err(format!("File write error: {}", e));
            }
            total_bytes += buffer.len() as u64;

            // Live RMS and peak volume calculation
            let sample_count = buffer.len() / 2;
            if sample_count > 0 {
                let mut sum_squares = 0.0f64;
                let mut peak: i16 = 0;
                for chunk in buffer.chunks_exact(2) {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    let abs_sample = sample.saturating_abs();
                    if abs_sample > peak {
                        peak = abs_sample;
                    }
                    let norm = (sample as f64) / 32768.0;
                    sum_squares += norm * norm;
                }
                let rms = (sum_squares / sample_count as f64).sqrt();
                // Perceptual boost so normal speech (0.01 - 0.15 RMS) maps to 0.15 - 0.85
                let boosted = (rms * 4.5).min(1.0);
                let peak_norm = (peak as f64 / 32768.0).min(1.0);

                if let Ok(mut l) = level_slot.lock() {
                    l.level = boosted;
                    l.peak = peak_norm;
                    l.push(boosted);
                }
            }
        }

        unsafe { (pa_lib_clone.pa_simple_free)(s) };

        if let Err(e) = finalize_wav_header(&mut file, total_bytes as u32) {
            return Err(format!("Failed to finalize WAV header: {}", e));
        }
        let _ = file.flush();
        Ok(total_bytes)
    });

    Ok((
        ActiveSession::InProcessPulse {
            stop_flag,
            thread_handle,
            path: path_clone,
        },
        "pulseaudio-inprocess",
    ))
}

// ============================================================================
// Subprocess Recording Fallbacks
// ============================================================================

fn spawn_subprocess_recorder(output: &Path) -> Result<(Child, &'static str), String> {
    if let Ok(child) = spawn_and_verify(|| spawn_parecord(output)) {
        return Ok((child, "parecord"));
    }
    if let Ok(child) = spawn_and_verify(|| spawn_arecord(output)) {
        return Ok((child, "arecord"));
    }

    Err(
        "No audio recorder available. PulseAudio in-process recording failed and neither parecord nor arecord is installed."
            .to_string(),
    )
}

fn spawn_parecord(output: &Path) -> io::Result<Child> {
    Command::new("parecord")
        .args([
            "--channels",
            &CHANNELS.to_string(),
            "--rate",
            &SAMPLE_RATE.to_string(),
            "--format=s16le",
            "--file-format=wav",
        ])
        .arg(output.as_os_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

fn spawn_and_verify(mut spawn: impl FnMut() -> io::Result<Child>) -> io::Result<Child> {
    let mut child = spawn()?;
    std::thread::sleep(Duration::from_millis(80));
    match child.try_wait() {
        Ok(Some(status)) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("recorder exited immediately with status {}", status),
        )),
        Ok(None) => Ok(child),
        Err(e) => {
            let _ = child.kill();
            Err(e)
        }
    }
}

fn spawn_arecord(output: &Path) -> io::Result<Child> {
    Command::new("arecord")
        .args([
            "-q",
            "-f",
            "S16_LE",
            "-r",
            &SAMPLE_RATE.to_string(),
            "-c",
            &CHANNELS.to_string(),
            "-t",
            "wav",
            "-D",
            "default",
        ])
        .arg(output.as_os_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

fn finalize_child(child: &mut Child, backend: &str) -> Result<(), String> {
    send_sigint(child).map_err(|e| format!("Failed to stop {}: {}", backend, e))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return Ok(());
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("Failed waiting for {}: {}", backend, e));
            }
        }
    }
}

#[cfg(unix)]
fn send_sigint(child: &Child) -> io::Result<()> {
    let pid = child.id() as i32;
    let rc = unsafe { libc::kill(pid, libc::SIGINT) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn send_sigint(child: &mut Child) -> io::Result<()> {
    child.kill()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_path_is_wav_under_cache() {
        let path = AudioRecorder::default_output_path();
        assert!(path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .ends_with(".wav"));
        assert!(path.to_string_lossy().contains("harbour-notesplusplus"));
    }

    #[test]
    fn stop_without_start_errors() {
        let rec = AudioRecorder::new();
        assert!(rec.stop().is_err());
        assert!(!rec.is_recording());
    }

    #[test]
    fn cancel_when_idle_is_safe() {
        let rec = AudioRecorder::new();
        rec.cancel();
        assert!(!rec.is_recording());
    }

    #[test]
    fn test_wav_header_generation_and_decoding() {
        let mut buffer = Vec::new();
        let num_samples = 16_000; // 1 second of audio
        let pcm_data = vec![0u8; num_samples * 2]; // 16-bit mono = 32000 bytes

        write_wav_header(&mut buffer, pcm_data.len() as u32).unwrap();
        buffer.extend_from_slice(&pcm_data);

        let info = notesplusplus_core::stt::parse_wav_header(&buffer).unwrap();
        assert_eq!(info.sample_rate, 16000);
        assert_eq!(info.num_channels, 1);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.audio_format, 1);
        assert_eq!(info.total_samples, 16000);
    }
}
