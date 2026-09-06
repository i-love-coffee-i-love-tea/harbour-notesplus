//! Backup and snapshot manager for non-destructive note edits and instant rollback.

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: i64,
    pub filename: String,
    pub content: String,
    pub reason: String,
}

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new(backup_dir: impl AsRef<Path>) -> Self {
        let dir = backup_dir.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        Self { backup_dir: dir }
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Creates a pre-edit snapshot on disk before a note is modified.
    pub fn create_snapshot(
        &self,
        filename: &str,
        content: &str,
        reason: &str,
    ) -> std::io::Result<Snapshot> {
        let _ = fs::create_dir_all(&self.backup_dir);
        let now = Utc::now();
        let timestamp = now.timestamp();
        let nanos = now.timestamp_subsec_nanos();
        let safe_name = filename.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        let id = format!("snap_{}_{}_{}", timestamp, nanos, safe_name);

        let snapshot = Snapshot {
            id: id.clone(),
            timestamp,
            filename: filename.to_string(),
            content: content.to_string(),
            reason: reason.to_string(),
        };

        let file_path = self.backup_dir.join(format!("{}.json", id));
        let json_data = serde_json::to_string_pretty(&snapshot)
            .map_err(std::io::Error::other)?;
        fs::write(file_path, json_data)?;

        Ok(snapshot)
    }

    /// Lists all snapshots, sorted by timestamp descending (newest first).
    pub fn list_snapshots(&self, filter_filename: Option<&str>) -> std::io::Result<Vec<Snapshot>> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(snap) = serde_json::from_str::<Snapshot>(&content) {
                        if let Some(target) = filter_filename {
                            if snap.filename != target {
                                continue;
                            }
                        }
                        list.push(snap);
                    }
                }
            }
        }

        list.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
        Ok(list)
    }

    /// Retrieves a specific snapshot by its unique ID.
    pub fn get_snapshot(&self, id: &str) -> std::io::Result<Option<Snapshot>> {
        let file_path = self.backup_dir.join(format!("{}.json", id));
        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(file_path)?;
        let snap = serde_json::from_str::<Snapshot>(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(snap))
    }

    /// Retrieves the most recent snapshot overall or for a specific file.
    pub fn get_latest_snapshot(&self, filter_filename: Option<&str>) -> std::io::Result<Option<Snapshot>> {
        let list = self.list_snapshots(filter_filename)?;
        Ok(list.into_iter().next())
    }

    /// Deletes a snapshot after successful undo or manual purge.
    pub fn delete_snapshot(&self, id: &str) -> std::io::Result<bool> {
        let file_path = self.backup_dir.join(format!("{}.json", id));
        if file_path.exists() {
            fs::remove_file(file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_read_snapshot() {
        let tmp = tempdir().unwrap();
        let mgr = BackupManager::new(tmp.path());

        let snap = mgr.create_snapshot("todo.adoc", "= Todo\n* [ ] Task 1", "Updated to add task 2").unwrap();
        assert_eq!(snap.filename, "todo.adoc");
        assert_eq!(snap.content, "= Todo\n* [ ] Task 1");

        let fetched = mgr.get_snapshot(&snap.id).unwrap().expect("Must find snapshot");
        assert_eq!(fetched.id, snap.id);
        assert_eq!(fetched.content, snap.content);
        assert_eq!(fetched.reason, "Updated to add task 2");
    }

    #[test]
    fn test_list_and_filter_snapshots() {
        let tmp = tempdir().unwrap();
        let mgr = BackupManager::new(tmp.path());

        let s1 = mgr.create_snapshot("note1.adoc", "Old Note 1", "Edit 1").unwrap();
        let _s2 = mgr.create_snapshot("note2.adoc", "Old Note 2", "Edit 2").unwrap();

        let all = mgr.list_snapshots(None).unwrap();
        assert_eq!(all.len(), 2);

        let filtered = mgr.list_snapshots(Some("note1.adoc")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, s1.id);

        let latest = mgr.get_latest_snapshot(Some("note1.adoc")).unwrap().unwrap();
        assert_eq!(latest.id, s1.id);
    }
}
