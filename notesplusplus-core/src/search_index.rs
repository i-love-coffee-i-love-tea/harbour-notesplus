//! Keeps the SQLite full-text search index synchronized with the notes on disk.
//!
//! Scans the notes directory for `.adoc` files, registers new pages in the database,
//! updates FTS content for changed files, and purges orphaned records when files are
//! deleted. Also provides a full index rebuild for recovery from corruption.

use std::path::Path;
use rusqlite::Connection;

use crate::constants::{JOURNAL_FILENAME, JOURNAL_TITLE};
use crate::NotesError;
use crate::db;
use crate::page::{extract_doc_title, sanitize_filename, is_asset_dir};

/// Scan notes directory recursively for .adoc files, insert any missing into DB and index into FTS.
/// Skips re-reading and re-indexing files that have not changed since last recorded update.
/// Purges records from DB and FTS when files no longer exist on disk.
pub fn sync_and_index_pages(conn: &Connection, notes_dir: &Path) -> Result<(), NotesError> {
    if !notes_dir.exists() {
        return Ok(());
    }

    sync_dir_recursive(conn, notes_dir, notes_dir, "", 0)?;
    cleanup_orphaned_pages(conn, notes_dir)?;
    cleanup_orphaned_groups(conn)?;

    Ok(())
}

fn sync_dir_recursive(
    conn: &Connection,
    notes_root: &Path,
    current_dir: &Path,
    current_group: &str,
    depth: usize,
) -> Result<(), NotesError> {
    if depth > 10 {
        return Ok(());
    }

    let mut existing_pages: std::collections::HashMap<String, (i64, Option<chrono::DateTime<chrono::Utc>>)> = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT id, filename, updated_at FROM pages WHERE group_path = ?1") {
        if let Ok(rows) = stmt.query_map([current_group], |row| {
            let id: i64 = row.get(0)?;
            let filename: String = row.get(1)?;
            let updated_at_str: String = row.get(2)?;
            let dt = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|d| d.with_timezone(&chrono::Utc))
                .ok();
            Ok((filename, (id, dt)))
        }) {
            for row in rows.flatten() {
                existing_pages.insert(row.0, row.1);
            }
        }
    }

    for entry in std::fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "adoc") {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let is_journal = current_group.is_empty() && filename == JOURNAL_FILENAME;

            let file_mtime = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<chrono::Utc>::from);

            match existing_pages.get(&filename) {
                Some(&(page_id, Some(db_updated_at))) => {
                    if let Some(mtime) = file_mtime {
                        if mtime > db_updated_at {
                            let content = std::fs::read_to_string(&path).unwrap_or_default();
                            let _ = db::update_fts_content(conn, page_id, &content);
                        }
                    }
                }
                Some(&(page_id, None)) => {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let _ = db::update_fts_content(conn, page_id, &content);
                }
                None => {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let title = if is_journal {
                        JOURNAL_TITLE.to_string()
                    } else {
                        extract_doc_title(&content, &filename)
                    };
                    let now = file_mtime.map(|m| m.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                    conn.execute(
                        "INSERT OR IGNORE INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)",
                        rusqlite::params![filename, current_group, title, is_journal as i32, now],
                    )?;

                    if let Ok(page_id) = conn.query_row(
                        "SELECT id FROM pages WHERE group_path = ?1 AND filename = ?2",
                        rusqlite::params![current_group, filename],
                        |row| row.get::<_, i64>(0),
                    ) {
                        let _ = db::update_fts_content(conn, page_id, &content);
                    }
                }
            }
        } else if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
            if dir_name.starts_with('.') {
                continue;
            }
            let sanitized_dir = sanitize_filename(&dir_name).trim_matches('_').to_string();
            if sanitized_dir.is_empty() {
                continue;
            }

            if is_asset_dir(&path) {
                sync_dir_recursive(conn, notes_root, &path, current_group, depth + 1)?;
            } else {
                let child_group = if current_group.is_empty() {
                    sanitized_dir.clone()
                } else {
                    format!("{}/{}", current_group, sanitized_dir)
                };

                let now = chrono::Utc::now().to_rfc3339();
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO groups (path, display_name, collapsed, sort_order, note_sort, created_at)
                     VALUES (?1, ?2, 0, 0, 'newest', ?3)",
                    rusqlite::params![child_group, dir_name, now],
                );

                sync_dir_recursive(conn, notes_root, &path, &child_group, depth + 1)?;
            }
        }
    }

    Ok(())
}

/// Statistics returned by `rebuild_index`.
pub struct RebuildStats {
    pub pages_indexed: usize,
    pub groups_found: usize,
}

/// Destroys and recreates the full-text search index, clears all page and group
/// records, and rescans the notes directory from scratch.
pub fn rebuild_index(conn: &Connection, notes_dir: &Path) -> Result<RebuildStats, NotesError> {
    conn.execute_batch("DROP TABLE IF EXISTS pages_fts")?;
    conn.execute("DELETE FROM pages", [])?;
    conn.execute("DELETE FROM groups", [])?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE pages_fts USING fts5(
            filename, title, content,
            tokenize='porter unicode61'
        )",
    )?;

    sync_and_index_pages(conn, notes_dir)?;

    let pages_count: usize = conn
        .query_row("SELECT COUNT(*) FROM pages", [], |r| r.get(0))
        .unwrap_or(0);
    let groups_count: usize = conn
        .query_row("SELECT COUNT(*) FROM groups", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(RebuildStats {
        pages_indexed: pages_count,
        groups_found: groups_count,
    })
}

/// Purges page records from DB and FTS when files no longer exist on disk.
pub fn cleanup_orphaned_pages(conn: &Connection, notes_dir: &Path) -> Result<(), NotesError> {
    let mut stmt = conn.prepare("SELECT id, filename, group_path FROM pages")?;
    let rows: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(Result::ok)
        .collect();

    for (id, filename, group_path) in rows {
        let file_path = if group_path.is_empty() {
            notes_dir.join(&filename)
        } else {
            notes_dir.join(&group_path).join(&filename)
        };

        if !file_path.exists() {
            let _ = db::delete_fts_entry(conn, id);
            let _ = conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![id]);
        }
    }

    Ok(())
}

/// Removes groups that have no pages and no child groups.
pub fn cleanup_orphaned_groups(conn: &Connection) -> Result<(), NotesError> {
    let groups: Vec<String> = conn
        .prepare("SELECT path FROM groups")?
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(Result::ok)
        .collect();

    for group_path in groups {
        let page_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM pages WHERE group_path = ?1",
                rusqlite::params![group_path],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let prefix = format!("{}/", group_path);
        let child_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM groups WHERE path LIKE ?1",
                rusqlite::params![format!("{}%", prefix)],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if page_count == 0 && child_count == 0 {
            let _ = conn.execute(
                "DELETE FROM groups WHERE path = ?1",
                rusqlite::params![group_path],
            );
        }
    }
    Ok(())
}
