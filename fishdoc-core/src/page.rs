use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db;

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub id: i64,
    pub filename: String,
    pub title: String,
    pub is_journal: bool,
    pub created_at: String,
    pub updated_at: String,
    pub block_count: i32,
}

/// Create a new page: write .adoc file + insert into SQLite.
pub fn create_page(conn: &Connection, notes_dir: &Path, name: &str, is_journal: bool) -> Result<PageInfo, String> {
    let filename = if is_journal {
        "journal.adoc".to_string()
    } else {
        format!("{}.adoc", sanitize_filename(name))
    };

    let path = notes_dir.join(&filename);
    if path.exists() && !is_journal {
        return Err(format!("Page '{}' already exists", name));
    }

    // Write initial content
    if !is_journal {
        let content = format!("= {}\n", name);
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
    } else {
        // Journal starts empty; journal.rs handles content
        if !path.exists() {
            std::fs::write(&path, "").map_err(|e| e.to_string())?;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
         VALUES (?1, ?2, ?3, ?4, ?4, 0)",
        rusqlite::params![filename, name, is_journal as i32, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(PageInfo {
        id,
        filename,
        title: name.to_string(),
        is_journal,
        created_at: now.clone(),
        updated_at: now,
        block_count: 0,
    })
}

/// Read the raw AsciiDoc content of a page.
pub fn read_page(notes_dir: &Path, filename: &str) -> Result<String, String> {
    let path = notes_dir.join(filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
}

/// Delete a page: remove file + delete from SQLite.
pub fn delete_page(conn: &Connection, notes_dir: &Path, filename: &str) -> Result<(), String> {
    let path = notes_dir.join(filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    conn.execute("DELETE FROM pages WHERE filename = ?1", rusqlite::params![filename])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// List all pages sorted by updated_at descending.
pub fn list_pages(conn: &Connection) -> Result<Vec<PageInfo>, String> {
    let mut stmt = conn
        .prepare("SELECT id, filename, title, is_journal, created_at, updated_at, block_count FROM pages ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let pages = stmt
        .query_map([], |row| {
            Ok(PageInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                title: row.get(2)?,
                is_journal: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                block_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(pages)
}

/// Get the N most recently updated non-journal pages.
pub fn recent_pages(conn: &Connection, limit: usize) -> Result<Vec<PageInfo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, filename, title, is_journal, created_at, updated_at, block_count
             FROM pages WHERE is_journal = 0 ORDER BY updated_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let pages = stmt
        .query_map([limit as i64], |row| {
            Ok(PageInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                title: row.get(2)?,
                is_journal: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                block_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(pages)
}

/// Get a page by filename.
pub fn get_page(conn: &Connection, filename: &str) -> Result<Option<PageInfo>, String> {
    let mut stmt = conn
        .prepare("SELECT id, filename, title, is_journal, created_at, updated_at, block_count FROM pages WHERE filename = ?1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map(rusqlite::params![filename], |row| {
            Ok(PageInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                title: row.get(2)?,
                is_journal: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                block_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    match rows.next() {
        Some(Ok(page)) => Ok(Some(page)),
        Some(Err(e)) => Err(e.to_string()),
        None => Ok(None),
    }
}

/// Copy example .adoc files to the notes directory on first run
/// and register them in the database.
pub fn copy_examples(conn: &Connection, notes_dir: &Path, examples_dir: &Path) -> Result<(), String> {
    if !examples_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(examples_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "adoc") {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let dest = notes_dir.join(&filename);
            if !dest.exists() {
                std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
            }
            let title = filename.trim_end_matches(".adoc").replace('_', " ");
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
                 VALUES (?1, ?2, 0, ?3, ?3, 0)",
                rusqlite::params![filename, title, now],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        db::init_schema(&conn).unwrap();
        let notes_dir = dir.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        (conn, dir)
    }

    #[test]
    fn create_page_writes_file() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "My Page", false).unwrap();
        assert!(notes.join("My_Page.adoc").exists());
    }

    #[test]
    fn create_page_inserts_into_db() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        let pages = list_pages(&conn).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Test");
    }

    #[test]
    fn create_journal_page() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Journal", true).unwrap();
        assert!(notes.join("journal.adoc").exists());
        let pages = list_pages(&conn).unwrap();
        assert!(pages[0].is_journal);
    }

    #[test]
    fn create_duplicate_page_errors() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        let result = create_page(&conn, &notes, "Test", false);
        assert!(result.is_err());
    }

    #[test]
    fn read_page_returns_content() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        let content = read_page(&notes, "Test.adoc").unwrap();
        assert!(content.contains("= Test"));
    }

    #[test]
    fn delete_page_removes_file() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        delete_page(&conn, &notes, "Test.adoc").unwrap();
        assert!(!notes.join("Test.adoc").exists());
    }

    #[test]
    fn delete_page_removes_from_db() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        delete_page(&conn, &notes, "Test.adoc").unwrap();
        let pages = list_pages(&conn).unwrap();
        assert!(pages.is_empty());
    }

    #[test]
    fn list_pages_sorted_by_updated() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "First", false).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_page(&conn, &notes, "Second", false).unwrap();
        let pages = list_pages(&conn).unwrap();
        assert_eq!(pages[0].title, "Second");
    }

    #[test]
    fn recent_pages_excludes_journal() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Journal", true).unwrap();
        create_page(&conn, &notes, "Note", false).unwrap();
        let recent = recent_pages(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "Note");
    }

    #[test]
    fn get_page_by_filename() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false).unwrap();
        let page = get_page(&conn, "Test.adoc").unwrap();
        assert!(page.is_some());
        assert_eq!(page.unwrap().title, "Test");
    }

    #[test]
    fn copy_examples_copies_adoc_files() {
        let (conn, dir) = setup();
        let examples = dir.path().join("examples");
        std::fs::create_dir_all(&examples).unwrap();
        std::fs::write(examples.join("test.adoc"), "= Test\n").unwrap();
        std::fs::write(examples.join("not_adoc.txt"), "skip").unwrap();

        let notes = dir.path().join("notes");

        copy_examples(&conn, &notes, &examples).unwrap();
        assert!(notes.join("test.adoc").exists());
        assert!(!notes.join("not_adoc.txt").exists());
        let pages = list_pages(&conn).unwrap();
        assert!(pages.iter().any(|p| p.filename == "test.adoc"));
    }
}
