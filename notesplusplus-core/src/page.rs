use std::io::Write;
use std::path::Path;

use ring::rand::SecureRandom;
use rusqlite::Connection;
use serde_json::json;

use crate::constants::{JOURNAL_FILENAME, JOURNAL_TITLE};
use crate::NotesError;
use crate::db;
use crate::xref::rewrite_xrefs;

pub const NOTE_CARD_PALETTE: [&str; 12] = [
    "#e74c3c", // Red
    "#e67e22", // Orange
    "#f1c40f", // Yellow
    "#8bc34a", // Lime
    "#2ecc71", // Green
    "#00b894", // Teal
    "#00a8ff", // Sky Blue
    "#3498db", // Blue
    "#3c40c6", // Indigo
    "#9b59b6", // Purple
    "#e84393", // Pink
    "#e17055", // Coral
];

/// Picks a random color from NOTE_CARD_PALETTE.
pub fn random_note_color() -> &'static str {
    let mut buf = [0u8; 4];
    let sys_rand = ring::rand::SystemRandom::new();
    if sys_rand.fill(&mut buf).is_ok() {
        let val = u32::from_ne_bytes(buf) as usize;
        return NOTE_CARD_PALETTE[val % NOTE_CARD_PALETTE.len()];
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    NOTE_CARD_PALETTE[(nanos as usize) % NOTE_CARD_PALETTE.len()]
}

/// Deterministically computes note card color from title matching QML's hash algorithm.
pub fn compute_note_color(name: &str) -> &'static str {
    let mut hash: u32 = 0;
    for u in name.encode_utf16() {
        hash = (hash.wrapping_mul(31).wrapping_add(u as u32)) & 0x7FFFFFFF;
    }
    NOTE_CARD_PALETTE[(hash as usize) % NOTE_CARD_PALETTE.len()]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageInfo {
    pub id: i64,
    pub filename: String,
    pub group_path: String,
    pub title: String,
    pub is_journal: bool,
    pub created_at: String,
    pub updated_at: String,
    pub block_count: i32,
    pub color: String,
}

impl PageInfo {
    pub fn full_path(&self) -> String {
        if self.group_path.is_empty() {
            self.filename.clone()
        } else {
            format!("{}/{}", self.group_path, self.filename)
        }
    }

    pub fn effective_color(&self) -> &str {
        if !self.color.is_empty() {
            &self.color
        } else {
            compute_note_color(&self.title)
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(PageInfo {
            id: row.get(0)?,
            filename: row.get(1)?,
            group_path: row.get(2)?,
            title: row.get(3)?,
            is_journal: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            block_count: row.get(7)?,
            color: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        })
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "filename": self.filename,
            "group_path": self.group_path,
            "full_path": self.full_path(),
            "title": self.title,
            "name": self.title,
            "color": self.effective_color(),
            "custom_color": self.color,
            "is_journal": self.is_journal,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "block_count": self.block_count,
        })
    }
}

/// Create a new page: write .adoc file + insert into SQLite.
/// Pages are always created with a color (either specified or computed from title).
pub fn create_page(
    conn: &Connection,
    notes_dir: &Path,
    name: &str,
    is_journal: bool,
    color: Option<&str>,
) -> Result<PageInfo, NotesError> {
    let (group_path, filename) = if is_journal {
        ("".to_string(), JOURNAL_FILENAME.to_string())
    } else {
        sanitize_note_path(name)
    };

    let path = if is_journal {
        notes_dir.join(JOURNAL_FILENAME)
    } else {
        safe_note_path(notes_dir, name)
    };
    if path.exists() && !is_journal {
        return Err(NotesError::Msg(format!("Page '{}' already exists", name)));
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    if !group_path.is_empty() {
        let now = chrono::Utc::now().to_rfc3339();
        let display_name = group_path.rsplit('/').next().unwrap_or(&group_path);
        let _ = conn.execute(
            "INSERT OR IGNORE INTO groups (path, display_name, collapsed, sort_order, note_sort, created_at)
             VALUES (?1, ?2, 0, 0, 'newest', ?3)",
            rusqlite::params![group_path, display_name, now],
        );
    }

    let clean_title = if is_journal {
        JOURNAL_TITLE.to_string()
    } else {
        filename.trim_end_matches(".adoc").replace('_', " ")
    };

    let chosen_color = if let Some(c) = color.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        c.to_string()
    } else {
        random_note_color().to_string()
    };

    // Write initial content
    if !is_journal {
        let content = format!("= {}\n\n", clean_title);
        atomic_write(&path, &content)?;
    } else {
        // Journal starts empty; journal.rs handles content
        if !path.exists() {
            atomic_write(&path, "")?;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let rows_affected = conn.execute(
        "INSERT OR IGNORE INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count, color)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0, ?6)",
        rusqlite::params![filename, group_path, clean_title, is_journal as i32, now, chosen_color],
    )?;

    let id = if rows_affected > 0 {
        conn.last_insert_rowid()
    } else {
        conn.query_row(
            "SELECT id FROM pages WHERE group_path = ?1 AND filename = ?2",
            rusqlite::params![group_path, filename],
            |row| row.get(0),
        )?
    };

    // Index initial content into FTS
    if !is_journal {
        let init_fts = format!("= {}\n\n", clean_title);
        let _ = db::update_fts_content(conn, id, &init_fts);
    }

    Ok(PageInfo {
        id,
        filename,
        group_path,
        title: clean_title,
        is_journal,
        created_at: now.clone(),
        updated_at: now,
        block_count: 0,
        color: chosen_color,
    })
}

/// Saves a page's content atomically and updates its title, updated_at, and FTS index directly in SQLite without directory scanning.
pub fn save_and_index_page(
    conn: &Connection,
    notes_dir: &Path,
    name_or_filename: &str,
    content: &str,
) -> Result<PageInfo, NotesError> {
    let (group_path, filename) = sanitize_note_path(name_or_filename);
    let path = safe_note_path(notes_dir, name_or_filename);

    // Write content atomically to disk
    atomic_write(&path, content)?;

    let is_journal = group_path.is_empty() && filename == JOURNAL_FILENAME;
    let title = if is_journal {
        JOURNAL_TITLE.to_string()
    } else {
        extract_doc_title(content, &filename)
    };
    // Check if page already exists in DB
    let existing_page: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, color FROM pages WHERE group_path = ?1 AND filename = ?2",
            rusqlite::params![group_path, filename],
            |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default())),
        )
        .ok();

    let color = if let Some((_, ref prev_color)) = existing_page {
        prev_color.clone()
    } else {
        String::new()
    };
    let now = chrono::Utc::now().to_rfc3339();

    let id = if let Some((id, _)) = existing_page {
        conn.execute(
            "UPDATE pages SET title = ?1, updated_at = ?2, color = ?3 WHERE id = ?4",
            rusqlite::params![title, now, color, id],
        )
        ?;
        id
    } else {
        conn.execute(
            "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count, color)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0, ?6)",
            rusqlite::params![filename, group_path, title, is_journal as i32, now, color],
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
        group_path,
        title,
        is_journal,
        created_at,
        updated_at: now,
        block_count: 0,
        color,
    })
}

