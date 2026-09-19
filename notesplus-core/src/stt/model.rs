use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadata and state for a Speech-to-Text (STT) model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SttModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub is_multilingual: bool,
    pub url: String,
    /// Hash digest for integrity verification. Algorithm is determined by length:
    /// 40 chars = SHA1 (legacy), 64 chars = SHA256 (preferred).
    #[serde(alias = "sha1", alias = "sha256")]
    pub checksum: String,
    pub is_installed: bool,
    pub is_active: bool,
}

impl SttModelInfo {
    /// Returns the standard filename for this model on disk (e.g., `whisper-tiny.bin`).
    pub fn filename(&self) -> String {
        model_filename(&self.id)
    }

    /// Returns the full path where the model binary is expected to reside in `models_dir`.
    pub fn file_path(&self, models_dir: &Path) -> PathBuf {
        model_file_path(models_dir, &self.id)
    }

    /// Checks if this model binary exists and is a regular file in `models_dir`.
    pub fn check_installed(&self, models_dir: &Path) -> bool {
        self.file_path(models_dir).is_file()
    }
}

/// Download progress payload passed to progress callbacks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

/// Helper function to determine the file name for a model ID.
pub fn model_filename(model_id: &str) -> String {
    format!("{}.bin", model_id)
}

/// Helper function to determine the full file path for a model ID in the given directory.
pub fn model_file_path(models_dir: &Path, model_id: &str) -> PathBuf {
    models_dir.join(model_filename(model_id))
}

/// Returns the default catalog of supported Whisper models.
pub fn default_model_catalog() -> Vec<SttModelInfo> {
    vec![
        SttModelInfo {
            id: "whisper-tiny".to_string(),
            name: "Whisper Tiny".to_string(),
            description: "Fast multilingual speech recognition model (~75 MB)".to_string(),
            size_bytes: 77_691_713,
            is_multilingual: true,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".to_string(),
            checksum: "bd577a113a864445d4c299885e0cb97d4ba92b5f".to_string(),
            is_installed: false,
            is_active: false,
        },
        SttModelInfo {
            id: "whisper-tiny-en".to_string(),
            name: "Whisper Tiny (English)".to_string(),
            description: "Fast English-only speech recognition model (~75 MB)".to_string(),
            size_bytes: 77_704_715,
            is_multilingual: false,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin".to_string(),
            checksum: "c78c86eb1a8faa21b369bcd33207cc90d64ae9df".to_string(),
            is_installed: false,
            is_active: false,
        },
        SttModelInfo {
            id: "whisper-base".to_string(),
            name: "Whisper Base".to_string(),
            description: "Balanced multilingual speech recognition model (~142 MB)".to_string(),
            size_bytes: 147_951_465,
            is_multilingual: true,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".to_string(),
            checksum: "465707469ff3a37a2b9b8d8f89f2f99de7299dac".to_string(),
            is_installed: false,
            is_active: false,
        },
        SttModelInfo {
            id: "whisper-base-en".to_string(),
            name: "Whisper Base (English)".to_string(),
            description: "Balanced English-only speech recognition model (~142 MB)".to_string(),
            size_bytes: 147_964_211,
            is_multilingual: false,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".to_string(),
            checksum: "137c40403d78fd54d454da0f9bd998f78703390c".to_string(),
            is_installed: false,
            is_active: false,
        },
        SttModelInfo {
            id: "whisper-small".to_string(),
            name: "Whisper Small".to_string(),
            description: "High accuracy multilingual speech recognition model (~466 MB)".to_string(),
            size_bytes: 487_601_967,
            is_multilingual: true,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".to_string(),
            checksum: "55356645c2b361a969dfd0ef2c5a50d530afd8d5".to_string(),
            is_installed: false,
            is_active: false,
        },
        SttModelInfo {
            id: "whisper-small-en".to_string(),
            name: "Whisper Small (English)".to_string(),
            description: "High accuracy English-only speech recognition model (~466 MB)".to_string(),
            size_bytes: 487_601_967,
            is_multilingual: false,
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin".to_string(),
            checksum: "db8a495a91d927739e50b3fc1cc4c6b8f6c2d022".to_string(),
            is_installed: false,
            is_active: false,
        },
    ]
}

/// Finds a model in the default catalog by its ID.
pub fn find_model_by_id(id: &str) -> Option<SttModelInfo> {
    default_model_catalog().into_iter().find(|m| m.id == id)
}

/// Retrieves the full catalog with current `is_installed` and `is_active` states populated.
pub fn get_model_catalog(models_dir: &Path, active_model_id: Option<&str>) -> Vec<SttModelInfo> {
    let mut catalog = default_model_catalog();
    for model in &mut catalog {
        model.is_installed = model.check_installed(models_dir);
        model.is_active = active_model_id.map_or(false, |active_id| active_id == model.id);
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_catalog_not_empty_and_valid() {
        let catalog = default_model_catalog();
        assert_eq!(catalog.len(), 6);

        for model in &catalog {
            assert!(!model.id.is_empty());
            assert!(!model.name.is_empty());
            assert!(!model.description.is_empty());
            assert!(model.size_bytes > 0);
            assert!(model.url.starts_with("https://"));
            assert!(model.checksum.len() == 40 || model.checksum.len() == 64);
            assert!(!model.is_installed);
            assert!(!model.is_active);
            assert_eq!(model.filename(), format!("{}.bin", model.id));
        }
    }

    #[test]
    fn test_find_model_by_id() {
        assert!(find_model_by_id("whisper-tiny").is_some());
        assert!(find_model_by_id("whisper-tiny-en").is_some());
        assert!(find_model_by_id("whisper-base").is_some());
        assert!(find_model_by_id("whisper-non-existent").is_none());
    }

    #[test]
    fn test_get_model_catalog_installed_and_active() {
        let temp_dir = tempdir().unwrap();
        let models_dir = temp_dir.path();

        // Initially no models installed
        let catalog = get_model_catalog(models_dir, Some("whisper-tiny"));
        let tiny = catalog.iter().find(|m| m.id == "whisper-tiny").unwrap();
        assert!(!tiny.is_installed);
        assert!(tiny.is_active);

        let base = catalog.iter().find(|m| m.id == "whisper-base").unwrap();
        assert!(!base.is_installed);
        assert!(!base.is_active);

        // Create a dummy model file for whisper-tiny
        let tiny_path = model_file_path(models_dir, "whisper-tiny");
        fs::write(&tiny_path, b"dummy model").unwrap();

        let updated_catalog = get_model_catalog(models_dir, Some("whisper-tiny"));
        let updated_tiny = updated_catalog.iter().find(|m| m.id == "whisper-tiny").unwrap();
        assert!(updated_tiny.is_installed);
        assert!(updated_tiny.is_active);
    }
}
