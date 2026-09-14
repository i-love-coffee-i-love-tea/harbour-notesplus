use std::path::{Path, PathBuf};
use crate::constants::{APP_DIR_NAME, ASSETS_DIR_NAME, DB_FILENAME, JOURNAL_FILENAME, NOTES_DIR_NAME};

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

    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir.join(ASSETS_DIR_NAME)
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

/// Formats a path with a tilde prefix (`~`) if it resides within the given home directory.
pub fn collapse_tilde_with_home(path: impl AsRef<Path>, home: Option<&str>) -> String {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();
    let path_trimmed = path_str.trim_end_matches('/');

    if let Some(home_dir) = home {
        let home_trimmed = home_dir.trim_end_matches('/');
        if !home_trimmed.is_empty() {
            if path_trimmed == home_trimmed {
                return "~".to_string();
            }
            if let Some(rest) = path_str.strip_prefix(home_trimmed) {
                if rest.starts_with('/') {
                    let rest_trimmed = rest.trim_end_matches('/');
                    if rest_trimmed.is_empty() {
                        return "~".to_string();
                    }
                    return format!("~{}", rest);
                }
            }
        }
    } else {
        // Fallback: if home is None, check standard Linux /home/<user>
        if let Some(rest) = path_str.strip_prefix("/home/") {
            if let Some(slash_idx) = rest.find('/') {
                let suffix = &rest[slash_idx..];
                let suffix_trimmed = suffix.trim_end_matches('/');
                if suffix_trimmed.is_empty() {
                    return "~".to_string();
                }
                return format!("~{}", suffix);
            } else if !rest.is_empty() {
                return "~".to_string();
            }
        }
    }

    path_str.to_string()
}

/// Formats a path with a tilde prefix (`~`) if it resides within the user's home directory (`$HOME`).
pub fn collapse_tilde(path: impl AsRef<Path>) -> String {
    let home = std::env::var("HOME").ok();
    collapse_tilde_with_home(path, home.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapse_tilde() {
        assert_eq!(
            collapse_tilde_with_home("/home/defaultuser/.local/share/notes/1.adoc", Some("/home/defaultuser")),
            "~/.local/share/notes/1.adoc"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/defaultuser/notes.adoc", Some("/home/defaultuser")),
            "~/notes.adoc"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/defaultuser", Some("/home/defaultuser")),
            "~"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/defaultuser/", Some("/home/defaultuser/")),
            "~"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/defaultuser_other/1.adoc", Some("/home/defaultuser")),
            "/home/defaultuser_other/1.adoc"
        );
        assert_eq!(
            collapse_tilde_with_home("/tmp/notes/1.adoc", Some("/home/defaultuser")),
            "/tmp/notes/1.adoc"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/nemo/notes/1.adoc", None),
            "~/notes/1.adoc"
        );
        assert_eq!(
            collapse_tilde_with_home("/home/nemo", None),
            "~"
        );
        assert_eq!(
            collapse_tilde_with_home("", Some("/home/defaultuser")),
            ""
        );
    }

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