/// Slice leading text sufficient to extract `limit` preview blocks without cutting open delimited blocks.
fn slice_preview_content(content: &str, limit: usize) -> &str {
    let min_lines = (limit * 4).max(50);
    let mut in_delim: Option<&str> = None;
    let mut structural_lines = 0;
    let mut line_count = 0;
    let mut cut_byte_idx = content.len();

    let mut remaining = content;
    while !remaining.is_empty() {
        line_count += 1;
        let (line, next_remaining) = match remaining.split_once('\n') {
            Some((l, r)) => (l.strip_suffix('\r').unwrap_or(l), r),
            None => (remaining.strip_suffix('\r').unwrap_or(remaining), ""),
        };
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Some(opener) = crate::parser::as_delimiter_opener(trimmed) {
                in_delim = if in_delim == Some(opener) { None } else { Some(opener) };
            }

            structural_lines += 1;
            if structural_lines >= min_lines && in_delim.is_none() {
                cut_byte_idx = content.len() - next_remaining.len();
                break;
            }
        }
        remaining = next_remaining;
    }

    if line_count <= 60 || cut_byte_idx >= content.len() {
        content
    } else {
        let mut boundary = cut_byte_idx;
        while boundary > 0 && !content.is_char_boundary(boundary) {
            boundary -= 1;
        }
        &content[..boundary]
    }
}

