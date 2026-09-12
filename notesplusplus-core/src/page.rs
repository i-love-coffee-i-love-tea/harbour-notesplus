use std::io::Write;
use std::path::Path;

use ring::rand::SecureRandom;
use rusqlite::Connection;
use serde_json::json;

use crate::constants::{JOURNAL_FILENAME, JOURNAL_TITLE};
use crate::CoreError;
use crate::db;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageInfo {
    pub id: i64,
    pub filename: String,
    pub title: String,
    pub is_journal: bool,
    pub created_at: String,
    pub updated_at: String,
    pub block_count: i32,
}

impl PageInfo {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(PageInfo {
            id: row.get(0)?,
            filename: row.get(1)?,
            title: row.get(2)?,
            is_journal: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            block_count: row.get(6)?,
        })
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "filename": self.filename,
            "title": self.title,
            "is_journal": self.is_journal,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "block_count": self.block_count,
        })
    }
}

/// Create a new page: write .adoc file + insert into SQLite.
pub fn create_page(conn: &Connection, notes_dir: &Path, name: &str, is_journal: bool) -> Result<PageInfo, CoreError> {
    let filename = if is_journal {
        JOURNAL_FILENAME.to_string()
    } else {
        format!("{}.adoc", sanitize_filename(name))
    };

    let path = notes_dir.join(&filename);
    if path.exists() && !is_journal {
        return Err(CoreError::Msg(format!("Page '{}' already exists", name)));
    }

    // Write initial content
    if !is_journal {
        let content = format!("= {}\n", name);
        atomic_write(&path, content)?;
    } else {
        // Journal starts empty; journal.rs handles content
        if !path.exists() {
            atomic_write(&path, "")?;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
         VALUES (?1, ?2, ?3, ?4, ?4, 0)",
        rusqlite::params![filename, name, is_journal as i32, now],
    )
    ?;

    let id = conn.last_insert_rowid();

    // Index initial content into FTS
    if !is_journal {
        let _ = db::update_fts_content(conn, id, &format!("= {}\n", name));
    }

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

/// Saves a page's content atomically and updates its title, updated_at, and FTS index directly in SQLite without directory scanning.
pub fn save_and_index_page(
    conn: &Connection,
    notes_dir: &Path,
    name_or_filename: &str,
    content: &str,
) -> Result<PageInfo, CoreError> {
    let filename = sanitize_note_filename(name_or_filename);
    let path = safe_note_path(notes_dir, &filename);

    // Write content atomically to disk
    atomic_write(&path, content)?;

    let is_journal = filename == JOURNAL_FILENAME;
    let title = if is_journal {
        JOURNAL_TITLE.to_string()
    } else {
        extract_doc_title(content, &filename)
    };
    let now = chrono::Utc::now().to_rfc3339();

    // Check if page already exists in DB
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM pages WHERE filename = ?1",
            rusqlite::params![filename],
            |row| row.get(0),
        )
        .ok();

    let id = if let Some(id) = existing_id {
        conn.execute(
            "UPDATE pages SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )
        ?;
        id
    } else {
        conn.execute(
            "INSERT INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
             VALUES (?1, ?2, ?3, ?4, ?4, 0)",
            rusqlite::params![filename, title, is_journal as i32, now],
        )
        ?;
        conn.last_insert_rowid()
    };

    let _ = db::update_fts_content(conn, id, content);

    let created_at: String = conn
        .query_row(
            "SELECT created_at FROM pages WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| now.clone());

    Ok(PageInfo {
        id,
        filename,
        title,
        is_journal,
        created_at,
        updated_at: now,
        block_count: 0,
    })
}

/// Slice leading text sufficient to extract `limit` preview blocks without cutting open delimited blocks.
fn slice_preview_content(content: &str, limit: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 60 {
        return content;
    }

    let min_lines = (limit * 4).max(50);
    let mut in_delim: Option<&str> = None;
    let mut structural_lines = 0;
    let mut cut_line_idx = lines.len();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(opener) = crate::parser::as_delimiter_opener(trimmed) {
            in_delim = if in_delim == Some(opener) { None } else { Some(opener) };
        }

        structural_lines += 1;
        if structural_lines >= min_lines && in_delim.is_none() {
            cut_line_idx = idx + 1;
            break;
        }
    }

    if cut_line_idx >= lines.len() {
        content
    } else {
        let mut byte_count = 0;
        for line in &lines[..cut_line_idx] {
            byte_count += line.len() + 1;
        }
        if byte_count <= content.len() {
            &content[..byte_count]
        } else {
            content
        }
    }
}

/// Get the first N non-empty preview blocks of a page with configurable options.
pub fn get_page_preview_blocks_with_options(notes_dir: &Path, filename: &str, limit: usize, drop_comments: bool) -> Vec<crate::block::Block> {
    let path = notes_dir.join(filename);
    if let Ok(content) = std::fs::read_to_string(&path) {
        let preview_slice = slice_preview_content(&content, limit);
        let blocks = crate::parser::parse_blocks_with_options(preview_slice, drop_comments);
        blocks
            .into_iter()
            .filter(|b| !matches!(b, crate::block::Block::EmptyLine))
            .take(limit)
            .collect()
    } else {
        Vec::new()
    }
}

/// Get preview blocks as JSON values with document headings populated in Table of Contents blocks.
pub fn get_page_preview_values_with_options(
    notes_dir: &Path, filename: &str, limit: usize, drop_comments: bool,
    qt_theme: Option<&crate::html::qt_html::QtThemeColors>,
    qt_options: Option<&crate::html::qt_html::QtRenderOptions>,
) -> Vec<serde_json::Value> {
    let path = notes_dir.join(filename);
    if let Ok(content) = std::fs::read_to_string(&path) {
        let has_toc = content.lines().any(|l| {
            let t = l.trim();
            t == ":toc:" || t.starts_with(":toc:")
        });

        let mut headings_vec = Vec::new();
        if has_toc {
            // Fast line scanner for headings when TOC is present
            let mut block_idx = 0;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    block_idx += 1;
                    continue;
                }
                if let Some((level, rest)) = crate::parser::blocks::headings::parse_heading(line) {
                    if (1..=5).contains(&level) {
                        let spans = crate::inline::parse_inline(rest.trim());
                        let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                        let mut h_map = serde_json::Map::new();
                        h_map.insert("level".into(), serde_json::Value::Number(level.into()));
                        h_map.insert("text".into(), serde_json::Value::String(text));
                        h_map.insert("index".into(), serde_json::Value::Number(block_idx.into()));
                        headings_vec.push(serde_json::Value::Object(h_map));
                    }
                }
                block_idx += 1;
            }
        }

        let preview_slice = slice_preview_content(&content, limit);
        let preview_blocks: Vec<_> = crate::parser::parse_blocks_with_options(preview_slice, drop_comments)
            .into_iter()
            .filter(|b| !matches!(b, crate::block::Block::EmptyLine))
            .take(limit)
            .collect();

        let mut result = Vec::new();
        for (idx, block) in preview_blocks.into_iter().enumerate() {
            let mut json = block.to_qvariant_map();
            if matches!(block, crate::block::Block::Toc { .. }) {
                if let serde_json::Value::Object(ref mut map) = json {
                    map.insert("headings".into(), serde_json::Value::Array(headings_vec.clone()));
                }
            }
            if let (Some(theme), Some(opts)) = (qt_theme, qt_options) {
                let html = crate::html::qt_html::render_qt_block(&block, idx, theme, opts);
                if let serde_json::Value::Object(ref mut map) = json {
                    map.insert("html".into(), serde_json::Value::String(html));
                }
            }
            result.push(json);
        }
        result
    } else {
        Vec::new()
    }
}

