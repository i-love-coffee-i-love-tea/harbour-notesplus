use std::path::Path;
use rusqlite::{Connection, Result as SqlResult};

/// Opens a SQLite database connection, configures WAL mode and performance PRAGMAs, and initializes schema.
pub fn open_db(path: impl AsRef<Path>) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    init_schema(&conn)?;
    Ok(conn)
}

/// Initialize the SQLite schema and perform migrations if necessary. Idempotent.
pub fn init_schema(conn: &Connection) -> SqlResult<()> {
    // Performance PRAGMAs — log failures but don't block startup
    if let Err(e) = conn.pragma_update(None, "journal_mode", "WAL") {
        log::warn!("Failed to set journal_mode=WAL: {}", e);
    }
    if let Err(e) = conn.pragma_update(None, "synchronous", "NORMAL") {
        log::warn!("Failed to set synchronous=NORMAL: {}", e);
    }
    if let Err(e) = conn.pragma_update(None, "cache_size", -8000) {
        log::warn!("Failed to set cache_size: {}", e);
    }
    if let Err(e) = conn.pragma_update(None, "busy_timeout", crate::constants::DB_BUSY_TIMEOUT_MS) {
        log::warn!("Failed to set busy_timeout: {}", e);
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY,
            value INTEGER NOT NULL
        );
        ",
    )?;

    migrate_schema(conn)?;

    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
            filename, title, content,
            tokenize='porter unicode61'
        );

        CREATE TRIGGER IF NOT EXISTS pages_ai AFTER INSERT ON pages BEGIN
            INSERT INTO pages_fts(rowid, filename, title, content)
            VALUES (new.id, CASE WHEN new.group_path = '' THEN new.filename ELSE new.group_path || '/' || new.filename END, new.title, '');
        END;

        CREATE TRIGGER IF NOT EXISTS pages_ad AFTER DELETE ON pages BEGIN
            DELETE FROM pages_fts WHERE rowid = old.id;
        END;

        CREATE TRIGGER IF NOT EXISTS pages_au AFTER UPDATE OF filename, group_path, title ON pages BEGIN
            UPDATE pages_fts
            SET filename = CASE WHEN new.group_path = '' THEN new.filename ELSE new.group_path || '/' || new.filename END,
                title = new.title
            WHERE rowid = old.id;
        END;
        ",
    )?;

    Ok(())
}

/// Migrates schema to the latest version.
pub fn migrate_schema(conn: &Connection) -> SqlResult<()> {
    let current_version: i64 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 2 {
        // Check if legacy pages table exists
        let pages_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pages'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if pages_exists {
            // Check if group_path already exists
            let mut stmt = conn.prepare("PRAGMA table_info(pages)")?;
            let has_group_path = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .any(|col| col.map(|c| c == "group_path").unwrap_or(false));

            if !has_group_path {
                conn.execute_batch(
                    "
                    CREATE TABLE pages_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        filename TEXT NOT NULL,
                        group_path TEXT NOT NULL DEFAULT '',
                        title TEXT NOT NULL,
                        is_journal INTEGER NOT NULL DEFAULT 0,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        block_count INTEGER NOT NULL DEFAULT 0,
                        color TEXT NOT NULL DEFAULT '',
                        UNIQUE(group_path, filename)
                    );

                    INSERT INTO pages_new (id, filename, group_path, title, is_journal, created_at, updated_at, block_count, color)
                        SELECT id, filename, '', title, is_journal, created_at, updated_at, block_count, '' FROM pages;

                    DROP TABLE pages;
                    ALTER TABLE pages_new RENAME TO pages;
                    ",
                )?;
            }
        } else {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS pages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    filename TEXT NOT NULL,
                    group_path TEXT NOT NULL DEFAULT '',
                    title TEXT NOT NULL,
                    is_journal INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    block_count INTEGER NOT NULL DEFAULT 0,
                    color TEXT NOT NULL DEFAULT '',
                    UNIQUE(group_path, filename)
                );
                ",
            )?;
        }

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                collapsed INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0,
                note_sort TEXT NOT NULL DEFAULT 'newest',
                created_at TEXT NOT NULL
            );

            INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('version', 2);
            ",
        )?;
    }

    if current_version < 3 {
        let groups_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='groups'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if groups_exists {
            let mut stmt = conn.prepare("PRAGMA table_info(groups)")?;
            let has_note_sort = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .any(|col| col.map(|c| c == "note_sort").unwrap_or(false));

            if !has_note_sort {
                conn.execute_batch("ALTER TABLE groups ADD COLUMN note_sort TEXT NOT NULL DEFAULT 'newest';")?;
            }
        } else {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS groups (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    path TEXT NOT NULL UNIQUE,
                    display_name TEXT NOT NULL,
                    collapsed INTEGER NOT NULL DEFAULT 0,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    note_sort TEXT NOT NULL DEFAULT 'newest',
                    created_at TEXT NOT NULL
                );
                ",
            )?;
        }

        conn.execute_batch(
            "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('version', 3);",
        )?;
    }

    if current_version < 4 {
        let pages_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='pages'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if pages_exists {
            let mut stmt = conn.prepare("PRAGMA table_info(pages)")?;
            let has_color = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .any(|col| col.map(|c| c == "color").unwrap_or(false));

            if !has_color {
                conn.execute_batch("ALTER TABLE pages ADD COLUMN color TEXT NOT NULL DEFAULT '';")?;
            }
        }

        conn.execute_batch(
            "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('version', 4);",
        )?;
    }

    Ok(())
}

