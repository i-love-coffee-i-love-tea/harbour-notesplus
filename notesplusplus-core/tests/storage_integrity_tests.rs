use tempfile::TempDir;
use notesplusplus_core::db::open_db;
use notesplusplus_core::page::{save_and_index_page, get_page};
use notesplusplus_core::parser::parse_preview_blocks;
use notesplusplus_core::block::Block;

#[test]
fn test_automated_fts5_triggers_insert_delete_update() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = open_db(&db_path).unwrap();

    let now = chrono::Utc::now().to_rfc3339();

    // 1. Insert into pages table directly -> trigger pages_ai must populate pages_fts
    conn.execute(
        "INSERT INTO pages (filename, group_path, title, is_journal, created_at, updated_at, block_count)
         VALUES ('architecture.adoc', 'Engineering', 'Architecture Overview', 0, ?1, ?1, 0)",
        rusqlite::params![now],
    ).unwrap();

    let page_id: i64 = conn.last_insert_rowid();

    // Verify FTS search matches title and full path immediately via trigger
    let fts_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'Architecture'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(fts_count, 1, "pages_ai trigger should index title on insert");

    // 2. Update metadata in pages table -> trigger pages_au must update pages_fts
    conn.execute(
        "UPDATE pages SET title = 'Rebranded System', group_path = 'Core' WHERE id = ?1",
        rusqlite::params![page_id],
    ).unwrap();

    let (fts_filename, fts_title): (String, String) = conn.query_row(
        "SELECT filename, title FROM pages_fts WHERE rowid = ?1",
        rusqlite::params![page_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(fts_filename, "Core/architecture.adoc");
    assert_eq!(fts_title, "Rebranded System");

    let fts_rebranded: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pages_fts WHERE pages_fts MATCH 'Rebranded'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(fts_rebranded, 1, "pages_au trigger should update search terms");

    // 3. Delete from pages table -> trigger pages_ad must remove pages_fts entry
    conn.execute("DELETE FROM pages WHERE id = ?1", rusqlite::params![page_id]).unwrap();

    let fts_deleted_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pages_fts WHERE rowid = ?1",
        rusqlite::params![page_id],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(fts_deleted_count, 0, "pages_ad trigger should delete FTS entry on page deletion");
}

#[test]
fn test_two_phase_save_consistency() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let notes_dir = dir.path().join("notes");
    let conn = open_db(&db_path).unwrap();

    let note_name = "Work/Sprint_Plan.adoc";
    let note_content = "= Sprint Plan\n\n* Task 1\n* Task 2\n";

    // Successful save with two-phase commit
    let page_info = save_and_index_page(&conn, &notes_dir, note_name, note_content).unwrap();
    assert_eq!(page_info.title, "Sprint Plan");
    assert_eq!(page_info.group_path, "Work");
    assert_eq!(page_info.filename, "Sprint_Plan.adoc");

    let note_path = notes_dir.join("Work").join("Sprint_Plan.adoc");
    assert!(note_path.exists(), "Target file must exist after two-phase save");

    // Verify no leftover .tmp files
    let tmp_files_count = std::fs::read_dir(notes_dir.join("Work"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(tmp_files_count, 0, "No staging .tmp files should remain");

    // Verify database record
    let retrieved = get_page(&conn, "Work/Sprint_Plan.adoc").unwrap().expect("Page in DB");
    assert_eq!(retrieved.title, "Sprint Plan");
}

#[test]
fn test_incremental_preview_parser_early_stopping() {
    // Construct a large document with 1000 paragraphs
    let mut large_doc = String::from("= Document Title\n\n");
    for i in 1..=1000 {
        large_doc.push_str(&format!("Paragraph {} line of text with some details.\n\n", i));
    }

    // Extract first 4 preview blocks
    let preview_blocks = parse_preview_blocks(&large_doc, 4, true);
    assert_eq!(preview_blocks.len(), 4, "Should extract exactly 4 preview blocks");
    assert!(matches!(preview_blocks[0], Block::Heading { level: 1, .. }));
    assert!(matches!(preview_blocks[1], Block::Paragraph { .. }));
    assert!(matches!(preview_blocks[2], Block::Paragraph { .. }));
    assert!(matches!(preview_blocks[3], Block::Paragraph { .. }));
}

#[test]
fn test_incremental_preview_delimited_blocks() {
    let doc = "\
= My Guide

[source,rust]
----
fn main() {
    println!(\"Hello\");
}
----

|===
| A | B
| 1 | 2
|===

* Checklist item
";

    let preview_blocks = parse_preview_blocks(doc, 4, true);
    assert_eq!(preview_blocks.len(), 4);
    assert!(matches!(preview_blocks[0], Block::Heading { .. }));
    assert!(matches!(preview_blocks[1], Block::CodeBlock { .. }));
    assert!(matches!(preview_blocks[2], Block::Table { .. }));
    assert!(matches!(preview_blocks[3], Block::UnorderedListItem { .. }));
}
