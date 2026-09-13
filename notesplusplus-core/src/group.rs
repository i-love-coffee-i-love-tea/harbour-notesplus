use std::path::Path;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::CoreError;
use crate::db;
use crate::page::{self, sanitize_filename};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    pub path: String,           // e.g. "Work/Projects"
    pub display_name: String,   // e.g. "Projects"
    pub note_count: usize,
    pub child_group_count: usize,
    pub collapsed: bool,
    pub sort_order: i32,
}

/// Creates a new note group on disk and registers it in the database.
pub fn create_group(
    conn: &Connection,
    notes_dir: &Path,
    parent_path: &str,
    name: &str,
) -> Result<GroupInfo, CoreError> {
    let sanitized_name = sanitize_filename(name.trim()).trim_matches('_').to_string();
    if sanitized_name.is_empty() {
        return Err(CoreError::Msg("Group name cannot be empty".to_string()));
    }

    let full_path = if parent_path.trim().is_empty() {
        sanitized_name.clone()
    } else {
        let parent_clean = parent_path.trim().trim_matches('/');
        format!("{}/{}", parent_clean, sanitized_name)
    };

    let dir_path = notes_dir.join(&full_path);
    if !dir_path.exists() {
        std::fs::create_dir_all(&dir_path)?;
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO groups (path, display_name, collapsed, sort_order, created_at)
         VALUES (?1, ?2, 0, 0, ?3)
         ON CONFLICT(path) DO UPDATE SET display_name = ?2",
        rusqlite::params![full_path, name.trim(), now],
    )?;

    Ok(GroupInfo {
        path: full_path,
        display_name: name.trim().to_string(),
        note_count: 0,
        child_group_count: 0,
        collapsed: false,
        sort_order: 0,
    })
}

/// Renames a group, moving its directory, updating all child pages and subgroups in SQLite, and batch-rewriting xrefs.
pub fn rename_group(
    conn: &Connection,
    notes_dir: &Path,
    old_path: &str,
    new_name: &str,
) -> Result<String, CoreError> {
    let old_path = old_path.trim().trim_matches('/');
    if old_path.is_empty() {
        return Err(CoreError::Msg("Cannot rename root group".to_string()));
    }

    let sanitized_new_name = sanitize_filename(new_name.trim()).trim_matches('_').to_string();
    if sanitized_new_name.is_empty() {
        return Err(CoreError::Msg("New group name cannot be empty".to_string()));
    }

    let new_path = if let Some((parent, _)) = old_path.rsplit_once('/') {
        format!("{}/{}", parent, sanitized_new_name)
    } else {
        sanitized_new_name.clone()
    };

    if new_path == old_path {
        // Just update display_name
        conn.execute(
            "UPDATE groups SET display_name = ?1 WHERE path = ?2",
            rusqlite::params![new_name.trim(), old_path],
        )?;
        return Ok(new_path);
    }

    let old_dir = notes_dir.join(old_path);
    let new_dir = notes_dir.join(&new_path);

    if new_dir.exists() {
        return Err(CoreError::Msg(format!("Target group directory '{}' already exists", new_path)));
    }

    if old_dir.exists() {
        if let Some(parent) = new_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&old_dir, &new_dir)?;
    }

    // Update the group itself
    conn.execute(
        "UPDATE groups SET path = ?1, display_name = ?2 WHERE path = ?3",
        rusqlite::params![new_path, new_name.trim(), old_path],
    )?;

    // Update child groups in groups table
    let old_prefix_slash = format!("{}/", old_path);
    let new_prefix_slash = format!("{}/", new_path);

    let mut stmt = conn.prepare("SELECT id, path FROM groups WHERE path LIKE ?1")?;
    let child_groups: Vec<(i64, String)> = stmt
        .query_map([format!("{}%", old_prefix_slash)], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(Result::ok)
        .collect();

    for (gid, gpath) in child_groups {
        let updated_gpath = format!("{}{}", new_prefix_slash, gpath.strip_prefix(&old_prefix_slash).unwrap_or(&gpath));
        let _ = conn.execute("UPDATE groups SET path = ?1 WHERE id = ?2", rusqlite::params![updated_gpath, gid]);
    }

    // Update child pages in pages table
    // 1. Direct children
    conn.execute(
        "UPDATE pages SET group_path = ?1 WHERE group_path = ?2",
        rusqlite::params![new_path, old_path],
    )?;

    // 2. Sub-group children
    let mut stmt_pages = conn.prepare("SELECT id, group_path FROM pages WHERE group_path LIKE ?1")?;
    let sub_pages: Vec<(i64, String)> = stmt_pages
        .query_map([format!("{}%", old_prefix_slash)], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(Result::ok)
        .collect();

    for (pid, gpath) in sub_pages {
        let updated_gpath = format!("{}{}", new_prefix_slash, gpath.strip_prefix(&old_prefix_slash).unwrap_or(&gpath));
        let _ = conn.execute("UPDATE pages SET group_path = ?1 WHERE id = ?2", rusqlite::params![updated_gpath, pid]);
    }

    // Batch-rewrite xrefs across all notes in notes_dir for any notes inside this renamed hierarchy
    let all_pages = page::list_pages(conn)?;
    for page in &all_pages {
        let file_path = page::safe_note_path(notes_dir, &page.full_path());
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let rewritten = page::rewrite_xrefs(&content, old_path, &new_path);
            if rewritten != content {
                let _ = page::atomic_write(&file_path, &rewritten);
                let _ = db::update_fts_content(conn, page.id, &rewritten);
            }
        }
    }

    Ok(new_path)
}

