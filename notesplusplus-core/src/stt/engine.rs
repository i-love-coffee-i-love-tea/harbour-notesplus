use crate::error::NotesError;
#[cfg(feature = "whisper")]
use super::whisper_ffi::WhisperContext;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const GGML_MAGIC: u32 = 0x67676d6c; // "ggml" in little-endian
pub const GGMF_MAGIC: u32 = 0x67676d66; // "ggmf"
pub const GGJT_MAGIC: u32 = 0x67676a74; // "ggjt"
pub const GGUF_MAGIC: u32 = 0x46554747; // "GGUF" in little-endian

/// Information extracted from a WAV audio header.
#[derive(Debug, Clone, PartialEq)]
pub struct WavInfo {
    pub audio_format: u16,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub total_samples: usize,
    pub duration_secs: f32,
}

/// Parses and decodes a WAV file into normalized 16kHz mono `f32` samples in range `[-1.0, 1.0]`.
pub fn decode_wav_file(path: impl AsRef<Path>) -> Result<Vec<f32>, NotesError> {
    let mut file = File::open(path.as_ref()).map_err(|e| {
        NotesError::SttAudio(format!(
            "Failed to open audio file {}: {}",
            path.as_ref().display(),
            e
        ))
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    decode_wav_bytes(&bytes)
}

/// Parses and decodes WAV byte buffer into normalized 16kHz mono `f32` samples.
pub fn decode_wav_bytes(bytes: &[u8]) -> Result<Vec<f32>, NotesError> {
    let (info, raw_pcm) = parse_wav_chunks(bytes)?;

    let mut mono_samples = match info.audio_format {
        1 => decode_pcm_integer(raw_pcm, info.num_channels, info.bits_per_sample)?,
        3 => decode_pcm_float(raw_pcm, info.num_channels, info.bits_per_sample)?,
        other => {
            return Err(NotesError::SttAudio(format!(
                "Unsupported audio format tag: {} (only PCM 1 and IEEE Float 3 supported)",
                other
            )))
        }
    };

    if mono_samples.is_empty() {
        return Ok(Vec::new());
    }

    // Resample to 16000 Hz if needed
    if info.sample_rate != 16000 {
        mono_samples = resample_linear(&mono_samples, info.sample_rate, 16000);
    }

    Ok(mono_samples)
}

/// Parses the WAV header and returns metadata without decoding the entire audio payload.
pub fn parse_wav_header(bytes: &[u8]) -> Result<WavInfo, NotesError> {
    let (info, _) = parse_wav_chunks(bytes)?;
    Ok(info)
}

/// Internal helper to parse RIFF chunks and extract fmt and data slices.
fn parse_wav_chunks(bytes: &[u8]) -> Result<(WavInfo, &[u8]), NotesError> {
    if bytes.len() < 12 {
        return Err(NotesError::SttAudio(
            "Audio file too short to be a valid WAV".to_string(),
        ));
    }

    if &bytes[0..4] != b"RIFF" {
        return Err(NotesError::SttAudio(
            "Missing RIFF header magic in audio file".to_string(),
        ));
    }

    if &bytes[8..12] != b"WAVE" {
        return Err(NotesError::SttAudio(
            "Missing WAVE format tag in audio file".to_string(),
        ));
    }

    let mut pos = 12;
    let mut fmt_info: Option<(u16, u16, u32, u16)> = None;
    let mut data_slice: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let chunk_start = pos + 8;
        let chunk_end = chunk_start + chunk_size;

        if chunk_end > bytes.len() {
            if chunk_id == b"data" {
                data_slice = Some(&bytes[chunk_start..bytes.len()]);
                break;
            }
            return Err(NotesError::SttAudio(
                "Malformed chunk size in WAV file".to_string(),
            ));
        }

        let chunk_data = &bytes[chunk_start..chunk_end];

        if chunk_id == b"fmt " {
            if chunk_data.len() < 16 {
                return Err(NotesError::SttAudio(
                    "fmt chunk too small in WAV file".to_string(),
                ));
            }

            let audio_format = u16::from_le_bytes(chunk_data[0..2].try_into().unwrap());
            let num_channels = u16::from_le_bytes(chunk_data[2..4].try_into().unwrap());
            let sample_rate = u32::from_le_bytes(chunk_data[4..8].try_into().unwrap());
            let bits_per_sample = u16::from_le_bytes(chunk_data[14..16].try_into().unwrap());

            if num_channels == 0 {
                return Err(NotesError::SttAudio("WAV file has 0 channels".to_string()));
            }
            if sample_rate == 0 {
                return Err(NotesError::SttAudio(
                    "WAV file has 0 sample rate".to_string(),
                ));
            }

            fmt_info = Some((audio_format, num_channels, sample_rate, bits_per_sample));
        } else if chunk_id == b"data" {
            data_slice = Some(chunk_data);
        }

        let padded_size = (chunk_size + 1) & !1;
        pos += 8 + padded_size;
    }

    let (audio_format, num_channels, sample_rate, bits_per_sample) = fmt_info
        .ok_or_else(|| NotesError::SttAudio("Missing 'fmt ' chunk in WAV file".to_string()))?;

    let raw_pcm = data_slice
        .ok_or_else(|| NotesError::SttAudio("Missing 'data' chunk in WAV file".to_string()))?;

    let bytes_per_sample = (bits_per_sample as usize) / 8;
    let block_align = (num_channels as usize) * bytes_per_sample;
    let total_samples = if block_align > 0 {
        raw_pcm.len() / block_align
    } else {
        0
    };
    let duration_secs = if sample_rate > 0 {
        total_samples as f32 / sample_rate as f32
    } else {
        0.0
    };

    let info = WavInfo {
        audio_format,
        num_channels,
        sample_rate,
        bits_per_sample,
        total_samples,
        duration_secs,
    };

    Ok((info, raw_pcm))
}

/// Decodes integer PCM data (8, 16, 24, 32 bits) and averages multiple channels to mono.
fn decode_pcm_integer(
    raw_pcm: &[u8],
    num_channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<f32>, NotesError> {
    let channels = num_channels as usize;

    match bits_per_sample {
        8 => {
            let total_samples = raw_pcm.len() / channels;
            let mut mono = Vec::with_capacity(total_samples);
            for frame in raw_pcm.chunks_exact(channels) {
                let sum: f32 = frame.iter().map(|&b| (b as f32 - 128.0) / 128.0).sum();
                mono.push(sum / channels as f32);
            }
            Ok(mono)
        }
        16 => {
            let bytes_per_frame = channels * 2;
            let total_samples = raw_pcm.len() / bytes_per_frame;
            let mut mono = Vec::with_capacity(total_samples);
            for frame in raw_pcm.chunks_exact(bytes_per_frame) {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    let offset = ch * 2;
                    let sample_i16 = i16::from_le_bytes([frame[offset], frame[offset + 1]]);
                    sum += sample_i16 as f32 / 32768.0;
                }
                mono.push(sum / channels as f32);
            }
            Ok(mono)
        }
        24 => {
            let bytes_per_frame = channels * 3;
            let total_samples = raw_pcm.len() / bytes_per_frame;
            let mut mono = Vec::with_capacity(total_samples);
            for frame in raw_pcm.chunks_exact(bytes_per_frame) {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    let offset = ch * 3;
                    let b0 = frame[offset] as i32;
                    let b1 = frame[offset + 1] as i32;
                    let b2 = frame[offset + 2] as i32;
                    let val = ((b0 | (b1 << 8) | (b2 << 16)) << 8) >> 8;
                    sum += val as f32 / 8388608.0;
                }
                mono.push(sum / channels as f32);
            }
            Ok(mono)
        }
        32 => {
            let bytes_per_frame = channels * 4;
            let total_samples = raw_pcm.len() / bytes_per_frame;
            let mut mono = Vec::with_capacity(total_samples);
            for frame in raw_pcm.chunks_exact(bytes_per_frame) {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    let offset = ch * 4;
                    let val = i32::from_le_bytes(frame[offset..offset + 4].try_into().unwrap());
                    sum += val as f32 / 2147483648.0;
                }
                mono.push(sum / channels as f32);
            }
            Ok(mono)
        }
        other => Err(NotesError::SttAudio(format!(
            "Unsupported bits per sample: {}",
            other
        ))),
    }
}

