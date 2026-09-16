use super::model::{model_file_path, model_filename, DownloadProgress, SttModelInfo};
use crate::error::NotesError;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Helper function to convert raw byte slice to lowercase hex string.
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Downloader responsible for fetching STT models from remote URLs, verifying checksums,
/// and storing them in the designated models directory.
#[derive(Debug, Clone)]
pub struct ModelDownloader {
    models_dir: PathBuf,
    timeout: Duration,
}

impl ModelDownloader {
    /// Creates a new `ModelDownloader` configured with a target models directory.
    pub fn new(models_dir: impl AsRef<Path>) -> Self {
        Self {
            models_dir: models_dir.as_ref().to_path_buf(),
            timeout: Duration::from_secs(120),
        }
    }

    /// Sets the network timeout for downloads.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the target models directory.
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Downloads a model using streaming chunk reads, calculates SHA256 checksum on the fly,
    /// writes to a `.part` file, verifies integrity, and atomically renames to the final model filename.
    pub fn download<F>(
        &self,
        model: &SttModelInfo,
        cancel_flag: Option<Arc<AtomicBool>>,
        mut on_progress: F,
    ) -> Result<PathBuf, NotesError>
    where
        F: FnMut(DownloadProgress),
    {
        fs::create_dir_all(&self.models_dir)?;

        let dest_path = self.models_dir.join(model.filename());
        let part_path = self.models_dir.join(format!("{}.part", model.filename()));

        // Make HTTP request
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout_read(self.timeout)
            .build();

        let response = match agent.get(&model.url).call() {
            Ok(resp) => resp,
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                return Err(NotesError::Http(format!("STT HTTP {}: {}", status, body)));
            }
            Err(ureq::Error::Transport(err)) => {
                return Err(NotesError::Http(format!("STT download: {}", err)));
            }
        };

        let total_bytes = response
            .header("Content-Length")
            .and_then(|h| h.parse::<u64>().ok())
            .unwrap_or(model.size_bytes);

        let mut reader = response.into_reader();
        let mut file = File::create(&part_path)?;
        let algorithm = if model.sha256.len() == 40 {
            &ring::digest::SHA1_FOR_LEGACY_USE_ONLY
        } else {
            &ring::digest::SHA256
        };
        let mut digest_ctx = ring::digest::Context::new(algorithm);

        let mut buffer = [0u8; 64 * 1024]; // 64 KB chunks
        let mut bytes_downloaded: u64 = 0;

        // Send initial 0% progress
        on_progress(DownloadProgress {
            model_id: model.id.clone(),
            bytes_downloaded: 0,
            total_bytes,
            percent: 0.0,
        });

