use std::path::{Path, PathBuf};
use crate::constants::{APP_DIR_NAME, DB_FILENAME, JOURNAL_FILENAME, NOTES_DIR_NAME};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NoteFilename(pub String);

impl NoteFilename {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NoteFilename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NoteFilename {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for NoteFilename {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionToken(pub String);

impl SessionToken {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChallengeId(pub String);

impl ChallengeId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChallengeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub notes_dir: PathBuf,
    pub db_path: PathBuf,
}

impl AppPaths {
    pub fn new() -> Self {
        Self::from_data_dir(Self::default_data_dir())
    }

    pub fn default_data_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(&home).join(".local").join("share").join(APP_DIR_NAME)
    }

    pub fn from_data_dir(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        let notes_dir = data_dir.join(NOTES_DIR_NAME);
        let db_path = data_dir.join(DB_FILENAME);
        Self {
            data_dir,
            notes_dir,
            db_path,
        }
    }

    pub fn journal_path(&self) -> PathBuf {
        self.notes_dir.join(JOURNAL_FILENAME)
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    pub fn notes_subdir(&self) -> PathBuf {
        self.notes_dir.join("notes")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    pub fn stt_models_dir(&self) -> PathBuf {
        self.models_dir().join("stt")
    }
}

impl Default for AppPaths {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_paths_from_data_dir() {
        let custom = PathBuf::from("/custom/dir");
        let paths = AppPaths::from_data_dir(&custom);
        assert_eq!(paths.data_dir, custom);
        assert_eq!(paths.notes_dir, custom.join("notes"));
        assert_eq!(paths.db_path, custom.join("notesplusplus.db"));
        assert_eq!(paths.journal_path(), custom.join("notes").join("journal.adoc"));
        assert_eq!(paths.models_dir(), custom.join("models"));
        assert_eq!(paths.stt_models_dir(), custom.join("models").join("stt"));
    }
}
