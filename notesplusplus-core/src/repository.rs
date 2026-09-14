//! Note repository abstraction for decoupled storage, retrieval, and indexing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

use crate::CoreError;
use crate::group::{self, GroupInfo, NoteSortOrder};
use crate::page::{self, PageInfo};
use crate::search::{self, SearchResult};

/// Common trait for note storage and index operations.
pub trait NoteRepository: Send + Sync {
    /// List all pages from the index.
    fn list_pages(&self) -> Result<Vec<PageInfo>, CoreError>;

    /// Search indexed pages via full-text search.
    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, CoreError>;

    /// Retrieve page metadata by title or filename.
    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, CoreError>;

    /// Read raw note content from disk.
    fn read_note_content(&self, filename: &str) -> Result<String, CoreError>;

    /// Save note content atomically and update database metadata and FTS index in O(1) time.
    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, CoreError>;

    /// Create a new note page and register it in the index.
    fn create_page(&self, name: &str, is_journal: bool) -> Result<PageInfo, CoreError>;

    /// Delete a note page from disk and the SQLite index.
    fn delete_page(&self, name_or_filename: &str) -> Result<(), CoreError>;

    /// Perform a full filesystem reconciliation sync.
    fn sync_all(&self) -> Result<(), CoreError>;

    /// Create a new note group.
    fn create_group(&self, parent_path: &str, name: &str) -> Result<GroupInfo, CoreError>;

    /// Rename an existing group.
    fn rename_group(&self, old_path: &str, new_name: &str) -> Result<String, CoreError>;

    /// Delete a note group.
    fn delete_group(&self, path: &str, recursive: bool) -> Result<(), CoreError>;

    /// List groups, optionally filtered.
    fn list_groups(&self, parent_path: Option<&str>, max_depth: Option<i32>) -> Result<Vec<GroupInfo>, CoreError>;

    /// Set note sort order for a group.
    fn set_group_note_sort(&self, path: &str, note_sort: NoteSortOrder) -> Result<NoteSortOrder, CoreError>;

    /// Move a page to a new group.
    fn move_page(&self, source_name_or_path: &str, target_group: &str) -> Result<PageInfo, CoreError>;
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
    fn list_pages(&self) -> Result<Vec<PageInfo>, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::list_pages(&conn)
    }

    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        search::search_pages(&conn, query)
    }

    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::get_page(&conn, name_or_filename)
    }

    fn read_note_content(&self, filename: &str) -> Result<String, CoreError> {
        let path = page::safe_note_path(&self.notes_dir, filename);
        std::fs::read_to_string(&path).map_err(|e| CoreError::Msg(format!("Failed to read {}: {}", filename, e)))
    }

    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::save_and_index_page(&conn, &self.notes_dir, filename, content)
    }

    fn create_page(&self, name: &str, is_journal: bool) -> Result<PageInfo, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::create_page(&conn, &self.notes_dir, name, is_journal)
    }

    fn delete_page(&self, name_or_filename: &str) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::delete_page(&conn, &self.notes_dir, name_or_filename)
    }

    fn sync_all(&self) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::sync_and_index_pages(&conn, &self.notes_dir)
    }

    fn create_group(&self, parent_path: &str, name: &str) -> Result<GroupInfo, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::create_group(&conn, &self.notes_dir, parent_path, name)
    }

    fn rename_group(&self, old_path: &str, new_name: &str) -> Result<String, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::rename_group(&conn, &self.notes_dir, old_path, new_name)
    }

    fn delete_group(&self, path: &str, recursive: bool) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::delete_group(&conn, &self.notes_dir, path, recursive)
    }

    fn list_groups(&self, parent_path: Option<&str>, max_depth: Option<i32>) -> Result<Vec<GroupInfo>, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::list_groups(&conn, parent_path, max_depth)
    }

    fn set_group_note_sort(&self, path: &str, note_sort: NoteSortOrder) -> Result<NoteSortOrder, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::set_group_note_sort(&conn, path, note_sort)
    }

    fn move_page(&self, source_name_or_path: &str, target_group: &str) -> Result<PageInfo, CoreError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::move_page(&conn, &self.notes_dir, source_name_or_path, target_group)
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

    #[test]
    fn test_repo_group_and_move_page() {
        let (repo, _dir) = setup_repo();

        // Create group
        let g = repo.create_group("", "Projects").unwrap();
        assert_eq!(g.path, "Projects");

        // Create page
        let page = repo.create_page("Projects/Task.adoc", false).unwrap();
        assert_eq!(page.group_path, "Projects");

        // Move page to Archives
        let moved = repo.move_page(&page.full_path(), "Archives").unwrap();
        assert_eq!(moved.group_path, "Archives");
        assert_eq!(moved.full_path(), "Archives/Task.adoc");

        let groups = repo.list_groups(None, None).unwrap();
        assert!(groups.iter().any(|g| g.path == "Archives"));

        // Sort setting
        let sort = repo.set_group_note_sort("Archives", NoteSortOrder::ByName).unwrap();
        assert_eq!(sort, NoteSortOrder::ByName);

        let groups_after = repo.list_groups(None, None).unwrap();
        let archives = groups_after.iter().find(|g| g.path == "Archives").unwrap();
        assert_eq!(archives.note_sort, NoteSortOrder::ByName);
    }
}