/// Decodes 32-bit IEEE float PCM data and averages multiple channels to mono.
fn decode_pcm_float(
    raw_pcm: &[u8],
    num_channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<f32>, NotesError> {
    if bits_per_sample != 32 {
        return Err(NotesError::SttAudio(format!(
            "Unsupported float PCM bit depth: {} (expected 32)",
            bits_per_sample
        )));
    }

    let channels = num_channels as usize;
    let bytes_per_frame = channels * 4;
    let total_samples = raw_pcm.len() / bytes_per_frame;
    let mut mono = Vec::with_capacity(total_samples);

    for frame in raw_pcm.chunks_exact(bytes_per_frame) {
        let mut sum = 0.0f32;
        for ch in 0..channels {
            let offset = ch * 4;
            let val = f32::from_le_bytes(frame[offset..offset + 4].try_into().unwrap());
            sum += val;
        }
        mono.push(sum / channels as f32);
    }

    Ok(mono)
}

/// Linear interpolation audio resampler.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let target_len = ((samples.len() as f64 / ratio).round()) as usize;
    let mut output = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_index = i as f64 * ratio;
        let index_floor = src_index.floor() as usize;
        let frac = (src_index - index_floor as f64) as f32;

        if index_floor + 1 < samples.len() {
            let s0 = samples[index_floor];
            let s1 = samples[index_floor + 1];
            output.push(s0 + frac * (s1 - s0));
        } else if index_floor < samples.len() {
            output.push(samples[index_floor]);
        }
    }

    output
}