/// Get the first N non-empty preview blocks of a page with configurable options.
pub fn get_page_preview_blocks_with_options(notes_dir: &Path, filename: &str, limit: usize, drop_comments: bool) -> Vec<crate::block::Block> {
    let path = safe_note_path(notes_dir, filename);
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
    let path = safe_note_path(notes_dir, filename);
    if let Ok(content) = std::fs::read_to_string(&path) {
        // Parse :toc: or :toc:N to determine if TOC is present and its depth
        let mut toc_depth: Option<u8> = None;
        for l in content.lines() {
            let t = l.trim();
            if t == ":toc:" {
                toc_depth = Some(5); // default: all levels
            } else if let Some(rest) = t.strip_prefix(":toc:") {
                if let Ok(d) = rest.trim().parse::<u8>() {
                    if (1..=5).contains(&d) {
                        toc_depth = Some(d);
                    }
                }
            }
        }

        let has_toc = toc_depth.is_some();

        let mut headings_vec = Vec::new();
        if has_toc {
            let max_level = toc_depth.unwrap_or(5);
            // Fast line scanner for headings when TOC is present
            let mut block_idx = 0;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    block_idx += 1;
                    continue;
                }
                if let Some((level, rest)) = crate::parser::blocks::headings::parse_heading(line) {
                    if level >= 1 && level <= max_level {
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

            let plain_text = match &block {
                crate::block::Block::Heading { spans, .. }
                | crate::block::Block::Paragraph { spans, .. } => {
                    spans.iter().map(|s| s.plain_text()).collect::<String>()
                }
                crate::block::Block::CodeBlock { lines, .. }
                | crate::block::Block::LiteralBlock { lines, .. } => lines.join("\n"),
                crate::block::Block::UnorderedListItem { raw, .. }
                | crate::block::Block::OrderedListItem { raw, .. } => raw.clone(),
                _ => block.raw_text().to_string(),
            };

            let block_html = if let (Some(theme), Some(opts)) = (qt_theme, qt_options) {
                crate::html::qt_html::render_qt_block(&block, idx, theme, opts)
            } else if qt_options.is_some() || qt_theme.is_some() {
                let default_theme = crate::html::qt_html::QtThemeColors::default();
                let default_opts = crate::html::qt_html::QtRenderOptions::default();
                let theme = qt_theme.unwrap_or(&default_theme);
                let opts = qt_options.unwrap_or(&default_opts);
                crate::html::qt_html::render_qt_block(&block, idx, theme, opts)
            } else {
                crate::html::blocks_to_html_body(&[block.clone()], Some(notes_dir))
            };

            if let serde_json::Value::Object(ref mut map) = json {
                map.insert("text".into(), serde_json::Value::String(plain_text));
                map.insert("html".into(), serde_json::Value::String(block_html));
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
pub fn read_page(notes_dir: &Path, name_or_path: &str) -> Result<String, NotesError> {
    let path = safe_note_path(notes_dir, name_or_path);
    std::fs::read_to_string(&path).map_err(|e| NotesError::Msg(format!("Failed to read {}: {}", name_or_path, e)))
}

/// Delete a page: remove file + delete from SQLite and FTS.
pub fn delete_page(conn: &Connection, notes_dir: &Path, name_or_filename: &str) -> Result<(), NotesError> {
    let page_info = get_page(conn, name_or_filename)?;

    if let Some(info) = page_info {
        let _ = db::delete_fts_entry(conn, info.id);
        let path = safe_note_path(notes_dir, &info.full_path());
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![info.id])
            ?;
    } else {
        let (group_path, filename) = sanitize_note_path(name_or_filename);
        let direct_path = safe_note_path(notes_dir, name_or_filename);
        if direct_path.exists() {
            let _ = std::fs::remove_file(&direct_path);
        }
        conn.execute(
            "DELETE FROM pages WHERE (group_path = ?1 AND filename = ?2) OR filename = ?2 OR title = ?3",
            rusqlite::params![group_path, filename, name_or_filename],
        )
        ?;
    }
    Ok(())
}

/// List all pages sorted by group_path, then updated_at descending.
pub fn list_pages(conn: &Connection) -> Result<Vec<PageInfo>, NotesError> {
    let mut stmt = conn
        .prepare("SELECT id, filename, group_path, title, is_journal, created_at, updated_at, block_count, color FROM pages ORDER BY group_path ASC, updated_at DESC")
        ?;

    let pages = stmt
        .query_map([], |row| PageInfo::from_row(row))
        ?
        .collect::<Result<Vec<_>, _>>()
        ?;

    Ok(pages)
}

/// Get the N most recently updated non-journal pages.
pub fn recent_pages(conn: &Connection, limit: usize) -> Result<Vec<PageInfo>, NotesError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, filename, group_path, title, is_journal, created_at, updated_at, block_count, color
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

/// Get a page by filename, full relative path, or title.
pub fn get_page(conn: &Connection, name_or_filename: &str) -> Result<Option<PageInfo>, NotesError> {
    let (group_path, filename) = sanitize_note_path(name_or_filename);
    let title_without_adoc = name_or_filename
        .rsplit('/')
        .next()
        .unwrap_or(name_or_filename)
        .trim_end_matches(".adoc")
        .trim_end_matches(".ADOC")
        .replace('_', " ");

    let mut stmt = conn
        .prepare(
            "SELECT id, filename, group_path, title, is_journal, created_at, updated_at, block_count, color
             FROM pages
             WHERE (group_path = ?1 AND filename = ?2)
                OR (group_path = ?1 AND title = ?3)
                OR (?1 = '' AND group_path = '' AND filename = ?2)
                OR (?1 = '' AND group_path = '' AND title = ?3)
                OR (filename = ?2)
                OR (title = ?3)
                OR (filename = ?4)
                OR (title = ?4)
             ORDER BY
                CASE
                    WHEN group_path = ?1 AND filename = ?2 THEN 1
                    WHEN group_path = ?1 AND title = ?3 THEN 2
                    WHEN ?1 = '' AND group_path = '' AND filename = ?2 THEN 3
                    WHEN ?1 = '' AND group_path = '' AND title = ?3 THEN 4
                    WHEN filename = ?2 THEN 5
                    WHEN title = ?3 THEN 6
                    WHEN filename = ?4 THEN 7
                    WHEN title = ?4 THEN 8
                    ELSE 9
                END ASC,
                id ASC
             LIMIT 1",
        )
        ?;

    let mut rows = stmt
        .query_map(
            rusqlite::params![
                group_path,
                filename,
                title_without_adoc,
                name_or_filename,
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

/// Sets or removes the custom note color for a page.
pub fn set_page_color(
    conn: &Connection,
    name_or_filename: &str,
    color: Option<&str>,
) -> Result<PageInfo, NotesError> {
    let page_opt = get_page(conn, name_or_filename)?;
    let page = match page_opt {
        Some(p) => p,
        None => return Err(NotesError::Msg(format!("Page '{}' not found", name_or_filename))),
    };

    let assigned_color = match color.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(c) => c.to_string(),
        None => random_note_color().to_string(),
    };
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE pages SET color = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![assigned_color, now, page.id],
    )?;

    Ok(PageInfo {
        color: assigned_color,
        updated_at: now,
        ..page
    })
}

/// Copy example .adoc files to the notes directory on first run
/// and register them in the database under the given group.
/// Skips copying if the target directory already contains .adoc files (not first run).
pub fn copy_examples(conn: &Connection, notes_dir: &Path, examples_dir: &Path, target_group: &str) -> Result<(), NotesError> {
    if !examples_dir.exists() {
        return Ok(());
    }
    let dest_dir = if target_group.is_empty() {
        notes_dir.to_path_buf()
    } else {
        let d = notes_dir.join(target_group);
        let _ = std::fs::create_dir_all(&d);
        d
    };
    // Skip if target already has .adoc files (examples were already copied)
    if !target_group.is_empty() {
        let has_adoc = std::fs::read_dir(&dest_dir)
            .ok()
            .and_then(|mut entries| entries.find(|e| {
                e.as_ref().ok().map_or(false, |e| {
                    e.path().extension().and_then(|ext| ext.to_str()) == Some("adoc")
                })
            }))
            .is_some();
        if has_adoc {
            return Ok(());
        }
    }
    copy_dir_recursive(conn, notes_dir, examples_dir, &dest_dir, target_group, false)?;
    Ok(())
}

/// Returns true if `dir` is an asset directory for a .adoc file in its parent.
/// Convention: a directory named `foo/` is an asset directory if `foo.adoc` exists alongside it.
pub fn is_asset_dir(dir: &Path) -> bool {
    let dir_name = match dir.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    let parent = match dir.parent() {
        Some(p) => p,
        None => return false,
    };
    parent.join(format!("{}.adoc", dir_name)).exists()
}

fn copy_dir_recursive(
    conn: &Connection,
    notes_root: &Path,
    current_src_dir: &Path,
    current_dest_dir: &Path,
    current_group: &str,
    in_asset_dir: bool,
) -> Result<(), NotesError> {
    for entry in std::fs::read_dir(current_src_dir)? {
        let entry = entry?;
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if in_asset_dir {
                // Inside an asset directory: skip .adoc files, copy everything else to notes root
                if ext == "adoc" {
                    continue;
                }
                let root_dest = notes_root.join(&filename);
                if !root_dest.exists() {
                    let _ = std::fs::copy(&path, &root_dest);
                }
                // Also copy to the current dest if different from root
                if current_dest_dir != notes_root {
                    let dest = current_dest_dir.join(&filename);
                    if !dest.exists() {
                        let _ = std::fs::copy(&path, &dest);
                    }
                }
            } else {
                let dest = current_dest_dir.join(&filename);
                if ext == "adoc" {
                    if !dest.exists() {
                        std::fs::copy(&path, &dest)?;
                        let content = std::fs::read_to_string(&dest).unwrap_or_default();
                        let title = extract_doc_title(&content, &filename);
                        let now = chrono::Utc::now().to_rfc3339();
                        conn.execute(
                            "INSERT OR IGNORE INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count, color)
                             VALUES (?1, ?2, ?3, 0, ?4, ?4, 0, '')",
                            rusqlite::params![filename, current_group, title, now],
                        )?;
                        if let Ok(page_id) = conn.query_row(
                            "SELECT id FROM pages WHERE group_path = ?1 AND filename = ?2",
                            rusqlite::params![current_group, filename],
                            |row| row.get::<_, i64>(0),
                        ) {
                            let _ = db::update_fts_content(conn, page_id, &content);
                        }
                    }
                } else if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "svg" || ext == "yml" {
                    if !dest.exists() {
                        let _ = std::fs::copy(&path, &dest);
                    }
                    if current_dest_dir != notes_root {
                        let root_dest = notes_root.join(&filename);
                        if !root_dest.exists() {
                            let _ = std::fs::copy(&path, &root_dest);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            let asset_dir = is_asset_dir(&path);
            if asset_dir {
                // Asset directory: recurse with parent's group, don't register as a group
                let sub_dest = current_dest_dir.join(&filename);
                let _ = std::fs::create_dir_all(&sub_dest);
                let _ = copy_dir_recursive(conn, notes_root, &path, &sub_dest, current_group, true);
            } else {
                // Normal directory: register as group, recurse with new group path
                let sanitized_dir = sanitize_filename(&filename).trim_matches('_').to_string();
                let sub_dest = current_dest_dir.join(&sanitized_dir);
                let _ = std::fs::create_dir_all(&sub_dest);
                let child_group = if current_group.is_empty() {
                    sanitized_dir.clone()
                } else {
                    format!("{}/{}", current_group, sanitized_dir)
                };
                let now = chrono::Utc::now().to_rfc3339();
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO groups (path, display_name, collapsed, sort_order, note_sort, created_at)
                     VALUES (?1, ?2, 0, 0, 'newest', ?3)",
                    rusqlite::params![child_group, filename, now],
                );
                let _ = copy_dir_recursive(conn, notes_root, &path, &sub_dest, &child_group, false);
            }
        }
    }
    Ok(())
}

// Re-export search index operations for backward compatibility.
pub use crate::search_index::{sync_and_index_pages, rebuild_index, cleanup_orphaned_pages, RebuildStats};

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

/// Sanitizes a full relative path, returning (group_path, filename).
pub fn sanitize_note_path(name_or_path: &str) -> (String, String) {
    let normalized = name_or_path.replace('\\', "/");
    let segments: Vec<&str> = normalized
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    if segments.is_empty() {
        return ("".to_string(), "Untitled.adoc".to_string());
    }

    if segments.len() == 1 {
        return ("".to_string(), sanitize_note_filename(segments[0]));
    }

    let group_segments: Vec<String> = segments[..segments.len() - 1]
        .iter()
        .map(|s| sanitize_filename(s).trim_matches('_').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let group_path = group_segments.join("/");
    let filename = sanitize_note_filename(segments[segments.len() - 1]);
    (group_path, filename)
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
            let raw = trimmed.trim_start_matches("= ").trim();
            let spans = crate::inline::parse_inline(raw);
            let plain = spans.iter().map(|s| s.plain_text()).collect::<String>();
            let plain_trimmed = plain.trim();
            if !plain_trimmed.is_empty() {
                return plain_trimmed.to_string();
            }
            return raw.to_string();
        }
    }
    fallback_filename.trim_end_matches(".adoc").replace('_', " ")
}

/// Returns a safe PathBuf within `notes_dir` for a note, guaranteed not to escape via path traversal.
pub fn safe_note_path(notes_dir: &Path, name_or_path: &str) -> std::path::PathBuf {
    let (group_path, filename) = sanitize_note_path(name_or_path);
    if group_path.is_empty() {
        notes_dir.join(filename)
    } else {
        notes_dir.join(group_path).join(filename)
    }
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

/// Moves a note to a new group, updates DB and disk atomically, and rewrites incoming and outgoing cross-references.
pub fn move_page(
    conn: &Connection,
    notes_dir: &Path,
    source_name_or_path: &str,
    target_group: &str,
) -> Result<PageInfo, NotesError> {
    let source_page = get_page(conn, source_name_or_path)?
        .ok_or_else(|| NotesError::Msg(format!("Page '{}' not found", source_name_or_path)))?;

    if source_page.is_journal || source_page.filename == JOURNAL_FILENAME {
        return Err(NotesError::Msg("Cannot move journal page".to_string()));
    }

    let target_group_clean = target_group.trim().trim_matches('/').to_string();

    if source_page.group_path == target_group_clean {
        return Ok(source_page);
    }

    let old_full_path = source_page.full_path();
    let new_full_path = if target_group_clean.is_empty() {
        source_page.filename.clone()
    } else {
        format!("{}/{}", target_group_clean, source_page.filename)
    };

    let src_file = safe_note_path(notes_dir, &old_full_path);
    let dest_file = safe_note_path(notes_dir, &new_full_path);

    if dest_file.exists() && dest_file != src_file {
        return Err(NotesError::Msg(format!("Destination file '{}' already exists", new_full_path)));
    }

    // Ensure target group directory exists on disk and in groups table
    if !target_group_clean.is_empty() {
        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let display_name = target_group_clean.rsplit('/').next().unwrap_or(&target_group_clean);
        let _ = conn.execute(
            "INSERT OR IGNORE INTO groups (path, display_name, collapsed, sort_order, note_sort, created_at)
             VALUES (?1, ?2, 0, 0, 'newest', ?3)",
            rusqlite::params![target_group_clean, display_name, now],
        );
    }

    // Move file on disk
    if src_file.exists() {
        std::fs::rename(&src_file, &dest_file)?;
    }

    // Update database row
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE pages SET group_path = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![target_group_clean, now, source_page.id],
    )?;

    // Update incoming cross-references across all other notes
    let all_pages = list_pages(conn)?;
    for page in &all_pages {
        if page.id == source_page.id {
            continue;
        }
        let file_path = safe_note_path(notes_dir, &page.full_path());
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let mut rewritten = rewrite_xrefs(&content, &old_full_path, &new_full_path);
            if !source_page.group_path.is_empty() && page.group_path == source_page.group_path {
                rewritten = rewrite_xrefs(&rewritten, &source_page.filename, &new_full_path);
            }
            if rewritten != content {
                let _ = atomic_write(&file_path, &rewritten);
                let _ = db::update_fts_content(conn, page.id, &rewritten);
            }
        }
    }

    // Update outgoing cross-references inside the moved note
    if let Ok(content) = std::fs::read_to_string(&dest_file) {
        let mut rewritten = content.clone();
        if !source_page.group_path.is_empty() {
            for sibling in &all_pages {
                if sibling.group_path == source_page.group_path && sibling.id != source_page.id {
                    rewritten = rewrite_xrefs(&rewritten, &sibling.filename, &sibling.full_path());
                }
            }
        }
        if rewritten != content {
            let _ = atomic_write(&dest_file, &rewritten);
        }
    }

    // Re-index moved note in FTS
    if let Ok(content) = std::fs::read_to_string(&dest_file) {
        let _ = db::update_fts_content(conn, source_page.id, &content);
    }

    // Clean up empty source directory if needed
    if let Some(src_parent) = src_file.parent() {
        if src_parent != notes_dir && std::fs::read_dir(src_parent).map(|mut d| d.next().is_none()).unwrap_or(false) {
            let _ = std::fs::remove_dir(src_parent);
        }
    }

    Ok(PageInfo {
        id: source_page.id,
        filename: source_page.filename,
        group_path: target_group_clean,
        title: source_page.title,
        is_journal: source_page.is_journal,
        created_at: source_page.created_at,
        updated_at: now,
        block_count: source_page.block_count,
        color: source_page.color,
    })
}

/// Rename a page: changes the title, renames the file on disk, and updates DB + FTS.
pub fn rename_page(
    conn: &Connection,
    notes_dir: &Path,
    name_or_path: &str,
    new_title: &str,
) -> Result<PageInfo, NotesError> {
    let source_page = get_page(conn, name_or_path)?
        .ok_or_else(|| NotesError::Msg(format!("Page '{}' not found", name_or_path)))?;

    if source_page.is_journal || source_page.filename == JOURNAL_FILENAME {
        return Err(NotesError::Msg("Cannot rename journal page".to_string()));
    }

    let new_filename = ensure_adoc_extension(&sanitize_filename(new_title));
    if new_filename == source_page.filename {
        // Title changed but filename would be the same — just update title
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE pages SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![new_title, now, source_page.id],
        )?;
        return Ok(PageInfo {
            id: source_page.id,
            filename: source_page.filename,
            group_path: source_page.group_path,
            title: new_title.to_string(),
            is_journal: source_page.is_journal,
            created_at: source_page.created_at,
            updated_at: now,
            block_count: source_page.block_count,
            color: source_page.color,
        });
    }

    let old_full_path = source_page.full_path();
    let src_file = safe_note_path(notes_dir, &old_full_path);
    let new_full_path = if source_page.group_path.is_empty() {
        new_filename.clone()
    } else {
        format!("{}/{}", source_page.group_path, new_filename)
    };
    let dest_file = safe_note_path(notes_dir, &new_full_path);

    if dest_file.exists() && dest_file != src_file {
        return Err(NotesError::Msg(format!("A file named '{}' already exists", new_filename)));
    }

    // Update the title inside the .adoc content (first line `= Title`)
    let mut content = std::fs::read_to_string(&src_file).unwrap_or_default();
    if content.starts_with("= ") {
        if let Some(newline_pos) = content.find('\n') {
            content = format!("= {}\n{}", new_title, &content[newline_pos + 1..]);
        } else {
            content = format!("= {}", new_title);
        }
    }

    // Rename file on disk
    if src_file.exists() {
        std::fs::rename(&src_file, &dest_file)?;
    }
    let _ = atomic_write(&dest_file, &content);

    // Update database
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE pages SET filename = ?1, title = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![new_filename, new_title, now, source_page.id],
    )?;

    // Update cross-references in other notes
    let all_pages = list_pages(conn)?;
    for page in &all_pages {
        if page.id == source_page.id {
            continue;
        }
        let file_path = safe_note_path(notes_dir, &page.full_path());
        if let Ok(c) = std::fs::read_to_string(&file_path) {
            let rewritten = rewrite_xrefs(&c, &old_full_path, &new_full_path);
            if rewritten != c {
                let _ = atomic_write(&file_path, &rewritten);
                let _ = db::update_fts_content(conn, page.id, &rewritten);
            }
        }
    }

    // Re-index renamed note in FTS
    let _ = db::update_fts_content(conn, source_page.id, &content);

    Ok(PageInfo {
        id: source_page.id,
        filename: new_filename,
        group_path: source_page.group_path,
        title: new_title.to_string(),
        is_journal: source_page.is_journal,
        created_at: source_page.created_at,
        updated_at: now,
        block_count: source_page.block_count,
        color: source_page.color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::{Arc, Mutex};
    use crate::repository::NoteRepository;

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
        create_page(&conn, &notes, "My Page", false, None).unwrap();
        assert!(notes.join("My_Page.adoc").exists());
    }

    #[test]
    fn create_page_inserts_into_db() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false, None).unwrap();
        let pages = list_pages(&conn).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Test");
    }

    #[test]
    fn create_journal_page() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Journal", true, None).unwrap();
        assert!(notes.join(JOURNAL_FILENAME).exists());
        let pages = list_pages(&conn).unwrap();
        assert!(pages[0].is_journal);
    }

    #[test]
    fn create_duplicate_page_errors() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false, None).unwrap();
        let result = create_page(&conn, &notes, "Test", false, None);
        assert!(result.is_err());
    }

    #[test]
    fn read_page_returns_content() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false, None).unwrap();
        let content = read_page(&notes, "Test.adoc").unwrap();
        assert!(content.contains("= Test"));
    }

    #[test]
    fn delete_page_removes_file() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false, None).unwrap();
        delete_page(&conn, &notes, "Test.adoc").unwrap();
        assert!(!notes.join("Test.adoc").exists());
    }

    #[test]
    fn delete_page_removes_file_and_db_by_title() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "My Test Page", false, None).unwrap();
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
        create_page(&conn, &notes, "Special Note", false, None).unwrap();

        assert!(get_page(&conn, "Special Note").unwrap().is_some());
        assert!(get_page(&conn, "Special_Note").unwrap().is_some());
        assert!(get_page(&conn, "Special_Note.adoc").unwrap().is_some());
        assert!(get_page(&conn, "Nonexistent").unwrap().is_none());
    }

    #[test]
    fn list_pages_sorted_by_updated() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "First", false, None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_page(&conn, &notes, "Second", false, None).unwrap();
        let pages = list_pages(&conn).unwrap();
        assert_eq!(pages[0].title, "Second");
    }

    #[test]
    fn get_page_preview_blocks_extracts_top_blocks() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Preview Note", false, None).unwrap();
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
        create_page(&conn, &notes, "Toc Note", false, None).unwrap();
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
        create_page(&conn, &notes, "Journal", true, None).unwrap();
        create_page(&conn, &notes, "Note", false, None).unwrap();
        let recent = recent_pages(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "Note");
    }

    #[test]
    fn get_page_by_filename() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Test", false, None).unwrap();
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

        copy_examples(&conn, &notes, &examples, "").unwrap();
        assert!(notes.join("test.adoc").exists());
        assert!(!notes.join("not_adoc.txt").exists());
        let pages = list_pages(&conn).unwrap();
        assert!(pages.iter().any(|p| p.filename == "test.adoc"));
    }

    #[test]
    fn copy_examples_nested_groups_and_subgroups() {
        let (conn, dir) = setup();
        let examples = dir.path().join("examples");
        let adr_dir = examples.join("Notes Plus Documentation").join("ADRs");
        std::fs::create_dir_all(&adr_dir).unwrap();
        std::fs::write(adr_dir.join("001-test.adoc"), "= ADR-001\nADR Content\n").unwrap();

        let notes = dir.path().join("notes");
        copy_examples(&conn, &notes, &examples, "").unwrap();

        assert!(notes.join("Notes_Plus_Documentation").join("ADRs").join("001-test.adoc").exists());
        let page = get_page(&conn, "Notes_Plus_Documentation/ADRs/001-test.adoc").unwrap();
        assert!(page.is_some());
        let p = page.unwrap();
        assert_eq!(p.group_path, "Notes_Plus_Documentation/ADRs");
        assert_eq!(p.filename, "001-test.adoc");

        let groups = crate::group::list_groups(&conn, None, None).unwrap();
        assert!(groups.iter().any(|g| g.path == "Notes_Plus_Documentation" || g.path == "Notes_Plus_Documentation/ADRs"));
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
        let page = create_page(&conn, &notes, "JSON Test Page", false, None).unwrap();
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

    #[test]
    fn test_sanitize_note_path_root() {
        let (group, filename) = sanitize_note_path("My Note");
        assert_eq!(group, "");
        assert_eq!(filename, "My_Note.adoc");

        let (group2, filename2) = sanitize_note_path("My_Note.adoc");
        assert_eq!(group2, "");
        assert_eq!(filename2, "My_Note.adoc");
    }

    #[test]
    fn test_sanitize_note_path_nested() {
        let (group, filename) = sanitize_note_path("Work/Projects/Sprint 1");
        assert_eq!(group, "Work/Projects");
        assert_eq!(filename, "Sprint_1.adoc");

        let (group2, filename2) = sanitize_note_path("Personal\\Finances\\Budget.adoc");
        assert_eq!(group2, "Personal/Finances");
        assert_eq!(filename2, "Budget.adoc");
    }

    #[test]
    fn test_sanitize_note_path_blocks_traversal() {
        let (group, filename) = sanitize_note_path("../../etc/passwd");
        assert_eq!(group, "etc");
        assert_eq!(filename, "passwd.adoc");

        let (group2, filename2) = sanitize_note_path("./foo/../bar/test.adoc");
        assert_eq!(group2, "foo/bar");
        assert_eq!(filename2, "test.adoc");
    }

    #[test]
    fn test_safe_note_path_containment() {
        let notes = Path::new("/tmp/test_notes");
        let safe1 = safe_note_path(notes, "Work/Projects/Note.adoc");
        assert_eq!(safe1, notes.join("Work/Projects/Note.adoc"));

        let safe2 = safe_note_path(notes, "../../etc/passwd");
        assert_eq!(safe2, notes.join("etc/passwd.adoc"));
        assert!(safe2.starts_with(notes));
    }

    #[test]
    fn test_sync_dir_recursive_and_orphan_cleanup() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        // Create nested notes on disk
        let work_dir = notes.join("Work").join("Projects");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(work_dir.join("Plan.adoc"), "= Sprint Plan\nImportant work items.\n").unwrap();

        let personal_dir = notes.join("Personal");
        std::fs::create_dir_all(&personal_dir).unwrap();
        std::fs::write(personal_dir.join("Todo.adoc"), "= Todo\nBuy groceries.\n").unwrap();

        // Run sync
        sync_and_index_pages(&conn, &notes).unwrap();

        let pages = list_pages(&conn).unwrap();
        assert_eq!(pages.len(), 2);

        let plan_page = pages.iter().find(|p| p.filename == "Plan.adoc").expect("Plan.adoc exists");
        assert_eq!(plan_page.group_path, "Work/Projects");
        assert_eq!(plan_page.full_path(), "Work/Projects/Plan.adoc");
        assert_eq!(plan_page.title, "Sprint Plan");

        let todo_page = pages.iter().find(|p| p.filename == "Todo.adoc").expect("Todo.adoc exists");
        assert_eq!(todo_page.group_path, "Personal");
        assert_eq!(todo_page.full_path(), "Personal/Todo.adoc");

        // FTS search on nested content and folder path
        let search_work = crate::search::search_pages(&conn, "Sprint").unwrap();
        assert_eq!(search_work.len(), 1);
        assert_eq!(search_work[0].page.group_path, "Work/Projects");

        // Now test orphan cleanup: delete Plan.adoc from disk and re-sync
        std::fs::remove_file(work_dir.join("Plan.adoc")).unwrap();
        sync_and_index_pages(&conn, &notes).unwrap();

        let pages_after_cleanup = list_pages(&conn).unwrap();
        assert_eq!(pages_after_cleanup.len(), 1);
        assert_eq!(pages_after_cleanup[0].filename, "Todo.adoc");

        let search_after_cleanup = crate::search::search_pages(&conn, "Sprint").unwrap();
        assert_eq!(search_after_cleanup.len(), 0);
    }

    #[test]
    fn test_sync_recursion_depth_capped_at_10() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        // Create directory nested 12 levels deep
        let mut deep_dir = notes.clone();
        for i in 1..=12 {
            deep_dir = deep_dir.join(format!("level{}", i));
        }
        std::fs::create_dir_all(&deep_dir).unwrap();
        std::fs::write(deep_dir.join("Deep.adoc"), "= Deep Note\nToo deep.\n").unwrap();

        // Create directory nested 5 levels deep
        let mut ok_dir = notes.clone();
        for i in 1..=5 {
            ok_dir = ok_dir.join(format!("ok{}", i));
        }
        std::fs::create_dir_all(&ok_dir).unwrap();
        std::fs::write(ok_dir.join("Ok.adoc"), "= OK Note\nJust right.\n").unwrap();

        sync_and_index_pages(&conn, &notes).unwrap();

        let pages = list_pages(&conn).unwrap();
        assert!(pages.iter().any(|p| p.filename == "Ok.adoc"));
        assert!(!pages.iter().any(|p| p.filename == "Deep.adoc"));
    }

    #[test]
    fn test_move_page_incoming_and_outgoing_xrefs() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        // Create NoteB in Work
        create_page(&conn, &notes, "Work/NoteB.adoc", false, None).unwrap();

        // Create NoteA in Work linking to NoteB
        let note_a_content = "= Note A\nSee xref:NoteB.adoc[Sibling] and <<NoteB>>.\n";
        save_and_index_page(&conn, &notes, "Work/NoteA.adoc", note_a_content).unwrap();

        // Create NoteC at root linking to NoteA
        let note_c_content = "= Note C\nReference xref:Work/NoteA.adoc#sec[Note A].\n";
        save_and_index_page(&conn, &notes, "NoteC.adoc", note_c_content).unwrap();

        // Move NoteA to Personal
        let moved_page = move_page(&conn, &notes, "Work/NoteA.adoc", "Personal").unwrap();
        assert_eq!(moved_page.group_path, "Personal");
        assert_eq!(moved_page.full_path(), "Personal/NoteA.adoc");
        assert!(notes.join("Personal/NoteA.adoc").exists());
        assert!(!notes.join("Work/NoteA.adoc").exists());

        // Verify incoming link in NoteC was updated to Personal/NoteA.adoc
        let note_c_updated = std::fs::read_to_string(notes.join("NoteC.adoc")).unwrap();
        assert_eq!(note_c_updated, "= Note C\nReference xref:Personal/NoteA.adoc#sec[Note A].\n");

        // Verify outgoing link in moved NoteA was updated to Work/NoteB.adoc
        let note_a_updated = std::fs::read_to_string(notes.join("Personal/NoteA.adoc")).unwrap();
        assert!(note_a_updated.contains("xref:Work/NoteB.adoc[Sibling]"));
        assert!(note_a_updated.contains("<<Work/NoteB>>"));
    }

    #[test]
    fn test_move_page_rejects_journal() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_page(&conn, &notes, "Journal", true, None).unwrap();
        let res = move_page(&conn, &notes, JOURNAL_FILENAME, "Personal");
        assert!(res.is_err());
    }

    #[test]
    fn test_get_page_scoped_to_group() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_page(&conn, &notes, "Subgroup/Hendrix.adoc", false, None).unwrap();

        let page_by_full_path = get_page(&conn, "Subgroup/Hendrix.adoc").unwrap().expect("Subgroup note found by full path");
        assert_eq!(page_by_full_path.group_path, "Subgroup");
        assert_eq!(page_by_full_path.filename, "Hendrix.adoc");
        assert_eq!(page_by_full_path.title, "Hendrix");

        let page_by_group_title = get_page(&conn, "Subgroup/Hendrix").unwrap().expect("Subgroup note found by group/title");
        assert_eq!(page_by_group_title.group_path, "Subgroup");
        assert_eq!(page_by_group_title.filename, "Hendrix.adoc");

        let page_by_bare_name = get_page(&conn, "Hendrix").unwrap().expect("Subgroup note found by fallback bare name");
        assert_eq!(page_by_bare_name.group_path, "Subgroup");
    }

    #[test]
    fn test_get_page_root_vs_subgroup_collision() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_page(&conn, &notes, "Hendrix.adoc", false, None).unwrap();
        create_page(&conn, &notes, "Subgroup/Hendrix.adoc", false, None).unwrap();

        let root_note = get_page(&conn, "Hendrix.adoc").unwrap().expect("Root note found");
        assert_eq!(root_note.group_path, "");
        assert_eq!(root_note.filename, "Hendrix.adoc");

        let root_note_title = get_page(&conn, "Hendrix").unwrap().expect("Root note found by title");
        assert_eq!(root_note_title.group_path, "");
        assert_eq!(root_note_title.filename, "Hendrix.adoc");

        let subgroup_note = get_page(&conn, "Subgroup/Hendrix.adoc").unwrap().expect("Subgroup note found");
        assert_eq!(subgroup_note.group_path, "Subgroup");
        assert_eq!(subgroup_note.filename, "Hendrix.adoc");

        let subgroup_note_title = get_page(&conn, "Subgroup/Hendrix").unwrap().expect("Subgroup note found by path without adoc");
        assert_eq!(subgroup_note_title.group_path, "Subgroup");
        assert_eq!(subgroup_note_title.filename, "Hendrix.adoc");
    }

    #[test]
    fn test_safe_note_path_nested_group() {
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        let path = safe_note_path(notes_dir, "Subgroup/Hendrix.adoc");
        assert_eq!(path, notes_dir.join("Subgroup").join("Hendrix.adoc"));

        let root_path = safe_note_path(notes_dir, "Hendrix.adoc");
        assert_eq!(root_path, notes_dir.join("Hendrix.adoc"));
    }

    #[test]
    fn safe_note_path_blocks_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let notes_dir = temp_dir.path();

        // Basic traversal
        let path = safe_note_path(notes_dir, "../../../etc/passwd");
        assert!(path.starts_with(notes_dir), "traversal path must stay within notes_dir: {:?}", path);

        // Traversal with valid prefix
        let path2 = safe_note_path(notes_dir, "valid/../../../etc/passwd");
        assert!(path2.starts_with(notes_dir), "traversal path must stay within notes_dir: {:?}", path2);

        // Dot-dot in middle
        let path3 = safe_note_path(notes_dir, "group/../../secret.adoc");
        assert!(path3.starts_with(notes_dir), "traversal path must stay within notes_dir: {:?}", path3);
    }

    #[test]
    fn sanitize_note_path_strips_dotdot() {
        let (group, file) = sanitize_note_path("../../../etc/passwd");
        assert!(!group.contains(".."), "group must not contain '..': {}", group);
        assert!(!file.contains(".."), "filename must not contain '..': {}", file);

        let (group2, file2) = sanitize_note_path("valid/../../../etc/passwd");
        assert!(!group2.contains(".."), "group must not contain '..': {}", group2);
        assert!(!file2.contains(".."), "filename must not contain '..': {}", file2);
    }

    #[test]
    fn repository_note_exists_prevents_traversal() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        create_page(&conn, &notes, "Secret", false, None).unwrap();

        let repo = crate::repository::FsSqliteNoteRepository::new(&notes, Arc::new(Mutex::new(conn)));

        // Should not find files outside notes dir
        assert!(!repo.note_exists("../../../etc/passwd"));
        assert!(!repo.note_exists("valid/../../../etc/passwd"));

        // Should find existing note
        assert!(repo.note_exists("Secret.adoc"));
        assert!(!repo.note_exists("Nonexistent.adoc"));
    }

    #[test]
    fn repository_read_note_content_stays_in_dir() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");
        // Write a secret file outside notes dir
        std::fs::write(dir.path().join("secret.txt"), "classified").unwrap();

        let repo = crate::repository::FsSqliteNoteRepository::new(&notes, Arc::new(Mutex::new(conn)));

        // Traversal attempt should fail
        let result = repo.read_note_content("../secret.txt");
        assert!(result.is_err(), "traversal read must fail");

        let result2 = repo.read_note_content("../../../etc/passwd");
        assert!(result2.is_err(), "traversal read must fail");
    }

    #[test]
    fn test_compute_note_color_deterministic() {
        assert_eq!(compute_note_color(""), NOTE_CARD_PALETTE[0]);
        let c1 = compute_note_color("Project Architecture");
        let c2 = compute_note_color("Project Architecture");
        assert_eq!(c1, c2);
        assert!(NOTE_CARD_PALETTE.contains(&c1));

        // Test hash parity with JS 31-multiplier
        let mut expected_hash: u32 = 0;
        for u in "Meeting Notes".encode_utf16() {
            expected_hash = (expected_hash.wrapping_mul(31).wrapping_add(u as u32)) & 0x7FFFFFFF;
        }
        let expected_color = NOTE_CARD_PALETTE[(expected_hash as usize) % NOTE_CARD_PALETTE.len()];
        assert_eq!(compute_note_color("Meeting Notes"), expected_color);
    }

    #[test]
    fn test_page_info_to_json_value_includes_color_and_name() {
        let page = PageInfo {
            id: 42,
            filename: "demo.adoc".to_string(),
            group_path: "Work".to_string(),
            title: "Demo Title".to_string(),
            is_journal: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
            block_count: 5,
            color: String::new(),
        };

        let json_val = page.to_json_value();
        assert_eq!(json_val["id"], 42);
        assert_eq!(json_val["title"], "Demo Title");
        assert_eq!(json_val["name"], "Demo Title");
        assert_eq!(json_val["color"], compute_note_color("Demo Title"));
        assert_eq!(json_val["full_path"], "Work/demo.adoc");

        let custom_page = PageInfo {
            color: "#e74c3c".to_string(),
            ..page
        };
        let custom_json = custom_page.to_json_value();
        assert_eq!(custom_json["color"], "#e74c3c");
        assert_eq!(custom_json["custom_color"], "#e74c3c");
    }

    #[test]
    fn test_set_page_color_lifecycle() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        let page = create_page(&conn, &notes, "ColorNote", false, Some("#2ecc71")).unwrap();
        assert_eq!(page.color, "#2ecc71");
        assert_eq!(page.effective_color(), "#2ecc71");

        // Verify stored in DB
        let fetched = get_page(&conn, "ColorNote").unwrap().expect("Should find ColorNote");
        assert_eq!(fetched.color, "#2ecc71");

        // Change color
        let updated = set_page_color(&conn, "ColorNote", Some("#9b59b6")).unwrap();
        assert_eq!(updated.color, "#9b59b6");
        let fetched2 = get_page(&conn, "ColorNote").unwrap().unwrap();
        assert_eq!(fetched2.color, "#9b59b6");

        // Clear color (assigns a random palette color)
        let cleared = set_page_color(&conn, "ColorNote", None).unwrap();
        assert!(NOTE_CARD_PALETTE.contains(&cleared.color.as_str()));
        assert_eq!(cleared.effective_color(), cleared.color);
    }

    #[test]
    fn test_random_note_color() {
        for _ in 0..50 {
            let color = random_note_color();
            assert!(NOTE_CARD_PALETTE.contains(&color));
        }
    }
}