/// Deletes a note group. If `recursive` is false and the group contains notes or subgroups, returns an error.
pub fn delete_group(
    conn: &Connection,
    notes_dir: &Path,
    path: &str,
    recursive: bool,
) -> Result<(), CoreError> {
    let clean_path = path.trim().trim_matches('/');
    if clean_path.is_empty() {
        return Err(CoreError::Msg("Cannot delete root group".to_string()));
    }

    let prefix_slash = format!("{}/", clean_path);

    let note_count: usize = conn.query_row(
        "SELECT COUNT(*) FROM pages WHERE group_path = ?1 OR group_path LIKE ?2",
        rusqlite::params![clean_path, format!("{}%", prefix_slash)],
        |row| row.get(0),
    )?;

    let child_group_count: usize = conn.query_row(
        "SELECT COUNT(*) FROM groups WHERE path LIKE ?1",
        rusqlite::params![format!("{}%", prefix_slash)],
        |row| row.get(0),
    )?;

    if !recursive && (note_count > 0 || child_group_count > 0) {
        return Err(CoreError::Msg(format!("Group '{}' is not empty", clean_path)));
    }

    if recursive {
        // Collect and delete all child pages
        let mut stmt = conn.prepare("SELECT id, filename, group_path FROM pages WHERE group_path = ?1 OR group_path LIKE ?2")?;
        let pages_to_delete: Vec<(i64, String, String)> = stmt
            .query_map(rusqlite::params![clean_path, format!("{}%", prefix_slash)], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(Result::ok)
            .collect();

        for (id, filename, gpath) in pages_to_delete {
            let _ = db::delete_fts_entry(conn, id);
            let p = if gpath.is_empty() {
                notes_dir.join(&filename)
            } else {
                notes_dir.join(&gpath).join(&filename)
            };
            if p.exists() {
                let _ = std::fs::remove_file(&p);
            }
            let _ = conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![id]);
        }

        // Delete child groups from DB
        conn.execute(
            "DELETE FROM groups WHERE path LIKE ?1",
            rusqlite::params![format!("{}%", prefix_slash)],
        )?;

        // Remove disk directory recursively
        let dir = notes_dir.join(clean_path);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    } else {
        let dir = notes_dir.join(clean_path);
        if dir.exists() {
            let _ = std::fs::remove_dir(&dir);
        }
    }

    // Delete group from groups table
    conn.execute("DELETE FROM groups WHERE path = ?1", rusqlite::params![clean_path])?;

    Ok(())
}

/// Lists groups, optionally filtered by direct parent or max display depth.
pub fn list_groups(
    conn: &Connection,
    parent_path: Option<&str>,
    max_depth: Option<i32>,
) -> Result<Vec<GroupInfo>, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT path, display_name, collapsed, sort_order FROM groups ORDER BY sort_order ASC, display_name ASC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)? != 0,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut result = Vec::new();
    for row in rows.flatten() {
        let (path, display_name, collapsed, sort_order) = row;

        // Depth check
        let depth = path.split('/').count() as i32;
        if let Some(max_d) = max_depth {
            if depth > max_d {
                continue;
            }
        }

        // Parent filter check
        if let Some(parent) = parent_path {
            let parent_clean = parent.trim().trim_matches('/');
            if parent_clean.is_empty() {
                // Top-level only
                if path.contains('/') {
                    continue;
                }
            } else {
                // Direct children of parent
                if let Some((p, _)) = path.rsplit_once('/') {
                    if p != parent_clean {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        }

        let prefix_slash = format!("{}/", path);
        let note_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM pages WHERE group_path = ?1",
            rusqlite::params![path],
            |r| r.get(0),
        ).unwrap_or(0);

        let child_group_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM groups WHERE path LIKE ?1 AND path NOT LIKE ?2",
            rusqlite::params![format!("{}%", prefix_slash), format!("{}%/%", prefix_slash)],
            |r| r.get(0),
        ).unwrap_or(0);

        result.push(GroupInfo {
            path,
            display_name,
            note_count,
            child_group_count,
            collapsed,
            sort_order,
        });
    }

    Ok(result)
}