/// Index a page into FTS. Replaces any existing entry.
pub fn update_fts_content(conn: &Connection, page_id: i64, content: &str) -> SqlResult<()> {
    // Update the timestamp
    conn.execute(
        "UPDATE pages SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![chrono::Utc::now().to_rfc3339(), page_id],
    )?;

    // Get filename, group_path, and title
    let (filename, group_path, title): (String, String, String) = conn.query_row(
        "SELECT filename, group_path, title FROM pages WHERE id = ?1",
        rusqlite::params![page_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    let full_path = if group_path.is_empty() {
        filename
    } else {
        format!("{}/{}", group_path, filename)
    };

    // Delete old FTS entry (match by rowid)
    conn.execute(
        "DELETE FROM pages_fts WHERE rowid = ?1",
        rusqlite::params![page_id],
    )?;

    // Insert new FTS entry
    conn.execute(
        "INSERT INTO pages_fts(rowid, filename, title, content) VALUES(?1, ?2, ?3, ?4)",
        rusqlite::params![page_id, full_path, title, content],
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

    #[test]
    fn test_schema_migration_v1_to_v2_preserves_notes() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v1.db");
        let conn = Connection::open(&db_path).unwrap();

        // Create legacy v1 schema manually
        conn.execute_batch(
            "
            CREATE TABLE pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                is_journal INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                block_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE VIRTUAL TABLE pages_fts USING fts5(
                filename, title, content,
                tokenize='porter unicode61'
            );
            ",
        ).unwrap();

        // Insert legacy page
        conn.execute(
            "INSERT INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
             VALUES ('legacy.adoc', 'Legacy Note', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0)",
            [],
        ).unwrap();

        // Run schema initialization / migration
        init_schema(&conn).unwrap();

        // Check group_path is empty string and record survived
        let (filename, group_path, title): (String, String, String) = conn.query_row(
            "SELECT filename, group_path, title FROM pages WHERE filename = 'legacy.adoc'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(filename, "legacy.adoc");
        assert_eq!(group_path, "");
        assert_eq!(title, "Legacy Note");

        // Check groups table exists
        let groups_table_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='groups'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(groups_table_count, 1);

        // Check schema version in schema_meta
        let version: i64 = conn.query_row(
            "SELECT value FROM schema_meta WHERE key='version'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 4);

        // Check note_sort column exists
        let mut stmt = conn.prepare("PRAGMA table_info(groups)").unwrap();
        let has_note_sort = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|col| col.map(|c| c == "note_sort").unwrap_or(false));
        assert!(has_note_sort);

        // Check color column exists on pages
        let mut pages_stmt = conn.prepare("PRAGMA table_info(pages)").unwrap();
        let has_color = pages_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|col| col.map(|c| c == "color").unwrap_or(false));
        assert!(has_color);
    }

    #[test]
    fn test_schema_migration_v2_to_v3_adds_note_sort() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v2.db");
        let conn = Connection::open(&db_path).unwrap();

        // Create v2 schema manually
        conn.execute_batch(
            "
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY,
                value INTEGER NOT NULL
            );
            INSERT INTO schema_meta (key, value) VALUES ('version', 2);
            CREATE TABLE pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                group_path TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL,
                is_journal INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                block_count INTEGER NOT NULL DEFAULT 0,
                UNIQUE(group_path, filename)
            );
            CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                collapsed INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            INSERT INTO groups (path, display_name, collapsed, sort_order, created_at)
            VALUES ('Projects', 'Projects', 0, 0, '2026-01-01T00:00:00Z');
            ",
        ).unwrap();

        // Run schema initialization / migration
        init_schema(&conn).unwrap();

        // Check schema version
        let version: i64 = conn.query_row(
            "SELECT value FROM schema_meta WHERE key='version'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 4);

        // Check note_sort column added with default value 'newest'
        let note_sort: String = conn.query_row(
            "SELECT note_sort FROM groups WHERE path = 'Projects'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(note_sort, "newest");

        // Check color column added to pages
        let mut pages_stmt = conn.prepare("PRAGMA table_info(pages)").unwrap();
        let has_color = pages_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|col| col.map(|c| c == "color").unwrap_or(false));
        assert!(has_color);
    }

    #[test]
    fn test_schema_migration_v3_to_v4_adds_color() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("v3.db");
        let conn = Connection::open(&db_path).unwrap();

        conn.execute_batch(
            "
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY,
                value INTEGER NOT NULL
            );
            INSERT INTO schema_meta (key, value) VALUES ('version', 3);
            CREATE TABLE pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                group_path TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL,
                is_journal INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                block_count INTEGER NOT NULL DEFAULT 0,
                UNIQUE(group_path, filename)
            );
            CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                collapsed INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0,
                note_sort TEXT NOT NULL DEFAULT 'newest',
                created_at TEXT NOT NULL
            );
            INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
            VALUES ('note.adoc', '', 'My Note', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1);
            ",
        ).unwrap();

        init_schema(&conn).unwrap();

        let version: i64 = conn.query_row(
            "SELECT value FROM schema_meta WHERE key='version'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(version, 4);

        let color: String = conn.query_row(
            "SELECT color FROM pages WHERE filename = 'note.adoc'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(color, "");
    }

    #[test]
    fn test_composite_unique_constraint_allows_duplicate_filenames_in_different_groups() {
        let (conn, _dir) = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        // Inserting same filename in different groups must succeed
        conn.execute(
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
             VALUES ('Todo.adoc', 'Work', 'Work Todo', 0, ?1, ?1, 0)",
            rusqlite::params![now],
        ).unwrap();

        conn.execute(
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
             VALUES ('Todo.adoc', 'Personal', 'Personal Todo', 0, ?1, ?1, 0)",
            rusqlite::params![now],
        ).unwrap();

        // Inserting same filename in the SAME group must fail due to UNIQUE(group_path, filename)
        let duplicate_res = conn.execute(
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
             VALUES ('Todo.adoc', 'Work', 'Work Todo Dup', 0, ?1, ?1, 0)",
            rusqlite::params![now],
        );
        assert!(duplicate_res.is_err(), "Expected unique constraint error for duplicate (group_path, filename)");
    }

    #[test]
    fn test_fts_trigger_on_delete_removes_fts_entry() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "note.adoc", "My Note");
        update_fts_content(&conn, page_id, "searchable trigger content").unwrap();

        // Verify FTS indexed it
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'searchable'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Directly delete from pages table
        conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![page_id]).unwrap();

        // Trigger should have deleted the FTS entry automatically
        let count_after: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'searchable'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 0);
    }

    #[test]
    fn test_fts_trigger_on_update_syncs_metadata() {
        let (conn, _dir) = test_db();
        let page_id = insert_test_page(&conn, "spec.adoc", "Old Title");
        update_fts_content(&conn, page_id, "some documentation").unwrap();

        // Update title and group_path on pages table
        conn.execute(
            "UPDATE pages SET title = 'Updated Title', group_path = 'Docs' WHERE id = ?1",
            rusqlite::params![page_id],
        ).unwrap();

        // Verify FTS entry has updated filename and title
        let (filename, title): (String, String) = conn
            .query_row(
                "SELECT filename, title FROM pages_fts WHERE rowid = ?1",
                rusqlite::params![page_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(filename, "Docs/spec.adoc");
        assert_eq!(title, "Updated Title");
    }
}
