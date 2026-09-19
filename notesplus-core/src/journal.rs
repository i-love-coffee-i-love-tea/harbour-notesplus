use crate::NotesError;
use std::path::Path;

use chrono::Local;

use crate::block::Block;
use crate::constants::JOURNAL_FILENAME;
use crate::parser;

/// Check if a line is a date heading for the given date (e.g. YYYY-MM-DD),
/// supporting any heading level (=, ==, ===, etc.) or Markdown style (#, ##, etc.)
pub fn is_date_heading_for_day(line: &str, day_str: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('=') && !trimmed.starts_with('#') {
        return false;
    }
    let rest = trimmed.trim_start_matches('=').trim_start_matches('#').trim();
    if rest == day_str {
        return true;
    }
    if let Some(remainder) = rest.strip_prefix(day_str) {
        if remainder.starts_with(' ')
            || remainder.starts_with(':')
            || remainder.starts_with('-')
            || remainder.starts_with('–')
            || remainder.starts_with('—')
            || remainder.starts_with('\t')
        {
            return true;
        }
    }
    false
}

/// Extract date string (YYYY-MM-DD) from a heading line if it is a date heading.
pub fn get_date_from_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('=') && !trimmed.starts_with('#') {
        return None;
    }
    let rest = trimmed.trim_start_matches('=').trim_start_matches('#').trim();
    if rest.len() >= 10 && is_date_string(&rest[..10]) {
        Some(&rest[..10])
    } else {
        None
    }
}