/// Get preview blocks serialized as JSON string with document headings populated in Table of Contents blocks.
pub fn get_page_preview_json_with_options(notes_dir: &Path, filename: &str, limit: usize, drop_comments: bool) -> String {
    let preview_json_vec = get_page_preview_values_with_options(notes_dir, filename, limit, drop_comments, None, None);
    serde_json::to_string(&preview_json_vec).unwrap_or_else(|_| "[]".to_string())
}

/// Read the raw AsciiDoc content of a page.
pub fn read_page(notes_dir: &Path, filename: &str) -> Result<String, CoreError> {
    let path = notes_dir.join(filename);
    std::fs::read_to_string(&path).map_err(|e| CoreError::Msg(format!("Failed to read {}: {}", filename, e)))
}

/// Delete a page: remove file + delete from SQLite and FTS.
pub fn delete_page(conn: &Connection, notes_dir: &Path, name_or_filename: &str) -> Result<(), CoreError> {
    let page_info = get_page(conn, name_or_filename)?;

    if let Some(info) = page_info {
        let _ = db::delete_fts_entry(conn, info.id);
        let path = notes_dir.join(&info.filename);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![info.id])
            ?;
    } else {
        let direct_path = notes_dir.join(name_or_filename);
        if direct_path.exists() {
            let _ = std::fs::remove_file(&direct_path);
        }
        let adoc_name = ensure_adoc_extension(name_or_filename);
        let adoc_path = notes_dir.join(&adoc_name);
        if adoc_path.exists() {
            let _ = std::fs::remove_file(&adoc_path);
        }
        let sanitized = format!("{}.adoc", sanitize_filename(name_or_filename));
        let sanitized_path = notes_dir.join(&sanitized);
        if sanitized_path.exists() {
            let _ = std::fs::remove_file(&sanitized_path);
        }
        conn.execute(
            "DELETE FROM pages WHERE filename = ?1 OR filename = ?2 OR title = ?3",
            rusqlite::params![name_or_filename, adoc_name, name_or_filename],
        )
        ?;
    }
    Ok(())
}

