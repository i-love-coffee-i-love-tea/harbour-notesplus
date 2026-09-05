use std::path::Path;

use chrono::Local;

use crate::block::Block;

/// Initialize the journal for today.
/// Uses raw string operations — avoids the AsciiDoc parser which panics on
/// multi-byte UTF-8 characters (em-dash, accented chars, etc.).
pub fn init_journal(notes_dir: &Path) -> Result<(), String> {
    let path = notes_dir.join("journal.adoc");
    let today = Local::now().format("%Y-%m-%d").to_string();
    let today_heading = format!("== {}", today);

    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    // Check if today's heading already exists
    if content.lines().any(|l| l.trim() == today_heading) {
        return Ok(());
    }

    // Prepend today's heading
    let new_content = if content.is_empty() {
        format!("{}\n", today_heading)
    } else {
        format!("{}\n\n{}", today_heading, content)
    };

    std::fs::write(&path, new_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove date headings (== YYYY-MM-DD) that have no content before the next heading.
/// Preserves today's heading even if empty.
pub fn remove_empty_day_headings(blocks: &mut Vec<Block>, today: &str) {
    let today_heading = format!("== {}", today);
    let mut indices_to_remove = Vec::new();

    for i in 0..blocks.len() {
        // Check if this is a level-2 date heading
        if let Block::Heading { level: 2, raw, .. } = &blocks[i] {
            let trimmed = raw.trim();
            if trimmed == today_heading {
                continue; // Never remove today's heading
            }
            // Check if it looks like a date heading
            if !trimmed.starts_with("== ") {
                continue;
            }
            let date_part = &trimmed[3..];
            if !is_date_string(date_part.trim()) {
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
/// Uses raw string splitting — avoids the AsciiDoc parser which panics on
/// multi-byte UTF-8 characters (em-dash, accented chars, etc.).
pub fn recent_journal_lines(notes_dir: &Path, limit: usize) -> Result<Vec<String>, String> {
    let path = notes_dir.join("journal.adoc");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let lines: Vec<String> = content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    let start = if lines.len() > limit { lines.len() - limit } else { 0 };
    Ok(lines[start..].to_vec())
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

        let content = std::fs::read_to_string(notes.join("journal.adoc")).unwrap();
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("== {}", today)));
    }

    #[test]
    fn init_journal_does_not_duplicate_today() {
        let dir = TempDir::new().unwrap();
        let notes = dir.path();
        init_journal(notes).unwrap();
        init_journal(notes).unwrap();

        let content = std::fs::read_to_string(notes.join("journal.adoc")).unwrap();
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
        std::fs::write(notes.join("journal.adoc"), content).unwrap();

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
}
