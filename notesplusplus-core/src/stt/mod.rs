pub mod downloader;
pub mod engine;
pub mod model;
#[cfg(feature = "whisper")]
pub mod whisper_ffi;

pub use downloader::{hex_encode, ModelDownloader, SttError};
pub use engine::{decode_wav_bytes, decode_wav_file, parse_wav_header, WavInfo, WhisperEngine};
pub use model::{
    default_model_catalog, find_model_by_id, get_model_catalog, model_file_path, model_filename,
    DownloadProgress, SttModelInfo,
};