/// Returns all groups as a flat list with accurate note counts.
pub fn get_groups_flat(conn: &Connection) -> Result<Vec<GroupInfo>, CoreError> {
    list_groups(conn, None, None)
}

/// Toggles a group's collapsed state and returns the new boolean value.
pub fn toggle_group_collapsed(conn: &Connection, path: &str) -> Result<bool, CoreError> {
    let clean_path = path.trim().trim_matches('/');
    let now = chrono::Utc::now().to_rfc3339();
    let display_name = clean_path.rsplit('/').next().unwrap_or(clean_path);
    conn.execute(
        "INSERT INTO groups (path, display_name, collapsed, sort_order, created_at)
         VALUES (?1, ?2, 1, 0, ?3)
         ON CONFLICT(path) DO UPDATE SET collapsed = NOT collapsed",
        rusqlite::params![clean_path, display_name, now],
    )?;

    let collapsed: bool = conn.query_row(
        "SELECT collapsed FROM groups WHERE path = ?1",
        rusqlite::params![clean_path],
        |row| Ok(row.get::<_, i32>(0)? != 0),
    ).unwrap_or(false);

    Ok(collapsed)
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
    fn test_create_and_list_groups() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        let g1 = create_group(&conn, &notes, "", "Work").unwrap();
        assert_eq!(g1.path, "Work");
        assert_eq!(g1.display_name, "Work");

        let g2 = create_group(&conn, &notes, "Work", "Projects").unwrap();
        assert_eq!(g2.path, "Work/Projects");

        let groups = get_groups_flat(&conn).unwrap();
        assert_eq!(groups.len(), 2);

        let work = groups.iter().find(|g| g.path == "Work").unwrap();
        assert_eq!(work.child_group_count, 1);
    }

    #[test]
    fn test_rename_group_updates_all_child_pages_and_subgroups() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_group(&conn, &notes, "", "Work").unwrap();
        create_group(&conn, &notes, "Work", "Projects").unwrap();

        // Create a note inside Work/Projects
        page::create_page(&conn, &notes, "Work/Projects/Task.adoc", false).unwrap();

        // Rename Work -> Employment
        let new_path = rename_group(&conn, &notes, "Work", "Employment").unwrap();
        assert_eq!(new_path, "Employment");

        // Verify disk directory renamed
        assert!(!notes.join("Work").exists());
        assert!(notes.join("Employment/Projects/Task.adoc").exists());

        // Verify pages table group_path updated
        let pages = page::list_pages(&conn).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].group_path, "Employment/Projects");
        assert_eq!(pages[0].full_path(), "Employment/Projects/Task.adoc");

        // Verify child group path updated in groups table
        let groups = get_groups_flat(&conn).unwrap();
        assert!(groups.iter().any(|g| g.path == "Employment"));
        assert!(groups.iter().any(|g| g.path == "Employment/Projects"));
        assert!(!groups.iter().any(|g| g.path == "Work"));
    }

    #[test]
    fn test_delete_empty_group_succeeds() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_group(&conn, &notes, "", "EmptyFolder").unwrap();
        assert!(notes.join("EmptyFolder").exists());

        delete_group(&conn, &notes, "EmptyFolder", false).unwrap();
        assert!(!notes.join("EmptyFolder").exists());
        assert!(get_groups_flat(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_delete_non_empty_group_non_recursive_fails() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_group(&conn, &notes, "", "Work").unwrap();
        page::create_page(&conn, &notes, "Work/Note.adoc", false).unwrap();

        let res = delete_group(&conn, &notes, "Work", false);
        assert!(res.is_err());
        assert!(notes.join("Work/Note.adoc").exists());
    }

    #[test]
    fn test_delete_group_recursive_removes_all_notes_and_fts() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_group(&conn, &notes, "", "Work").unwrap();
        create_group(&conn, &notes, "Work", "Projects").unwrap();
        page::create_page(&conn, &notes, "Work/Projects/Secret.adoc", false).unwrap();

        delete_group(&conn, &notes, "Work", true).unwrap();
        assert!(!notes.join("Work").exists());
        assert!(page::list_pages(&conn).unwrap().is_empty());
        assert!(get_groups_flat(&conn).unwrap().is_empty());

        let search_res = crate::search::search_pages(&conn, "Secret").unwrap();
        assert!(search_res.is_empty());
    }

    #[test]
    fn test_toggle_group_collapsed() {
        let (conn, dir) = setup();
        let notes = dir.path().join("notes");

        create_group(&conn, &notes, "", "Work").unwrap();
        let c1 = toggle_group_collapsed(&conn, "Work").unwrap();
        assert!(c1);
        let c2 = toggle_group_collapsed(&conn, "Work").unwrap();
        assert!(!c2);
    }
}