/// Derives the whisper.cpp language code to force for a given model file, based on its
/// filename. Models whose id/filename ends in `-en` (e.g. `whisper-tiny-en.bin`) are
/// English-only builds and must be pinned to `"en"`; all other (multilingual) models are
/// left as `None` so whisper.cpp auto-detects the spoken language.
fn language_for_model_path(model_path: &Path) -> Option<String> {
    let stem = model_path.file_stem()?.to_str()?;
    if stem.ends_with("-en") || stem.ends_with(".en") {
        Some("en".to_string())
    } else {
        None
    }
}

/// Offline Whisper Speech-to-Text inference engine, backed by a real whisper.cpp model
/// context loaded through the `whisper_ffi` FFI bridge.
#[derive(Debug)]
pub struct WhisperEngine {
    model_path: PathBuf,
    language: Option<String>,
    #[cfg(feature = "whisper")]
    context: WhisperContext,
}

impl WhisperEngine {
    /// Loads a Whisper model from disk. Verifies the model file exists, is readable, and
    /// contains a valid GGML header before handing it off to whisper.cpp for full loading
    /// (weights, mel filters, vocabulary, tokenizer).
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self, NotesError> {
        let path = model_path.as_ref().to_path_buf();
        if !path.is_file() {
            return Err(NotesError::SttModel(format!("Model file not found at {}", path.display())));
        }

        let mut file = File::open(&path)?;
        let mut header_buf = [0u8; 48];
        let bytes_read = file.read(&mut header_buf)?;
        if bytes_read < 48 {
            return Err(NotesError::SttModel(format!(
                "Model file is invalid or truncated at {}",
                path.display()
            )));
        }

        let magic = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
        if magic != GGML_MAGIC && magic != GGMF_MAGIC && magic != GGJT_MAGIC && magic != GGUF_MAGIC {
            return Err(NotesError::SttModel(format!(
                "Invalid Whisper model format magic: 0x{:08x} at {}",
                magic,
                path.display()
            )));
        }

        let language = language_for_model_path(&path);

        #[cfg(feature = "whisper")]
        {
            let context = WhisperContext::load(&path)?;
            Ok(Self {
                model_path: path,
                language,
                context,
            })
        }

        #[cfg(not(feature = "whisper"))]
        {
            Ok(Self {
                model_path: path,
                language,
            })
        }
    }