/// List all pages sorted by updated_at descending.
pub fn list_pages(conn: &Connection) -> Result<Vec<PageInfo>, CoreError> {
    let mut stmt = conn
        .prepare("SELECT id, filename, title, is_journal, created_at, updated_at, block_count FROM pages ORDER BY updated_at DESC")
        ?;

    let pages = stmt
        .query_map([], |row| PageInfo::from_row(row))
        ?
        .collect::<Result<Vec<_>, _>>()
        ?;

    Ok(pages)
}

/// Get the N most recently updated non-journal pages.
pub fn recent_pages(conn: &Connection, limit: usize) -> Result<Vec<PageInfo>, CoreError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, filename, title, is_journal, created_at, updated_at, block_count
             FROM pages WHERE is_journal = 0 ORDER BY updated_at DESC LIMIT ?1",
        )
        ?;

    let pages = stmt
        .query_map([limit as i64], |row| PageInfo::from_row(row))
        ?
        .collect::<Result<Vec<_>, _>>()
        ?;

    Ok(pages)
}

/// Get a page by filename or title.
pub fn get_page(conn: &Connection, name_or_filename: &str) -> Result<Option<PageInfo>, CoreError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, filename, title, is_journal, created_at, updated_at, block_count
             FROM pages
             WHERE filename = ?1 OR title = ?1 OR filename = ?2 OR filename = ?3 OR title = ?4",
        )
        ?;

    let adoc_filename = ensure_adoc_extension(name_or_filename);
    let sanitized_adoc = format!("{}.adoc", sanitize_filename(name_or_filename));
    let title_without_adoc = name_or_filename.trim_end_matches(".adoc").replace('_', " ");

    let mut rows = stmt
        .query_map(
            rusqlite::params![
                name_or_filename,
                adoc_filename,
                sanitized_adoc,
                title_without_adoc
            ],
            |row| PageInfo::from_row(row),
        )
        ?;

    match rows.next() {
        Some(Ok(page)) => Ok(Some(page)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

/// Copy example .adoc files to the notes directory on first run
/// and register them in the database.
pub fn copy_examples(conn: &Connection, notes_dir: &Path, examples_dir: &Path) -> Result<(), CoreError> {
    if !examples_dir.exists() {
        return Ok(());
    }
    copy_dir_recursive(conn, notes_dir, examples_dir, notes_dir)?;
    Ok(())
}

fn copy_dir_recursive(conn: &Connection, notes_dir: &Path, current_src_dir: &Path, current_dest_dir: &Path) -> Result<(), CoreError> {
    for entry in std::fs::read_dir(current_src_dir)? {
        let entry = entry?;
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_file() {
            let dest = current_dest_dir.join(&filename);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "adoc" {
                if !dest.exists() {
                    std::fs::copy(&path, &dest)?;
                    let title = filename.trim_end_matches(".adoc").replace('_', " ");
                    let now = chrono::Utc::now().to_rfc3339();
                    conn.execute(
                        "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
                         VALUES (?1, ?2, 0, ?3, ?3, 0)",
                        rusqlite::params![filename, title, now],
                    )?;
                    if let Ok(page_id) = conn.query_row(
                        "SELECT id FROM pages WHERE filename = ?1",
                        rusqlite::params![filename],
                        |row| row.get::<_, i64>(0),
                    ) {
                        if let Ok(content) = std::fs::read_to_string(&dest) {
                            let _ = db::update_fts_content(conn, page_id, &content);
                        }
                    }
                }
            } else if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "svg" || ext == "yml" {
                if !dest.exists() {
                    let _ = std::fs::copy(&path, &dest);
                }
                if current_dest_dir != notes_dir {
                    let root_dest = notes_dir.join(&filename);
                    if !root_dest.exists() {
                        let _ = std::fs::copy(&path, &root_dest);
                    }
                }
            }
        } else if path.is_dir() {
            let sub_dest = current_dest_dir.join(&filename);
            let _ = std::fs::create_dir_all(&sub_dest);
            let _ = copy_dir_recursive(conn, notes_dir, &path, &sub_dest);
        }
    }
    Ok(())
}

/// Scan notes directory for .adoc files, insert any missing into DB and index into FTS.
/// Skips re-reading and re-indexing files that have not changed since last recorded update.
pub fn sync_and_index_pages(conn: &Connection, notes_dir: &Path) -> Result<(), CoreError> {
    if !notes_dir.exists() {
        return Ok(());
    }

    let mut existing_pages: std::collections::HashMap<String, (i64, Option<chrono::DateTime<chrono::Utc>>)> = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT id, filename, updated_at FROM pages") {
        if let Ok(rows) = stmt.query_map([], |row| {
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

    for entry in std::fs::read_dir(notes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "adoc") {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let is_journal = filename == JOURNAL_FILENAME;

            let file_mtime = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<chrono::Utc>::from);

            match existing_pages.get(&filename) {
                Some(&(page_id, Some(db_updated_at))) => {
                    // Only re-index if file was modified after db_updated_at
                    if let Some(mtime) = file_mtime {
                        if mtime > db_updated_at {
                            let content = std::fs::read_to_string(&path).unwrap_or_default();
                            let _ = db::update_fts_content(conn, page_id, &content);
                        }
                    }
                }
                Some(&(page_id, None)) => {
                    // Unparseable timestamp, re-index once
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let _ = db::update_fts_content(conn, page_id, &content);
                }
                None => {
                    // New page file not yet in database
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let title = if is_journal {
                        JOURNAL_TITLE.to_string()
                    } else {
                        extract_doc_title(&content, &filename)
                    };
                    let now = file_mtime.map(|m| m.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                    conn.execute(
                        "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
                         VALUES (?1, ?2, ?3, ?4, ?4, 0)",
                        rusqlite::params![filename, title, is_journal as i32, now],
                    )?;

                    if let Ok(page_id) = conn.query_row(
                        "SELECT id FROM pages WHERE filename = ?1",
                        rusqlite::params![filename],
                        |row| row.get::<_, i64>(0),
                    ) {
                        let _ = db::update_fts_content(conn, page_id, &content);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

pub fn ensure_adoc_extension(name: &str) -> String {
    if name.ends_with(".adoc") {
        name.to_string()
    } else {
        format!("{}.adoc", name)
    }
}

/// Sanitizes a note filename or title, ensuring it has an `.adoc` extension and no path traversal characters.
pub fn sanitize_note_filename(name_or_path: &str) -> String {
    let trimmed = name_or_path.trim();
    let stem = if let Some(s) = trimmed.strip_suffix(".adoc") {
        s
    } else if let Some(s) = trimmed.strip_suffix(".ADOC") {
        s
    } else {
        trimmed
    };

    let sanitized = sanitize_filename(stem);
    let final_stem = sanitized.trim_matches('_');
    if final_stem.is_empty() {
        "Untitled.adoc".to_string()
    } else {
        format!("{}.adoc", final_stem)
    }
}

/// Extracts document title from AsciiDoc content (first `= Title` heading), falling back to filename.
pub fn extract_doc_title(content: &str, fallback_filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("= ") {
            return trimmed.trim_start_matches("= ").trim().to_string();
        }
    }
    fallback_filename.trim_end_matches(".adoc").replace('_', " ")
}

/// Returns a safe PathBuf within `notes_dir` for a note, guaranteed not to escape via path traversal.
pub fn safe_note_path(notes_dir: &Path, name_or_path: &str) -> std::path::PathBuf {
    notes_dir.join(sanitize_note_filename(name_or_path))
}

/// Atomically write content to a file using a temporary sibling file, fsync, and rename.
pub fn atomic_write(path: &Path, content: impl AsRef<[u8]>) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "temp_file".to_string());

    let mut random_bytes = [0u8; 8];
    let rng = ring::rand::SystemRandom::new();
    let nonce = if rng.fill(&mut random_bytes).is_ok() {
        u64::from_ne_bytes(random_bytes)
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    };

    let tmp_path = parent.join(format!(".{}.tmp.{:016x}", file_name, nonce));

    let mut file = std::fs::File::create(&tmp_path)?;
    file.write_all(content.as_ref())?;
    file.sync_all()?;
    drop(file);

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e.into());
    }

    Ok(())
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
        assert!(notes.join(JOURNAL_FILENAME).exists());
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
    fn delete_page_removes_file_and_db_by_title() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "My Test Page", false).unwrap();
        assert!(notes.join("My_Test_Page.adoc").exists());
        let search1 = crate::search::search_pages(&conn, "Test").unwrap();
        assert_eq!(search1.len(), 1);

        delete_page(&conn, &notes, "My Test Page").unwrap();
        assert!(!notes.join("My_Test_Page.adoc").exists());
        let pages = list_pages(&conn).unwrap();
        assert!(pages.is_empty());
        let search2 = crate::search::search_pages(&conn, "Test").unwrap();
        assert!(search2.is_empty());
    }

    #[test]
    fn get_page_finds_by_title_or_filename() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Special Note", false).unwrap();

        assert!(get_page(&conn, "Special Note").unwrap().is_some());
        assert!(get_page(&conn, "Special_Note").unwrap().is_some());
        assert!(get_page(&conn, "Special_Note.adoc").unwrap().is_some());
        assert!(get_page(&conn, "Nonexistent").unwrap().is_none());
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
    fn get_page_preview_blocks_extracts_top_blocks() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Preview Note", false).unwrap();
        let content = "= Preview Note\n\nFirst intro paragraph.\n\n* Item 1\n* Item 2\n\n[NOTE]\nImportant tip!\n";
        std::fs::write(notes.join("Preview_Note.adoc"), content).unwrap();

        let previews = get_page_preview_blocks_with_options(&notes, "Preview_Note.adoc", 3, true);
        assert_eq!(previews.len(), 3);
        assert!(matches!(previews[0], crate::block::Block::Heading { level: 1, .. }));
        assert!(matches!(previews[1], crate::block::Block::Paragraph { .. }));
        assert!(matches!(previews[2], crate::block::Block::UnorderedListItem { .. }));
    }

    #[test]
    fn get_page_preview_json_includes_toc_headings() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Toc Note", false).unwrap();
        let content = "= Toc Note\n:toc:\n\n== Section One\nText 1\n\n== Section Two\nText 2\n";
        std::fs::write(notes.join("Toc_Note.adoc"), content).unwrap();

        let preview_json_str = get_page_preview_json_with_options(&notes, "Toc_Note.adoc", 5, true);
        let parsed: serde_json::Value = serde_json::from_str(&preview_json_str).unwrap();
        assert!(parsed.is_array());
        let toc_obj = parsed.as_array().unwrap().iter().find(|b| b["type"] == "toc").expect("TOC block");
        assert!(toc_obj["headings"].is_array());
        let headings = toc_obj["headings"].as_array().unwrap();
        assert_eq!(headings.len(), 3); // = Toc Note, == Section One, == Section Two
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

    #[test]
    fn sync_and_index_pages_registers_existing_files() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        std::fs::write(notes.join("manual_doc.adoc"), "= Manual Doc\nContent about thermodynamics.\n").unwrap();
        std::fs::write(notes.join(JOURNAL_FILENAME), "= Journal\n== 2026-09-05\n* Logged in\n").unwrap();

        sync_and_index_pages(&conn, &notes).unwrap();

        let page = get_page(&conn, "manual_doc.adoc").unwrap().expect("manual doc should be registered");
        assert_eq!(page.title, "Manual Doc");

        let journal = get_page(&conn, JOURNAL_FILENAME).unwrap().expect("journal should be registered");
        assert!(journal.is_journal);

        // Verify FTS search finds the content
        let results = crate::search::search_pages(&conn, "thermodynamics").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].page.title, "Manual Doc");

        // Now modify the file and verify sync_and_index_pages updates FTS
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(notes.join("manual_doc.adoc"), "= Manual Doc\nUpdated content with quantum mechanics.\n").unwrap();
        sync_and_index_pages(&conn, &notes).unwrap();

        let updated_results = crate::search::search_pages(&conn, "quantum").unwrap();
        assert_eq!(updated_results.len(), 1);
        assert_eq!(updated_results[0].page.title, "Manual Doc");
    }

    #[test]
    fn test_page_info_to_json_value() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        let page = create_page(&conn, &notes, "JSON Test Page", false).unwrap();
        let json_val = page.to_json_value();
        assert_eq!(json_val["title"], "JSON Test Page");
        assert_eq!(json_val["filename"], "JSON_Test_Page.adoc");
        assert_eq!(json_val["is_journal"], false);
    }

    #[test]
    fn test_sanitize_note_filename() {
        assert_eq!(sanitize_note_filename("notes.adoc"), "notes.adoc");
        assert_eq!(sanitize_note_filename("notes"), "notes.adoc");
        assert_eq!(sanitize_note_filename("sub/notes.adoc"), "sub_notes.adoc");
        assert_eq!(sanitize_note_filename("my-note"), "my-note.adoc");
        assert_eq!(sanitize_note_filename("my-note.adoc"), "my-note.adoc");
        assert_eq!(sanitize_note_filename("../../evil.adoc"), "evil.adoc");
        assert_eq!(sanitize_note_filename("../../../"), "Untitled.adoc");
        assert_eq!(sanitize_note_filename(""), "Untitled.adoc");
        assert_eq!(sanitize_note_filename("   "), "Untitled.adoc");
        assert_eq!(sanitize_note_filename("JOURNAL.ADOC"), "JOURNAL.adoc");
    }

    #[test]
    fn test_safe_note_path() {
        let notes = Path::new("/tmp/test_notes");
        let safe1 = safe_note_path(notes, "meeting.adoc");
        assert_eq!(safe1, notes.join("meeting.adoc"));

        let safe2 = safe_note_path(notes, "../../evil.adoc");
        assert_eq!(safe2, notes.join("evil.adoc"));
        assert!(safe2.starts_with(notes));
    }

    #[test]
    fn test_atomic_write() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("atomic_test.adoc");

        atomic_write(&target, "= Atomic Note\nContent here").unwrap();
        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "= Atomic Note\nContent here");

        // Overwrite atomically
        atomic_write(&target, "= Updated Atomic Note\nNew content").unwrap();
        let updated = std::fs::read_to_string(&target).unwrap();
        assert_eq!(updated, "= Updated Atomic Note\nNew content");
    }
}
