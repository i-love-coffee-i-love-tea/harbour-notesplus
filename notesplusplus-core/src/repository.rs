//! Note repository abstraction for decoupled storage, retrieval, and indexing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

use crate::page::{self, PageInfo};
use crate::search::{self, SearchResult};

/// Common trait for note storage and index operations.
pub trait NoteRepository: Send + Sync {
    /// List all pages from the index.
    fn list_pages(&self) -> Result<Vec<PageInfo>, String>;

    /// Search indexed pages via full-text search.
    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, String>;

    /// Retrieve page metadata by title or filename.
    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, String>;

    /// Read raw note content from disk.
    fn read_note_content(&self, filename: &str) -> Result<String, String>;

    /// Save note content atomically and update database metadata and FTS index in $O(1)$ time.
    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, String>;

    /// Create a new note page and register it in the index.
    fn create_page(&self, name: &str, is_journal: bool) -> Result<PageInfo, String>;

    /// Delete a note page from disk and the SQLite index.
    fn delete_page(&self, name_or_filename: &str) -> Result<(), String>;

    /// Perform a full filesystem reconciliation sync.
    fn sync_all(&self) -> Result<(), String>;
}

/// Filesystem and SQLite backed NoteRepository.
pub struct FsSqliteNoteRepository {
    notes_dir: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl FsSqliteNoteRepository {
    pub fn new(notes_dir: impl AsRef<Path>, conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            notes_dir: notes_dir.as_ref().to_path_buf(),
            conn,
        }
    }

    pub fn notes_dir(&self) -> &Path {
        &self.notes_dir
    }

    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

impl NoteRepository for FsSqliteNoteRepository {
    fn list_pages(&self) -> Result<Vec<PageInfo>, String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::list_pages(&conn)
    }

    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        search::search_pages(&conn, query)
    }

    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::get_page(&conn, name_or_filename)
    }

    fn read_note_content(&self, filename: &str) -> Result<String, String> {
        let path = page::safe_note_path(&self.notes_dir, filename);
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
    }

    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::save_and_index_page(&conn, &self.notes_dir, filename, content)
    }

    fn create_page(&self, name: &str, is_journal: bool) -> Result<PageInfo, String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::create_page(&conn, &self.notes_dir, name, is_journal)
    }

    fn delete_page(&self, name_or_filename: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::delete_page(&conn, &self.notes_dir, name_or_filename)
    }

    fn sync_all(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::sync_and_index_pages(&conn, &self.notes_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo() -> (FsSqliteNoteRepository, TempDir) {
        let dir = TempDir::new().unwrap();
        let notes_dir = dir.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let db_path = dir.path().join("test.db");
        let conn = crate::db::open_db(&db_path).unwrap();
        let repo = FsSqliteNoteRepository::new(notes_dir, Arc::new(Mutex::new(conn)));
        (repo, dir)
    }

    #[test]
    fn test_repo_crud_and_search() {
        let (repo, _dir) = setup_repo();

        // Create
        let page = repo.create_page("Meeting Notes", false).unwrap();
        assert_eq!(page.title, "Meeting Notes");

        // Save note
        let content = "= Project Plan\n\nRust and Qt notes application.";
        let saved = repo.save_note(&page.filename, content).unwrap();
        assert_eq!(saved.title, "Project Plan");

        // Read content
        let read = repo.read_note_content(&page.filename).unwrap();
        assert_eq!(read, content);

        // Search
        let hits = repo.search_pages("application").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].page.filename, page.filename);

        // List
        let list = repo.list_pages().unwrap();
        assert_eq!(list.len(), 1);

        // Delete
        repo.delete_page(&page.filename).unwrap();
        let list_after = repo.list_pages().unwrap();
        assert!(list_after.is_empty());
    }
}