    /// Returns the file path of the loaded Whisper model.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Transcribes a recorded WAV file into text.
    pub fn transcribe_wav_file(&self, audio_path: impl AsRef<Path>) -> Result<String, NotesError> {
        let samples = decode_wav_file(audio_path)?;
        self.transcribe_samples(&samples)
    }

    /// Transcribes normalized 16kHz mono audio samples into text using whisper.cpp.
    pub fn transcribe_samples(&self, samples: &[f32]) -> Result<String, NotesError> {
        if samples.is_empty() {
            log::warn!("WhisperEngine: samples buffer is empty");
            return Ok(String::new());
        }

        // Silence detection: skip inference on truly dead audio to avoid whisper
        // hallucinating on silence. Only guard against all-zero buffers; let whisper
        // handle quiet-but-real speech itself.
        let energy: f32 = samples.iter().map(|s| s * s).sum::<f32>() / (samples.len() as f32);
        log::info!(
            "WhisperEngine: processing {} samples ({:.2}s), energy={:.8}",
            samples.len(),
            samples.len() as f32 / 16000.0,
            energy
        );
        if samples.iter().all(|&s| s == 0.0) {
            log::info!("WhisperEngine: all-zero audio buffer; skipping inference");
            return Ok(String::new());
        }

        // Peak normalization: scale audio so maximum amplitude reaches 0.90 if it's below 0.70
        let max_abs = samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        let scale = if max_abs > 0.001 && max_abs < 0.70 {
            (0.90 / max_abs).min(15.0)
        } else {
            1.0
        };

        log::info!(
            "WhisperEngine: max_abs={:.4}, applied gain scale={:.2}",
            max_abs,
            scale
        );

        // Whisper.cpp requires sufficient audio length (at least 1.0s / 100 frames) to avoid
        // seek boundary aborts and drops the trailing ~1.0s if not padded. We pad audio with
        // at least 1.0s (16000 samples) of trailing silence, and ensure total length is at least
        // 32000 samples (2.0s).
        let mut padded: Vec<f32> = Vec::with_capacity(samples.len() + 16000);
        for &s in samples {
            padded.push(s * scale);
        }
        padded.resize(padded.len() + 16000, 0.0);
        if padded.len() < 32000 {
            padded.resize(32000, 0.0);
        }

        self.run_whisper_inference(&padded)
    }

    /// Executes real Whisper STT inference (mel -> encoder -> decoder) via whisper.cpp.
    #[cfg(feature = "whisper")]
    fn run_whisper_inference(&self, samples: &[f32]) -> Result<String, NotesError> {
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(2)
            .clamp(1, 4);

        self.context
            .transcribe(samples, self.language.as_deref(), n_threads)
    }

