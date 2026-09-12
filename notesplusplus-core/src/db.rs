use std::path::Path;
use rusqlite::{Connection, Result as SqlResult};

/// Opens a SQLite database connection, configures WAL mode and performance PRAGMAs, and initializes schema.
pub fn open_db(path: impl AsRef<Path>) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    init_schema(&conn)?;
    Ok(conn)
}

/// Initialize the SQLite schema. Idempotent.
pub fn init_schema(conn: &Connection) -> SqlResult<()> {
    // Performance PRAGMAs
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "cache_size", -8000);
    let _ = conn.pragma_update(None, "busy_timeout", 5000);

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            is_journal INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            block_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
            filename, title, content,
            tokenize='porter unicode61'
        );
        ",
    )?;
    Ok(())
}

/// Index a page into FTS. Replaces any existing entry.
pub fn update_fts_content(conn: &Connection, page_id: i64, content: &str) -> SqlResult<()> {
    // Update the timestamp
    conn.execute(
        "UPDATE pages SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![chrono::Utc::now().to_rfc3339(), page_id],
    )?;

    // Get filename and title
    let (filename, title): (String, String) = conn.query_row(
        "SELECT filename, title FROM pages WHERE id = ?1",
        rusqlite::params![page_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Delete old FTS entry (match by rowid)
    conn.execute(
        "DELETE FROM pages_fts WHERE rowid = ?1",
        rusqlite::params![page_id],
    )?;

    // Insert new FTS entry
    conn.execute(
        "INSERT INTO pages_fts(rowid, filename, title, content) VALUES(?1, ?2, ?3, ?4)",
        rusqlite::params![page_id, filename, title, content],
    )?;

    Ok(())
}

/// Delete a page's FTS entry.
pub fn delete_fts_entry(conn: &Connection, page_id: i64) -> SqlResult<()> {
    conn.execute(
        "DELETE FROM pages_fts WHERE rowid = ?1",
        rusqlite::params![page_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_db() -> (Connection, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        init_schema(&conn).unwrap();
        (conn, dir)
    }

    fn insert_test_page(conn: &Connection, filename: &str, title: &str) -> i64 {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
             VALUES (?1, ?2, 0, ?3, ?3, 0)",
            rusqlite::params![filename, title, now],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn schema_init_creates_tables() {
        let (conn, _dir) = test_db();
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pages'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn schema_init_is_idempotent() {
        let (conn, _dir) = test_db();
        init_schema(&conn).unwrap();
    }

    #[test]
    fn index_and_search() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "test.adoc", "Test Page");
        update_fts_content(&conn, page_id, "hello world content").unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'hello'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn search_by_title() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "test.adoc", "My Important Page");
        update_fts_content(&conn, page_id, "some content").unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'important'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn delete_fts_removes_entry() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "test.adoc", "Test");
        update_fts_content(&conn, page_id, "hello world").unwrap();

        delete_fts_entry(&conn, page_id).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'hello'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn update_fts_replaces_content() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "test.adoc", "Test");
        update_fts_content(&conn, page_id, "original content").unwrap();
        update_fts_content(&conn, page_id, "updated content").unwrap();

        let count_old: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'original'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_old, 0);

        let count_new: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'updated'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_new, 1);
    }
}
