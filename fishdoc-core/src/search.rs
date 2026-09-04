use rusqlite::Connection;

use crate::db;
use crate::page::PageInfo;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub page: PageInfo,
    pub snippet: String,
}

/// Index a page's content into FTS5.
pub fn index_page(conn: &Connection, page_id: i64, content: &str) -> Result<(), String> {
    db::update_fts_content(conn, page_id, content).map_err(|e| e.to_string())
}

/// Search pages via FTS5. Returns matching pages with snippets.
pub fn search_pages(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.filename, p.title, p.is_journal, p.created_at, p.updated_at, p.block_count,
                    snippet(pages_fts, 2, '<b>', '</b>', '...', 32) as snip
             FROM pages_fts
             JOIN pages p ON p.id = pages_fts.rowid
             WHERE pages_fts MATCH ?1
             ORDER BY rank
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map(rusqlite::params![query], |row| {
            Ok(SearchResult {
                page: PageInfo {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    title: row.get(2)?,
                    is_journal: row.get::<_, i32>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    block_count: row.get(6)?,
                },
                snippet: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(results)
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
        (conn, dir)
    }

    fn create_test_page(conn: &Connection, name: &str) -> i64 {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
             VALUES (?1, ?2, 0, ?3, ?3, 0)",
            rusqlite::params![format!("{}.adoc", name), name, now],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn index_and_search() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        index_page(&conn, page_id, "hello world content").unwrap();

        let results = search_pages(&conn, "hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "Test");
    }

    #[test]
    fn search_returns_snippet() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        index_page(&conn, page_id, "the quick brown fox jumps").unwrap();

        let results = search_pages(&conn, "brown").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("brown"));
    }

    #[test]
    fn search_empty_query() {
        let (conn, _dir) = setup();
        let results = search_pages(&conn, "").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_results() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        index_page(&conn, page_id, "hello world").unwrap();

        let results = search_pages(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_special_characters() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        index_page(&conn, page_id, "C++ programming").unwrap();

        // Should not panic
        let _results = search_pages(&conn, "C++");
    }
}