    /// Stub used only when the `whisper` feature is disabled (e.g. host-only builds that
    /// don't need speech-to-text). Always returns an explicit "not implemented" error
    /// rather than fabricating output.
    #[cfg(not(feature = "whisper"))]
    fn run_whisper_inference(&self, _samples: &[f32]) -> Result<String, NotesError> {
        let _ = &self.language; // only consulted by the real whisper.cpp backend
        Err(NotesError::SttModel(
            "Whisper inference backend is disabled (built without the `whisper` feature)"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Helper to generate a valid RIFF WAVE buffer with 16-bit PCM samples.
    fn create_test_wav_16bit(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::new();

        let num_channels = channels;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32) / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let data_size = (samples.len() * 2) as u32;
        let riff_size = 36 + data_size;

        // RIFF header
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");

        // "fmt " chunk
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // subchunk1_size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // audio_format = PCM
        bytes.extend_from_slice(&num_channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());

        // "data" chunk
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        bytes
    }

    /// Builds a buffer that passes `WhisperEngine::load`'s cheap magic-header sanity
    /// check (48 bytes, valid GGML magic) but is not a real, loadable Whisper model:
    /// it has none of the hyperparameters, mel filters, vocabulary, or tensor weights
    /// that whisper.cpp actually requires. Used to verify that whisper.cpp's own model
    /// loader is exercised and correctly rejects incomplete/corrupt models.
    #[cfg(feature = "whisper")]
    fn fake_header_only_model_bytes() -> Vec<u8> {
        let mut bytes = vec![0u8; 48];
        bytes[0..4].copy_from_slice(&GGML_MAGIC.to_le_bytes());
        bytes
    }

    #[test]
    fn test_parse_wav_header_mono_16khz() {
        let samples = vec![0i16, 1000, -1000, 2000, -2000];
        let wav_data = create_test_wav_16bit(16000, 1, &samples);

        let info = parse_wav_header(&wav_data).unwrap();
        assert_eq!(info.audio_format, 1);
        assert_eq!(info.num_channels, 1);
        assert_eq!(info.sample_rate, 16000);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.total_samples, 5);
        assert!((info.duration_secs - (5.0 / 16000.0)).abs() < 1e-5);
    }

    #[test]
    fn test_decode_wav_samples_mono() {
        let samples = vec![0i16, 32767, -32768];
        let wav_data = create_test_wav_16bit(16000, 1, &samples);

        let decoded = decode_wav_bytes(&wav_data).unwrap();
        assert_eq!(decoded.len(), 3);
        assert!((decoded[0] - 0.0).abs() < 1e-4);
        assert!((decoded[1] - 0.999969).abs() < 1e-4);
        assert!((decoded[2] - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn test_decode_wav_stereo_downmix() {
        let samples = vec![
            1000i16, 3000i16, // Frame 0: avg = 2000
            -2000i16, -4000i16, // Frame 1: avg = -3000
        ];
        let wav_data = create_test_wav_16bit(16000, 2, &samples);

        let decoded = decode_wav_bytes(&wav_data).unwrap();
        assert_eq!(decoded.len(), 2);
        assert!((decoded[0] - (2000.0 / 32768.0)).abs() < 1e-4);
        assert!((decoded[1] - (-3000.0 / 32768.0)).abs() < 1e-4);
    }

    #[test]
    fn test_resample_linear() {
        let input = vec![0.0f32, 1.0f32, 0.0f32];
        let resampled = resample_linear(&input, 8000, 16000);
        assert_eq!(resampled.len(), 6);
        assert!((resampled[0] - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_whisper_engine_bad_magic_fails() {
        let temp_dir = tempdir().unwrap();
        let bad_model_path = temp_dir.path().join("bad_model.bin");
        fs::write(&bad_model_path, b"not a valid ggml model").unwrap();

        let result = WhisperEngine::load(&bad_model_path);
        assert!(result.is_err());
        match result.unwrap_err() {
            NotesError::SttModel(_) => {}
            other => panic!("Expected NotesError::SttModel, got {:?}", other),
        }
    }

    // The following test only makes sense (and only compiles the real assertion) when the
    // `whisper` feature is enabled, since it is whisper.cpp's own model loader -- not our
    // cheap magic-header check -- that must reject this incomplete file.
    #[cfg(feature = "whisper")]
    #[test]
    fn test_whisper_engine_header_only_model_rejected_by_whisper_cpp() {
        let temp_dir = tempdir().unwrap();
        let model_path = temp_dir.path().join("header-only.bin");
        fs::write(&model_path, fake_header_only_model_bytes()).unwrap();

        // Passes our cheap magic check, but whisper.cpp's real model loader must still
        // reject it because there are no hyperparameters, mel filters, vocabulary, or
        // tensor weights beyond the 4-byte magic.
        let result = WhisperEngine::load(&model_path);
        assert!(
            result.is_err(),
            "a header-only file with no real model data must not load successfully"
        );
    }

    #[test]
    fn test_language_for_model_path_english_only_variants() {
        assert_eq!(
            language_for_model_path(Path::new("/models/whisper-tiny-en.bin")),
            Some("en".to_string())
        );
        assert_eq!(
            language_for_model_path(Path::new("/models/ggml-base.en.bin")),
            Some("en".to_string())
        );
    }

    #[test]
    fn test_language_for_model_path_multilingual_variants() {
        assert_eq!(
            language_for_model_path(Path::new("/models/whisper-tiny.bin")),
            None
        );
        assert_eq!(
            language_for_model_path(Path::new("/models/ggml-small.bin")),
            None
        );
    }

    #[test]
    fn test_decode_invalid_wav_fails() {
        let invalid_wav = b"RIFF....NOT_A_WAV";
        let result = decode_wav_bytes(invalid_wav);
        assert!(result.is_err());
    }

    /// Verify that the jfk.wav fixture decodes correctly and passes our
    /// silence detection — proving the audio would reach whisper inference.
    #[test]
    fn test_jfk_fixture_decodes_and_passes_silence_gate() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stt/jfk.wav");
        let samples = decode_wav_file(&fixture).expect("jfk.wav should decode");

        // ~11 seconds at 16 kHz
        assert!(samples.len() > 160_000, "too few samples: {}", samples.len());
        assert!(samples.len() < 200_000, "too many samples: {}", samples.len());

        // Must not be silent — energy well above the old 1e-8 threshold
        let energy: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
        assert!(energy > 1e-6, "energy too low ({:.2e}), silence gate would reject", energy);

        // The new all-zero check must also pass
        assert!(!samples.iter().all(|&s| s == 0.0), "all-zero check would reject");

        // Peak-normalisation sanity: max amplitude should be in audible range
        let max_abs = samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(max_abs > 0.001, "max amplitude too low: {}", max_abs);
    }

    /// Opt-in, real-model smoke test. Not run by default: it requires a genuine
    /// `ggml-*.bin` Whisper model on disk (too large to commit to the repository) and
    /// therefore is excluded from the normal `cargo test` loop.
    ///
    /// To run it locally:
    ///
    /// ```sh
    /// export WHISPER_MODEL_PATH=/path/to/ggml-tiny.en.bin
    /// cargo test -p notesplusplus-core --features whisper -- --ignored real_tiny_model
    /// ```
    ///
    /// Verifies that `WhisperEngine` performs genuine whisper.cpp inference end-to-end
    /// (load real weights -> mel -> encoder -> decoder) on the well-known `jfk.wav`
    /// fixture and returns non-fabricated text containing an expected word from the
    /// recording.
    #[cfg(feature = "whisper")]
    #[test]
    #[ignore = "requires a real ggml Whisper model at $WHISPER_MODEL_PATH"]
    fn real_tiny_model_transcribes_jfk_fixture() {
        let model_path = match std::env::var("WHISPER_MODEL_PATH") {
            Ok(path) if !path.is_empty() => path,
            _ => {
                panic!("Set WHISPER_MODEL_PATH to a real ggml-*.bin Whisper model to run this test");
            }
        };

        let engine = WhisperEngine::load(&model_path).expect("failed to load real Whisper model");

        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stt/jfk.wav");
        let text = engine
            .transcribe_wav_file(&fixture_path)
            .expect("real whisper.cpp inference failed");

        assert!(
            text.to_lowercase().contains("country"),
            "expected genuine transcription of the JFK speech, got: {:?}",
            text
        );
    }
}