        loop {
            if let Some(ref flag) = cancel_flag {
                if flag.load(Ordering::SeqCst) {
                    drop(file);
                    let _ = fs::remove_file(&part_path);
                    return Err(NotesError::SttCancelled);
                }
            }

            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if let Some(ref flag) = cancel_flag {
                        if flag.load(Ordering::SeqCst) {
                            drop(file);
                            let _ = fs::remove_file(&part_path);
                            return Err(NotesError::SttCancelled);
                        }
                    }

                    file.write_all(&buffer[..n])?;
                    digest_ctx.update(&buffer[..n]);
                    bytes_downloaded += n as u64;

                    let percent = if total_bytes > 0 {
                        ((bytes_downloaded as f64 / total_bytes as f64) * 100.0) as f32
                    } else {
                        0.0
                    };

                    on_progress(DownloadProgress {
                        model_id: model.id.clone(),
                        bytes_downloaded,
                        total_bytes,
                        percent: percent.min(100.0),
                    });
                }
                Err(e) => {
                    drop(file);
                    let _ = fs::remove_file(&part_path);
                    return Err(NotesError::Io(e));
                }
            }
        }

        file.flush()?;
        drop(file);

        // Verify SHA256
        let calculated_digest = digest_ctx.finish();
        let calculated_hex = hex_encode(calculated_digest.as_ref());

        if !calculated_hex.eq_ignore_ascii_case(&model.sha256) {
            let _ = fs::remove_file(&part_path);
            return Err(NotesError::SttChecksum {
                expected: model.sha256.clone(),
                actual: calculated_hex,
            });
        }

        // Atomically replace destination file
        fs::rename(&part_path, &dest_path)?;

        // Send 100% progress
        on_progress(DownloadProgress {
            model_id: model.id.clone(),
            bytes_downloaded,
            total_bytes: bytes_downloaded,
            percent: 100.0,
        });

        Ok(dest_path)
    }

    /// Deletes the model binary and any orphaned `.part` file from the models directory.
    pub fn delete(&self, model_id: &str) -> Result<bool, NotesError> {
        let dest_path = model_file_path(&self.models_dir, model_id);
        let part_path = self.models_dir.join(format!("{}.part", model_filename(model_id)));

        let _ = fs::remove_file(&part_path);

        if dest_path.exists() {
            fs::remove_file(&dest_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Verifies the SHA256 checksum of an already installed model file.
    pub fn verify_installed_model(&self, model: &SttModelInfo) -> Result<bool, NotesError> {
        let path = model.file_path(&self.models_dir);
        if !path.is_file() {
            return Ok(false);
        }

        let mut file = File::open(&path)?;
        let algorithm = if model.sha256.len() == 40 {
            &ring::digest::SHA1_FOR_LEGACY_USE_ONLY
        } else {
            &ring::digest::SHA256
        };
        let mut digest_ctx = ring::digest::Context::new(algorithm);
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest_ctx.update(&buffer[..n]);
        }

        let calculated_digest = digest_ctx.finish();
        let calculated_hex = hex_encode(calculated_digest.as_ref());

        Ok(calculated_hex.eq_ignore_ascii_case(&model.sha256))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;
    use std::thread;
    use tempfile::tempdir;

    fn calculate_sha256(data: &[u8]) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA256, data);
        hex_encode(digest.as_ref())
    }

    fn calculate_sha1(data: &[u8]) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, data);
        hex_encode(digest.as_ref())
    }

    #[test]
    fn test_hex_encode_and_sha256() {
        let sample = b"hello whisper stt";
        let hash256 = calculate_sha256(sample);
        assert_eq!(hash256.len(), 64);
        assert_eq!(calculate_sha256(sample), hash256);

        let hash1 = calculate_sha1(sample);
        assert_eq!(hash1.len(), 40);
        assert_eq!(calculate_sha1(sample), hash1);
    }

    #[test]
    fn test_delete_model() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let model_id = "test-model";
        let model_path = model_file_path(temp_dir.path(), model_id);
        let part_path = temp_dir.path().join(format!("{}.part", model_filename(model_id)));

        fs::write(&model_path, b"model data").unwrap();
        fs::write(&part_path, b"part data").unwrap();

        assert!(model_path.exists());
        assert!(part_path.exists());

        let deleted = downloader.delete(model_id).unwrap();
        assert!(deleted);
        assert!(!model_path.exists());
        assert!(!part_path.exists());

        let deleted_again = downloader.delete(model_id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_verify_installed_model() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let data = b"whisper test weights binary content";
        let sha256 = calculate_sha256(data);

        let model = SttModelInfo {
            id: "verify-model".to_string(),
            name: "Verify Model".to_string(),
            description: "Test".to_string(),
            size_bytes: data.len() as u64,
            is_multilingual: false,
            url: "https://example.com/model.bin".to_string(),
            sha256: sha256.clone(),
            is_installed: false,
            is_active: false,
        };

        // Initially not installed
        assert!(!downloader.verify_installed_model(&model).unwrap());

        // Write correct file
        let path = model.file_path(temp_dir.path());
        fs::write(&path, data).unwrap();
        assert!(downloader.verify_installed_model(&model).unwrap());

        // Corrupt file
        fs::write(&path, b"corrupted data").unwrap();
        assert!(!downloader.verify_installed_model(&model).unwrap());
    }

    #[test]
    fn test_download_success_with_local_server() {
        let content = b"fake whisper ggml model weights for test";
        let sha256 = calculate_sha256(content);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 1024];
                let _ = stream.read(&mut req_buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(content);
                let _ = stream.flush();
            }
        });

        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let model = SttModelInfo {
            id: "local-test-model".to_string(),
            name: "Local Test Model".to_string(),
            description: "Test".to_string(),
            size_bytes: content.len() as u64,
            is_multilingual: false,
            url: format!("http://{}/model.bin", addr),
            sha256,
            is_installed: false,
            is_active: false,
        };

        let progress_events = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = Arc::clone(&progress_events);

        let result = downloader.download(&model, None, move |p| {
            progress_clone.lock().unwrap().push(p);
        });

        server_thread.join().unwrap();

        assert!(result.is_ok());
        let saved_path = result.unwrap();
        assert!(saved_path.exists());
        assert_eq!(fs::read(&saved_path).unwrap(), content);

        let events = progress_events.lock().unwrap();
        assert!(!events.is_empty());
        assert_eq!(events.last().unwrap().percent, 100.0);
    }

    #[test]
    fn test_download_success_with_sha1_checksum() {
        let content = b"whisper model bytes for sha1 test";
        let sha1 = calculate_sha1(content);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 1024];
                let _ = stream.read(&mut req_buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(content);
                let _ = stream.flush();
            }
        });

        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let model = SttModelInfo {
            id: "sha1-model".to_string(),
            name: "SHA1 Model".to_string(),
            description: "Test".to_string(),
            size_bytes: content.len() as u64,
            is_multilingual: false,
            url: format!("http://{}/model.bin", addr),
            sha256: sha1,
            is_installed: false,
            is_active: false,
        };

        let result = downloader.download(&model, None, |_| {});
        server_thread.join().unwrap();

        assert!(result.is_ok());
        let saved_path = result.unwrap();
        assert!(saved_path.exists());
        assert_eq!(fs::read(&saved_path).unwrap(), content);
        assert!(downloader.verify_installed_model(&model).unwrap());
    }

    #[test]
    fn test_download_checksum_mismatch_fails_and_cleans_up() {
        let content = b"some data from server";
        let wrong_sha256 = "0000000000000000000000000000000000000000000000000000000000000000";

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 1024];
                let _ = stream.read(&mut req_buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(content);
                let _ = stream.flush();
            }
        });

        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let model = SttModelInfo {
            id: "mismatch-model".to_string(),
            name: "Mismatch Model".to_string(),
            description: "Test".to_string(),
            size_bytes: content.len() as u64,
            is_multilingual: false,
            url: format!("http://{}/model.bin", addr),
            sha256: wrong_sha256.to_string(),
            is_installed: false,
            is_active: false,
        };

        let result = downloader.download(&model, None, |_| {});
        server_thread.join().unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            NotesError::SttChecksum { expected, actual } => {
                assert_eq!(expected, wrong_sha256);
                assert_eq!(actual, calculate_sha256(content));
            }
            other => panic!("Unexpected error: {:?}", other),
        }

        // Final and part files must not exist
        assert!(!model.file_path(temp_dir.path()).exists());
        assert!(!temp_dir.path().join("mismatch-model.bin.part").exists());
    }

    #[test]
    fn test_download_cancellation() {
        let content = vec![0u8; 500_000];
        let sha256 = calculate_sha256(&content);
        let content_len = content.len() as u64;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_thread = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0u8; 1024];
                let _ = stream.read(&mut req_buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(&content);
                let _ = stream.flush();
            }
        });

        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path());

        let model = SttModelInfo {
            id: "cancel-model".to_string(),
            name: "Cancel Model".to_string(),
            description: "Test".to_string(),
            size_bytes: content_len,
            is_multilingual: false,
            url: format!("http://{}/model.bin", addr),
            sha256,
            is_installed: false,
            is_active: false,
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel_flag);

        let result = downloader.download(&model, Some(cancel_flag), move |_| {
            // Cancel immediately on first progress callback
            cancel_clone.store(true, Ordering::SeqCst);
        });

        server_thread.join().unwrap();

        assert!(result.is_err());
        match result.unwrap_err() {
            NotesError::SttCancelled => {}
            other => panic!("Expected Cancelled error, got {:?}", other),
        }

        // Ensure leftover files are cleaned up
        assert!(!model.file_path(temp_dir.path()).exists());
        assert!(!temp_dir.path().join("cancel-model.bin.part").exists());
    }
}
