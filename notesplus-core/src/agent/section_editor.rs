//! AsciiDoc section boundary detection, line slicing, section replacement,
//! insertion, and appending.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum SectionEditorError {
    #[error("Section '{target}' not found. Available headings in note: {available:?}")]
    SectionNotFound {
        target: String,
        available: Vec<String>,
    },
    #[error("Document is empty")]
    EmptyDocument,
    #[error("Invalid heading level {0}. Must be between 1 and 5.")]
    InvalidHeadingLevel(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionInfo {
    pub level: usize,
    pub title: String,
    pub heading_line: String,
    pub line_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionSpan {
    pub level: usize,
    pub title: String,
    pub start_line: usize,
    pub end_line: usize, // exclusive
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertPosition<'a> {
    BeforeHeading(&'a str),
    AfterHeading(&'a str),
    Top,
    Bottom,
}

/// Lists all AsciiDoc section headings in the document in line order.
pub fn list_sections(document: &str) -> Vec<SectionInfo> {
    let mut sections = Vec::new();
    for (idx, line) in document.lines().enumerate() {
        if let Some((level, title)) = parse_heading_line(line) {
            sections.push(SectionInfo {
                level,
                title,
                heading_line: line.to_string(),
                line_index: idx,
            });
        }
    }
    sections
}

/// Parses an AsciiDoc heading line (e.g. "= Title", "== Section", "=== Sub").
/// Returns (level, clean_title) if the line is a valid heading.
pub fn parse_heading_line(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('=') {
        return None;
    }

    let equals_count = trimmed.chars().take_while(|&c| c == '=').count();
    if equals_count == 0 || equals_count > 5 {
        return None;
    }

    let rest = &trimmed[equals_count..];
    if !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    let title = rest.trim().to_string();
    if title.is_empty() {
        return None;
    }

    Some((equals_count, title))
}

/// Finds the line span [start_line, end_line) of the target heading.
pub fn find_section_span(document: &str, target_heading: &str) -> Result<SectionSpan, SectionEditorError> {
    let sections = list_sections(document);
    let lines: Vec<&str> = document.lines().collect();
    let total_lines = lines.len();

    let target_clean = normalize_heading_query(target_heading);

    let match_idx = sections.iter().position(|s| {
        s.title.eq_ignore_ascii_case(&target_clean) ||
        normalize_heading_query(&s.heading_line).eq_ignore_ascii_case(&target_clean)
    });

    let target_idx = match match_idx {
        Some(idx) => idx,
        None => {
            let available = sections.into_iter().map(|s| s.title).collect();
            return Err(SectionEditorError::SectionNotFound {
                target: target_heading.to_string(),
                available,
            });
        }
    };

    let target_section = &sections[target_idx];
    let start_line = target_section.line_index;
    let target_level = target_section.level;

    // Section ends before next heading of level <= target_level, or at EOF
    let mut end_line = total_lines;
    for next_section in &sections[target_idx + 1..] {
        if next_section.level <= target_level {
            end_line = next_section.line_index;
            break;
        }
    }

    Ok(SectionSpan {
        level: target_level,
        title: target_section.title.clone(),
        start_line,
        end_line,
    })
}

/// Replaces the target section body (and optionally renames its heading).
pub fn edit_section(
    document: &str,
    target_heading: &str,
    new_content: &str,
    new_heading: Option<&str>,
) -> Result<String, SectionEditorError> {
    let span = find_section_span(document, target_heading)?;
    let lines: Vec<&str> = document.lines().collect();

    let mut result_lines = Vec::new();

    // 1. Lines before the section
    for &line in &lines[..span.start_line] {
        result_lines.push(line.to_string());
    }

    // 2. Heading line
    let heading_prefix = "=".repeat(span.level);
    let heading_text = match new_heading {
        Some(nh) => {
            let clean = normalize_heading_query(nh);
            format!("{} {}", heading_prefix, clean)
        }
        None => lines[span.start_line].to_string(),
    };

    let trimmed_new_content = new_content.trim();
    // Check if new_content already starts with the heading
    if parse_heading_line(trimmed_new_content).is_some() {
        for line in trimmed_new_content.lines() {
            result_lines.push(line.to_string());
        }
    } else {
        result_lines.push(heading_text);
        if !trimmed_new_content.is_empty() {
            for line in trimmed_new_content.lines() {
                result_lines.push(line.to_string());
            }
        }
    }

    // 3. Lines after the section
    for &line in &lines[span.end_line..] {
        result_lines.push(line.to_string());
    }

    let mut output = result_lines.join("\n");
    if document.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

/// Appends content to the end of a document or to the end of a specific named section.
pub fn append_to_note(
    document: &str,
    content: &str,
    target_heading: Option<&str>,
) -> Result<String, SectionEditorError> {
    let trimmed_content = content.trim();
    if trimmed_content.is_empty() {
        return Ok(document.to_string());
    }

    if let Some(heading) = target_heading {
        let span = find_section_span(document, heading)?;
        let lines: Vec<&str> = document.lines().collect();

        let mut result_lines = Vec::new();
        for &line in &lines[..span.end_line] {
            result_lines.push(line.to_string());
        }

        for line in trimmed_content.lines() {
            result_lines.push(line.to_string());
        }

        for &line in &lines[span.end_line..] {
            result_lines.push(line.to_string());
        }

        let mut output = result_lines.join("\n");
        if document.ends_with('\n') && !output.ends_with('\n') {
            output.push('\n');
        }
        Ok(output)
    } else {
        let mut output = document.trim_end().to_string();
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(trimmed_content);
        if document.ends_with('\n') || output.contains('\n') {
            output.push('\n');
        }
        Ok(output)
    }
}

/// Inserts a new AsciiDoc section at a specified position.
pub fn insert_section(
    document: &str,
    title: &str,
    level: usize,
    content: &str,
    position: InsertPosition,
) -> Result<String, SectionEditorError> {
    if level == 0 || level > 5 {
        return Err(SectionEditorError::InvalidHeadingLevel(level));
    }

    let heading_prefix = "=".repeat(level);
    let clean_title = normalize_heading_query(title);
    let heading_line = format!("{} {}", heading_prefix, clean_title);

    let mut section_block = String::new();
    section_block.push_str(&heading_line);
    let trimmed_content = content.trim();
    if !trimmed_content.is_empty() {
        section_block.push('\n');
        section_block.push_str(trimmed_content);
    }

    match position {
        InsertPosition::Top => {
            let sections = list_sections(document);
            if let Some(first) = sections.first() {
                // If first section is document title (= Level 1), insert after it
                if first.level == 1 {
                    let span = find_section_span(document, &first.title)?;
                    let lines: Vec<&str> = document.lines().collect();
                    let mut result_lines = Vec::new();
                    // Keep document title and preamble before first subheading
                    let next_section_line = sections.get(1).map(|s| s.line_index).unwrap_or(span.end_line);
                    for &l in &lines[..next_section_line] {
                        result_lines.push(l.to_string());
                    }
                    result_lines.push("".to_string());
                    for l in section_block.lines() {
                        result_lines.push(l.to_string());
                    }
                    for &l in &lines[next_section_line..] {
                        result_lines.push(l.to_string());
                    }
                    let mut output = result_lines.join("\n");
                    if document.ends_with('\n') { output.push('\n'); }
                    return Ok(output);
                }
            }
            let mut output = section_block;
            if !document.trim().is_empty() {
                output.push_str("\n\n");
                output.push_str(document.trim_start());
            }
            if document.ends_with('\n') && !output.ends_with('\n') {
                output.push('\n');
            }
            Ok(output)
        }
        InsertPosition::Bottom => {
            append_to_note(document, &section_block, None)
        }
        InsertPosition::BeforeHeading(target) => {
            let span = find_section_span(document, target)?;
            let lines: Vec<&str> = document.lines().collect();
            let mut result_lines = Vec::new();
            for &l in &lines[..span.start_line] {
                result_lines.push(l.to_string());
            }
            for l in section_block.lines() {
                result_lines.push(l.to_string());
            }
            result_lines.push("".to_string());
            for &l in &lines[span.start_line..] {
                result_lines.push(l.to_string());
            }
            let mut output = result_lines.join("\n");
            if document.ends_with('\n') { output.push('\n'); }
            Ok(output)
        }
        InsertPosition::AfterHeading(target) => {
            let span = find_section_span(document, target)?;
            let lines: Vec<&str> = document.lines().collect();
            let mut result_lines = Vec::new();
            for &l in &lines[..span.end_line] {
                result_lines.push(l.to_string());
            }
            result_lines.push("".to_string());
            for l in section_block.lines() {
                result_lines.push(l.to_string());
            }
            for &l in &lines[span.end_line..] {
                result_lines.push(l.to_string());
            }
            let mut output = result_lines.join("\n");
            if document.ends_with('\n') { output.push('\n'); }
            Ok(output)
        }
    }
}

fn normalize_heading_query(heading: &str) -> String {
    let trimmed = heading.trim();
    if trimmed.starts_with('=') {
        let count = trimmed.chars().take_while(|&c| c == '=').count();
        trimmed[count..].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DOC: &str = "\
= Project Phoenix
Author: Alice
:toc:

== Overview
Phoenix is our next-gen note engine.

== Architecture
Core written in Rust.

=== Database
SQLite with FTS5.

=== Frontend
Sailfish Silica and Vue 3.

== Roadmap
* [ ] Section editing
* [x] Prompt layering
";

    #[test]
    fn test_list_sections() {
        let sections = list_sections(SAMPLE_DOC);
        assert_eq!(sections.len(), 6);
        assert_eq!(sections[0].title, "Project Phoenix");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[1].title, "Overview");
        assert_eq!(sections[1].level, 2);
        assert_eq!(sections[2].title, "Architecture");
        assert_eq!(sections[2].level, 2);
        assert_eq!(sections[3].title, "Database");
        assert_eq!(sections[3].level, 3);
        assert_eq!(sections[4].title, "Frontend");
        assert_eq!(sections[4].level, 3);
        assert_eq!(sections[5].title, "Roadmap");
        assert_eq!(sections[5].level, 2);
    }

    #[test]
    fn test_find_section_span_boundaries() {
        // Architecture has subsections Database and Frontend; ends when Roadmap starts
        let span = find_section_span(SAMPLE_DOC, "Architecture").unwrap();
        assert_eq!(span.level, 2);
        assert_eq!(span.title, "Architecture");
        assert_eq!(span.start_line, 7); // "== Architecture" at line index 7

        // Database is level 3, ends when Frontend starts
        let db_span = find_section_span(SAMPLE_DOC, "Database").unwrap();
        assert_eq!(db_span.level, 3);
        assert_eq!(db_span.start_line, 10);
        assert_eq!(db_span.end_line, 13); // Frontend is line index 13
    }

    #[test]
    fn test_edit_section_body_only() {
        let updated = edit_section(SAMPLE_DOC, "Overview", "New overview description.", None).unwrap();
        assert!(updated.contains("== Overview\nNew overview description."));
        assert!(updated.contains("== Architecture"));
        assert!(updated.contains("== Roadmap"));
    }

    #[test]
    fn test_edit_section_with_heading_rename() {
        let updated = edit_section(SAMPLE_DOC, "Overview", "Executive summary text.", Some("Executive Summary")).unwrap();
        assert!(updated.contains("== Executive Summary\nExecutive summary text."));
        assert!(!updated.contains("== Overview"));
    }

    #[test]
    fn test_edit_nested_subsection_preserves_parent_and_siblings() {
        let updated = edit_section(SAMPLE_DOC, "Database", "PostgreSQL with pgvector.", None).unwrap();
        assert!(updated.contains("=== Database\nPostgreSQL with pgvector."));
        assert!(updated.contains("== Architecture"));
        assert!(updated.contains("=== Frontend\nSailfish Silica and Vue 3."));
    }

    #[test]
    fn test_edit_section_not_found_returns_available_headings() {
        let res = edit_section(SAMPLE_DOC, "Nonexistent Section", "Content", None);
        match res {
            Err(SectionEditorError::SectionNotFound { target, available }) => {
                assert_eq!(target, "Nonexistent Section");
                assert!(available.contains(&"Overview".to_string()));
                assert!(available.contains(&"Architecture".to_string()));
            }
            _ => panic!("Expected SectionNotFound error"),
        }
    }

    #[test]
    fn test_append_to_note_at_end() {
        let updated = append_to_note(SAMPLE_DOC, "== Postscript\nEnd notes.", None).unwrap();
        assert!(updated.ends_with("== Postscript\nEnd notes.\n") || updated.ends_with("== Postscript\nEnd notes."));
    }

    #[test]
    fn test_append_to_named_section() {
        let updated = append_to_note(SAMPLE_DOC, "* [ ] Live ReAct streaming", Some("Roadmap")).unwrap();
        assert!(updated.contains("* [ ] Section editing\n* [x] Prompt layering\n* [ ] Live ReAct streaming"));
    }

    #[test]
    fn test_insert_section_before_and_after() {
        let with_before = insert_section(SAMPLE_DOC, "Background", 2, "Historical context.", InsertPosition::BeforeHeading("Architecture")).unwrap();
        assert!(with_before.contains("== Background\nHistorical context."));
        assert!(with_before.find("== Background").unwrap() < with_before.find("== Architecture").unwrap());

        let with_after = insert_section(SAMPLE_DOC, "Deployment", 2, "Deploy scripts.", InsertPosition::AfterHeading("Architecture")).unwrap();
        assert!(with_after.contains("== Deployment\nDeploy scripts."));
        assert!(with_after.find("== Architecture").unwrap() < with_after.find("== Deployment").unwrap());
        // Must also be after Frontend subsection
        assert!(with_after.find("=== Frontend").unwrap() < with_after.find("== Deployment").unwrap());
    }
}
