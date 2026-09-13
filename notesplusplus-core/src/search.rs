use crate::CoreError;
use rusqlite::Connection;

use crate::page::PageInfo;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub page: PageInfo,
    pub snippet: String,
}

/// Search pages via FTS5 with query sanitization and fallback. Returns matching pages with snippets.
pub fn search_pages(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, CoreError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut fts_tokens = Vec::new();
    for word in trimmed.split_whitespace() {
        let clean: String = word.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
        if !clean.is_empty() {
            fts_tokens.push(format!("\"{}\"*", clean));
        }
    }

    let fts_query = if fts_tokens.is_empty() {
        format!("\"{}\"*", trimmed.replace('"', ""))
    } else {
        fts_tokens.join(" ")
    };

    let sql = "SELECT p.id, p.filename, p.group_path, p.title, p.is_journal, p.created_at, p.updated_at, p.block_count,
                      snippet(pages_fts, 2, '<b>', '</b>', '...', 32) as snip,
                      pages_fts.content
               FROM pages_fts
               JOIN pages p ON p.id = pages_fts.rowid
               WHERE pages_fts MATCH ?1
               ORDER BY rank
               LIMIT 50";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => {
            return search_pages_fallback(conn, trimmed);
        }
    };

    let results = stmt.query_map(rusqlite::params![fts_query], |row| {
        let content: String = row.get(9).unwrap_or_default();
        let mut snip: String = row.get(8).unwrap_or_default();
        let title: String = row.get(3)?;
        if snip.trim().is_empty() || snip == "..." {
            let first_line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or(&title);
            snip = first_line.chars().take(80).collect();
        }
        Ok(SearchResult {
            page: PageInfo::from_row(row)?,
            snippet: snip,
        })
    });

    match results {
        Ok(mapped) => {
            let res: Result<Vec<_>, _> = mapped.collect();
            match res {
                Ok(list) => {
                    if list.is_empty() {
                        search_pages_fallback(conn, trimmed)
                    } else {
                        Ok(list)
                    }
                }
                Err(_) => search_pages_fallback(conn, trimmed),
            }
        }
        Err(_) => search_pages_fallback(conn, trimmed),
    }
}

/// Escapes LIKE wildcards (`%`, `_`) in user input so they're treated as literals.
fn escape_like_pattern(s: &str) -> String {
    s.replace('%', "\\%").replace('_', "\\_")
}

fn search_pages_fallback(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, CoreError> {
    let escaped = escape_like_pattern(query);
    let pattern = format!("%{}%", escaped);
    let mut stmt = conn.prepare(
        "SELECT id, filename, group_path, title, is_journal, created_at, updated_at, block_count
         FROM pages
         WHERE title LIKE ?1 ESCAPE '\\' OR filename LIKE ?1 ESCAPE '\\' OR group_path LIKE ?1 ESCAPE '\\'
         ORDER BY updated_at DESC
         LIMIT 50"
    )?;

    let results = stmt.query_map(rusqlite::params![pattern], |row| {
        let title: String = row.get(3)?;
        Ok(SearchResult {
            page: PageInfo::from_row(row)?,
            snippet: title,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    ?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
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
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
             VALUES (?1, '', ?2, 0, ?3, ?3, 0)",
            rusqlite::params![format!("{}.adoc", name), name, now],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_search_pages_returns_group_path() {
        let (conn, _dir) = setup();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
             VALUES ('Sprint.adoc', 'Work/Projects', 'Sprint Plan', 0, ?1, ?1, 0)",
            rusqlite::params![now],
        ).unwrap();
        let page_id = conn.last_insert_rowid();
        db::update_fts_content(&conn, page_id, "Sprint goals and tickets").unwrap();

        let results = search_pages(&conn, "goals").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.group_path, "Work/Projects");
        assert_eq!(results[0].page.full_path(), "Work/Projects/Sprint.adoc");
    }

    #[test]
    fn index_and_search() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        db::update_fts_content(&conn, page_id, "hello world content").unwrap();

        let results = search_pages(&conn, "hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "Test");
    }

    #[test]
    fn search_returns_snippet() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        db::update_fts_content(&conn, page_id, "the quick brown fox jumps").unwrap();

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
        db::update_fts_content(&conn, page_id, "hello world").unwrap();

        let results = search_pages(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_special_characters() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "Test");
        db::update_fts_content(&conn, page_id, "C++ programming").unwrap();

        // Should not panic
        let _results = search_pages(&conn, "C++");
    }

    #[test]
    fn search_prefix_matching() {
        let (conn, _dir) = setup();
        let page_id = create_test_page(&conn, "SailfishOS Guide");
        db::update_fts_content(&conn, page_id, "Developing applications in Rust and QML").unwrap();

        let results = search_pages(&conn, "app").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "SailfishOS Guide");
    }

    #[test]
    fn search_multiple_words() {
        let (conn, _dir) = setup();
        let p1 = create_test_page(&conn, "Doc 1");
        db::update_fts_content(&conn, p1, "Quick brown fox").unwrap();
        let p2 = create_test_page(&conn, "Doc 2");
        db::update_fts_content(&conn, p2, "Lazy sleeping dog").unwrap();

        let results = search_pages(&conn, "brown fox").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "Doc 1");
    }

    #[test]
    fn search_fallback_by_title() {
        let (conn, _dir) = setup();
        let _p = create_test_page(&conn, "UniqueMeetingNotes");
        // No FTS indexing done for this page

        let results = search_pages(&conn, "UniqueMeeting").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "UniqueMeetingNotes");
    }

    #[test]
    fn search_case_insensitive_title() {
        let (conn, _dir) = setup();
        let _p = create_test_page(&conn, "ProjectRoadmap");

        let results = search_pages(&conn, "roadmap").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "ProjectRoadmap");
    }
}
