//! Note repository abstraction for decoupled storage, retrieval, and indexing.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

use crate::NotesError;
use crate::group::{self, GroupInfo, NoteSortOrder};
use crate::page::{self, PageInfo};
use crate::search::{self, SearchResult};

/// Common trait for note storage and index operations.
pub trait NoteRepository: Send + Sync {
    /// List all pages from the index.
    fn list_pages(&self) -> Result<Vec<PageInfo>, NotesError>;

    /// Search indexed pages via full-text search.
    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, NotesError>;

    /// Retrieve page metadata by title or filename.
    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, NotesError>;

    /// Read raw note content from disk.
    fn read_note_content(&self, filename: &str) -> Result<String, NotesError>;

    /// Save note content atomically and update database metadata and FTS index in O(1) time.
    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, NotesError>;

    /// Create a new note page and register it in the index with an optional custom color.
    fn create_page(&self, name: &str, is_journal: bool, color: Option<&str>) -> Result<PageInfo, NotesError>;

    /// Update the note's color attribute, re-saving and updating the database.
    fn set_page_color(&self, name_or_filename: &str, color: Option<&str>) -> Result<PageInfo, NotesError>;

    /// Delete a note page from disk and the SQLite index.
    fn delete_page(&self, name_or_filename: &str) -> Result<(), NotesError>;

    /// Perform a full filesystem reconciliation sync.
    fn sync_all(&self) -> Result<(), NotesError>;

    /// Create a new note group.
    fn create_group(&self, parent_path: &str, name: &str) -> Result<GroupInfo, NotesError>;

    /// Rename an existing group.
    fn rename_group(&self, old_path: &str, new_name: &str) -> Result<String, NotesError>;

    /// Delete a note group.
    fn delete_group(&self, path: &str, recursive: bool) -> Result<(), NotesError>;

    /// List groups, optionally filtered.
    fn list_groups(&self, parent_path: Option<&str>, max_depth: Option<i32>) -> Result<Vec<GroupInfo>, NotesError>;

    /// Set note sort order for a group.
    fn set_group_note_sort(&self, path: &str, note_sort: NoteSortOrder) -> Result<NoteSortOrder, NotesError>;

    /// Move a page to a new group.
    fn move_page(&self, source_name_or_path: &str, target_group: &str) -> Result<PageInfo, NotesError>;

    /// Check whether a note file exists on disk.
    fn note_exists(&self, name_or_filename: &str) -> bool;

    /// Get the notes directory path.
    fn notes_dir(&self) -> &Path;
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

    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

impl NoteRepository for FsSqliteNoteRepository {
    fn list_pages(&self) -> Result<Vec<PageInfo>, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::list_pages(&conn)
    }

    fn search_pages(&self, query: &str) -> Result<Vec<SearchResult>, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        search::search_pages(&conn, query)
    }

    fn get_page(&self, name_or_filename: &str) -> Result<Option<PageInfo>, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::get_page(&conn, name_or_filename)
    }

    fn read_note_content(&self, filename: &str) -> Result<String, NotesError> {
        let path = page::safe_note_path(&self.notes_dir, filename);
        if path.is_file() {
            return std::fs::read_to_string(&path).map_err(|e| NotesError::Msg(format!("Failed to read {}: {}", filename, e)));
        }
        if let Ok(Some(page)) = self.get_page(filename) {
            let path = page::safe_note_path(&self.notes_dir, &page.full_path());
            if path.is_file() {
                return std::fs::read_to_string(&path).map_err(|e| NotesError::Msg(format!("Failed to read {}: {}", filename, e)));
            }
        }
        std::fs::read_to_string(&path).map_err(|e| NotesError::Msg(format!("Failed to read {}: {}", filename, e)))
    }

    fn save_note(&self, filename: &str, content: &str) -> Result<PageInfo, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let target_filename = if !filename.contains('/') {
            if let Ok(Some(page)) = page::get_page(&conn, filename) {
                page.full_path()
            } else {
                filename.to_string()
            }
        } else {
            filename.to_string()
        };
        page::save_and_index_page(&conn, &self.notes_dir, &target_filename, content)
    }

    fn create_page(&self, name: &str, is_journal: bool, color: Option<&str>) -> Result<PageInfo, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::create_page(&conn, &self.notes_dir, name, is_journal, color)
    }

    fn set_page_color(&self, name_or_filename: &str, color: Option<&str>) -> Result<PageInfo, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let target_filename = if !name_or_filename.contains('/') {
            if let Ok(Some(page)) = page::get_page(&conn, name_or_filename) {
                page.full_path()
            } else {
                name_or_filename.to_string()
            }
        } else {
            name_or_filename.to_string()
        };
        page::set_page_color(&conn, &target_filename, color)
    }

    fn delete_page(&self, name_or_filename: &str) -> Result<(), NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::delete_page(&conn, &self.notes_dir, name_or_filename)
    }

    fn sync_all(&self) -> Result<(), NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::sync_and_index_pages(&conn, &self.notes_dir)
    }

    fn create_group(&self, parent_path: &str, name: &str) -> Result<GroupInfo, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::create_group(&conn, &self.notes_dir, parent_path, name)
    }

    fn rename_group(&self, old_path: &str, new_name: &str) -> Result<String, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::rename_group(&conn, &self.notes_dir, old_path, new_name)
    }

    fn delete_group(&self, path: &str, recursive: bool) -> Result<(), NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::delete_group(&conn, &self.notes_dir, path, recursive)
    }

    fn list_groups(&self, parent_path: Option<&str>, max_depth: Option<i32>) -> Result<Vec<GroupInfo>, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::list_groups(&conn, parent_path, max_depth)
    }

    fn set_group_note_sort(&self, path: &str, note_sort: NoteSortOrder) -> Result<NoteSortOrder, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        group::set_group_note_sort(&conn, path, note_sort)
    }

    fn move_page(&self, source_name_or_path: &str, target_group: &str) -> Result<PageInfo, NotesError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        page::move_page(&conn, &self.notes_dir, source_name_or_path, target_group)
    }

    fn note_exists(&self, name_or_filename: &str) -> bool {
        let path = page::safe_note_path(&self.notes_dir, name_or_filename);
        if path.is_file() {
            return true;
        }
        if let Ok(Some(page)) = self.get_page(name_or_filename) {
            let path = page::safe_note_path(&self.notes_dir, &page.full_path());
            if path.is_file() {
                return true;
            }
        }
        false
    }

    fn notes_dir(&self) -> &Path {
        &self.notes_dir
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
        let page = repo.create_page("Meeting Notes", false, None).unwrap();
        assert_eq!(page.title, "Meeting Notes");
        assert!(!page.color.is_empty(), "Note should always be created with a color");

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
        let page = repo.create_page("Projects/Task.adoc", false, None).unwrap();
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

    #[test]
    fn test_repo_page_color() {
        let (repo, _dir) = setup_repo();

        let page = repo.create_page("Colored Note", false, Some("#3498db")).unwrap();
        assert_eq!(page.color, "#3498db");

        let fetched = repo.get_page("Colored Note").unwrap().unwrap();
        assert_eq!(fetched.color, "#3498db");

        let updated = repo.set_page_color("Colored Note", Some("#00b894")).unwrap();
        assert_eq!(updated.color, "#00b894");

        let fetched2 = repo.get_page("Colored Note").unwrap().unwrap();
        assert_eq!(fetched2.color, "#00b894");
    }
}