/// Clean up journal content:
/// - Removes empty date headings for past days.
/// - Removes empty duplicate headings for today if another heading for today exists with content.
/// - Ensures today's heading exists at the top if no heading for today exists.
pub fn clean_journal_content(content: &str, today: &str) -> String {
    let mut blocks = parser::parse_blocks(content);
    let mut indices_to_remove = Vec::new();

    for i in 0..blocks.len() {
        if let Block::Heading { raw, .. } = &blocks[i] {
            let trimmed = raw.trim();
            if let Some(_date) = get_date_from_heading(trimmed) {
                let is_today = is_date_heading_for_day(trimmed, today);

                // Check if all blocks until the next heading are empty
                let mut all_empty = true;
                let mut next_is_same_day_heading = false;
                let mut j = i + 1;
                while j < blocks.len() {
                    match &blocks[j] {
                        Block::Heading { raw: next_raw, .. } => {
                            if is_today && is_date_heading_for_day(next_raw, today) {
                                next_is_same_day_heading = true;
                            }
                            break;
                        }
                        Block::EmptyLine => {}
                        _ => {
                            all_empty = false;
                            break;
                        }
                    }
                    j += 1;
                }

                // If it's an empty past day heading OR an empty duplicate today heading: remove it
                if (all_empty && !is_today) || (all_empty && is_today && next_is_same_day_heading) {
                    indices_to_remove.push(i);
                    for k in (i + 1)..j {
                        indices_to_remove.push(k);
                    }
                }
            }
        }
    }

    // Remove in reverse order to preserve indices
    for &idx in indices_to_remove.iter().rev() {
        if idx < blocks.len() {
            blocks.remove(idx);
        }
    }

    // Check if any heading for today exists
    let has_today_heading = blocks.iter().any(|b| {
        if let Block::Heading { raw, .. } = b {
            is_date_heading_for_day(raw, today)
        } else {
            false
        }
    });

    if !has_today_heading {
        let today_heading_block = Block::Heading {
            level: 2,
            spans: crate::inline::parse_inline(today),
            raw: format!("== {}", today),
        };
        if blocks.is_empty() {
            blocks.push(today_heading_block);
        } else {
            blocks.insert(0, Block::EmptyLine);
            blocks.insert(0, today_heading_block);
        }
    }

    let mut result = parser::blocks_to_adoc(&blocks);
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Initialize the journal for today.
pub fn init_journal(notes_dir: &Path) -> Result<(), NotesError> {
    let path = notes_dir.join(JOURNAL_FILENAME);
    let today = Local::now().format("%Y-%m-%d").to_string();

    let content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let cleaned = clean_journal_content(&content, &today);
    if cleaned != content {
        crate::page::atomic_write(&path, &cleaned)?;
    }
    Ok(())
}

/// Remove date headings (== YYYY-MM-DD) that have no content before the next heading.
/// Preserves today's heading even if empty.
pub fn remove_empty_day_headings(blocks: &mut Vec<Block>, today: &str) {
    let mut indices_to_remove = Vec::new();

    for i in 0..blocks.len() {
        if let Block::Heading { raw, .. } = &blocks[i] {
            let trimmed = raw.trim();
            if is_date_heading_for_day(trimmed, today) {
                continue; // Never remove today's heading
            }
            if get_date_from_heading(trimmed).is_none() {
                continue;
            }

            // Check if all blocks until the next heading are empty
            let mut all_empty = true;
            let mut j = i + 1;
            while j < blocks.len() {
                match &blocks[j] {
                    Block::Heading { .. } => break,
                    Block::EmptyLine => {}
                    _ => {
                        all_empty = false;
                        break;
                    }
                }
                j += 1;
            }

            if all_empty {
                indices_to_remove.push(i);
                // Also mark the empty lines after it
                for k in (i + 1)..j {
                    indices_to_remove.push(k);
                }
            }
        }
    }

    // Remove in reverse order to preserve indices
    for &idx in indices_to_remove.iter().rev() {
        blocks.remove(idx);
    }
}

/// Get the last N non-empty lines from the journal for preview.
pub fn recent_journal_lines(notes_dir: &Path, limit: usize) -> Result<Vec<String>, NotesError> {
    let path = notes_dir.join(JOURNAL_FILENAME);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    let lines: Vec<String> = content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    let start = if lines.len() > limit { lines.len() - limit } else { 0 };
    Ok(lines[start..].to_vec())
}

/// Append a line (task or note) directly under today's date heading in journal.adoc.
pub fn append_to_journal_today(notes_dir: &Path, line: &str) -> Result<(), NotesError> {
    init_journal(notes_dir)?;
    let path = notes_dir.join(JOURNAL_FILENAME);
    let content = std::fs::read_to_string(&path)?;
    let today = Local::now().format("%Y-%m-%d").to_string();

    let lines: Vec<&str> = content.lines().collect();
    let mut today_idx = None;
    for (i, &l) in lines.iter().enumerate() {
        if is_date_heading_for_day(l, &today) {
            today_idx = Some(i);
            break;
        }
    }

    let insert_idx = if let Some(t_idx) = today_idx {
        let mut next_heading_idx = lines.len();
        for (j, &l) in lines.iter().enumerate().skip(t_idx + 1) {
            let trimmed = l.trim();
            if (trimmed.starts_with('=') || trimmed.starts_with('#'))
                && get_date_from_heading(trimmed).is_some() && !is_date_heading_for_day(trimmed, &today) {
                    next_heading_idx = j;
                    break;
                }
        }
        let mut target = next_heading_idx;
        while target > t_idx + 1 && lines[target - 1].trim().is_empty() {
            target -= 1;
        }
        target
    } else {
        lines.len()
    };

    let mut new_lines = Vec::new();
    for (idx, &l) in lines.iter().enumerate() {
        if idx == insert_idx {
            new_lines.push(line);
        }
        new_lines.push(l);
    }
    if insert_idx == lines.len() {
        new_lines.push(line);
    }

    let mut new_content = new_lines.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    crate::page::atomic_write(&path, &new_content)?;
    Ok(())
}

/// Normalize a user-entered line into a task list item if `is_task` is true.
/// Handles various input formats (`- [ ]`, `* `, bare text) and ensures the
/// output always starts with `* [ ] ` or `* [x] `.
pub fn format_task_line(text: &str, is_task: bool) -> String {
    let trimmed = text.trim();
    if !is_task {
        return trimmed.to_string();
    }
    if trimmed.starts_with("* [ ] ") || trimmed.starts_with("* [x] ") || trimmed.starts_with("* [X] ") {
        trimmed.to_string()
    } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        format!("* {}", &trimmed[2..])
    } else if let Some(stripped) = trimmed.strip_prefix("* ") {
        format!("* [ ] {}", stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("- ") {
        format!("* [ ] {}", stripped)
    } else {
        format!("* [ ] {}", trimmed)
    }
}

fn is_date_string(s: &str) -> bool {
    // Simple check: YYYY-MM-DD
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[4] == b'-' && bytes[7] == b'-'
        && bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_journal_creates_today_heading() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        init_journal(notes).unwrap();

        let content = std::fs::read_to_string(notes.join(JOURNAL_FILENAME)).unwrap();
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("== {}", today)));
    }

    #[test]
    fn init_journal_does_not_duplicate_today() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        init_journal(notes).unwrap();
        init_journal(notes).unwrap();

        let content = std::fs::read_to_string(notes.join(JOURNAL_FILENAME)).unwrap();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let count = content.matches(&format!("== {}", today)).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn remove_empty_day_heading() {
        let mut blocks = vec![
            Block::Heading {
                level: 2,
                spans: vec![],
                raw: "== 2020-01-01".into(),
            },
            Block::EmptyLine,
            Block::Heading {
                level: 2,
                spans: vec![],
                raw: "== 2026-06-15".into(),
            },
        ];
        remove_empty_day_headings(&mut blocks, "2026-06-15");
        // 2020-01-01 should be removed (empty), 2026-06-15 preserved (today)
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { raw, .. } => assert_eq!(raw, "== 2026-06-15"),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn remove_empty_preserves_non_empty() {
        let mut blocks = vec![
            Block::Heading {
                level: 2,
                spans: vec![],
                raw: "== 2020-01-01".into(),
            },
            Block::Paragraph {
                spans: vec![],
                raw: "has content".into(),
            },
            Block::EmptyLine,
        ];
        remove_empty_day_headings(&mut blocks, "2026-06-15");
        assert_eq!(blocks.len(), 3); // all preserved
    }

    #[test]
    fn recent_journal_lines_returns_last_n() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        // Each line separated by blank line so parser treats them as separate paragraphs
        let content = "== 2026-01-01\n\nLine 1\n\nLine 2\n\nLine 3\n\nLine 4\n\nLine 5\n";
        std::fs::write(notes.join(JOURNAL_FILENAME), content).unwrap();

        let lines = recent_journal_lines(notes, 3).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Line 3");
    }

    #[test]
    fn recent_journal_lines_empty_journal() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        let lines = recent_journal_lines(notes, 5).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn append_to_journal_today_inserts_under_today_heading() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        let today = Local::now().format("%Y-%m-%d").to_string();

        let initial_content = format!("== {}\n\n* Task 1\n\n== 2026-01-01\n\n* Old task\n", today);
        std::fs::write(notes.join(JOURNAL_FILENAME), initial_content).unwrap();

        append_to_journal_today(notes, "* [ ] Task 2").unwrap();

        let content = std::fs::read_to_string(notes.join(JOURNAL_FILENAME)).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines[0], &format!("== {}", today));
        assert_eq!(lines[1], "* Task 1");
        assert_eq!(lines[2], "* [ ] Task 2");
        assert_eq!(lines[3], "== 2026-01-01");
        assert_eq!(lines[4], "* Old task");
    }

    #[test]
    fn get_journal_blocks_parses_and_preserves_today() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();

        append_to_journal_today(notes, "* [ ] Buy groceries").unwrap();

        let path = notes.join(JOURNAL_FILENAME);
        let content = std::fs::read_to_string(&path).unwrap();
        let mut blocks = crate::parser::parse_blocks_with_options(&content, true);
        let today = Local::now().format("%Y-%m-%d").to_string();
        remove_empty_day_headings(&mut blocks, &today);
        assert!(blocks.len() >= 2);
        match &blocks[0] {
            Block::Heading { raw, .. } => assert!(raw.contains(&today)),
            _ => panic!("expected today heading"),
        }
        match &blocks[1] {
            Block::UnorderedListItem { checked, raw, .. } => {
                assert_eq!(*checked, Some(false));
                assert!(raw.contains("Buy groceries"));
            }
            _ => panic!("expected task item block"),
        }
    }

    #[test]
    fn is_date_heading_for_day_matches_all_heading_levels() {
        assert!(is_date_heading_for_day("= 2026-09-06", "2026-09-06"));
        assert!(is_date_heading_for_day("== 2026-09-06", "2026-09-06"));
        assert!(is_date_heading_for_day("=== 2026-09-06", "2026-09-06"));
        assert!(is_date_heading_for_day("==== 2026-09-06", "2026-09-06"));
        assert!(is_date_heading_for_day("== 2026-09-06 - Sunday", "2026-09-06"));
        assert!(is_date_heading_for_day("# 2026-09-06", "2026-09-06"));
        assert!(is_date_heading_for_day("## 2026-09-06", "2026-09-06"));
        assert!(!is_date_heading_for_day("== 2026-09-05", "2026-09-06"));
        assert!(!is_date_heading_for_day("Not a heading 2026-09-06", "2026-09-06"));
    }

    #[test]
    fn init_journal_preserves_single_and_triple_equals_headings() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        let today = Local::now().format("%Y-%m-%d").to_string();

        // Level 1 heading
        let content = format!("= {}\n\nMy custom entry\n", today);
        std::fs::write(notes.join(JOURNAL_FILENAME), content).unwrap();
        init_journal(notes).unwrap();

        let read_back = std::fs::read_to_string(notes.join(JOURNAL_FILENAME)).unwrap();
        assert_eq!(read_back.matches(&today).count(), 1);
        assert!(read_back.starts_with(&format!("= {}", today)));

        // Level 3 heading
        let content3 = format!("=== {}\n\nMy level 3 entry\n", today);
        std::fs::write(notes.join(JOURNAL_FILENAME), content3).unwrap();
        init_journal(notes).unwrap();

        let read_back3 = std::fs::read_to_string(notes.join(JOURNAL_FILENAME)).unwrap();
        assert_eq!(read_back3.matches(&today).count(), 1);
        assert!(read_back3.starts_with(&format!("=== {}", today)));
    }

    #[test]
    fn clean_journal_content_removes_empty_duplicate_today() {
        let today = "2026-09-06";
        let raw = format!("== {}\n\n= {}\n\nMy note content\n", today, today);
        let cleaned = clean_journal_content(&raw, today);
        assert_eq!(cleaned.matches(today).count(), 1);
        assert!(cleaned.contains("My note content"));
    }
}
