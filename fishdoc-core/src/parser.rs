use crate::block::Block;
use crate::inline::{parse_inline, InlineSpan};

const ADMONITION_KINDS: &[&str] = &["WARNING", "NOTE", "INFO", "TIP", "IMPORTANT"];

/// Parse AsciiDoc text into a list of blocks.
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Empty line
        if line.trim().is_empty() {
            blocks.push(Block::EmptyLine);
            i += 1;
            continue;
        }

        // Document attribute: :toc:, :source-highlighter:, etc. — skip
        if is_doc_attribute(line.trim()) {
            i += 1;
            continue;
        }

        // Attribute line before table or code block: [source,sql], [cols="..."], etc.
        if is_attribute_line(line.trim()) {
            if let Some(next) = next_non_empty(&lines[i + 1..]) {
                if next == "|===" {
                    let (block, consumed) = parse_table(&lines[i..]);
                    blocks.push(block);
                    i += consumed;
                    continue;
                }
                if next.starts_with("----") {
                    let lang = parse_source_lang(line.trim());
                    i += 1; // skip attribute line
                    let (block, consumed) = parse_delimited_block(lines[i], &lines[i..], "----", lang);
                    blocks.push(block);
                    i += consumed;
                    continue;
                }
            }
            // Not before a known block — fall through to paragraph
        }

        // Code block: ---- (must check before horizontal rule)
        if line.trim() == "----" || line.trim().starts_with("----") {
            let (block, consumed) = parse_delimited_block(line, &lines[i..], "----", None);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Literal block: .... (must check before horizontal rule)
        if line.trim() == "...." || line.trim().starts_with("....") {
            let (block, consumed) = parse_delimited_block(line, &lines[i..], "....", None);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Admonition block: [WARNING]/[NOTE]/[INFO]/[TIP]/[IMPORTANT] followed by ====
        if is_admonition_kind(line.trim()) && i + 1 < lines.len() && lines[i + 1].trim() == "====" {
            let kind = line.trim().trim_start_matches('[').trim_end_matches(']').to_string();
            let (block, consumed) = parse_admonition_block(&kind, &lines[i..]);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Horizontal rule: ---, ***, ___ (3+ repeated chars)
        if is_horizontal_rule(line) {
            blocks.push(Block::HorizontalRule { raw: line.to_string() });
            i += 1;
            continue;
        }

        // Heading: = to ======
        if let Some((level, rest)) = parse_heading(line) {
            let spans = parse_inline(rest.trim());
            blocks.push(Block::Heading {
                level,
                spans,
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Blockquote: > text
        if line.starts_with('>') {
            let text = line.strip_prefix('>').unwrap_or(line).trim();
            let spans = parse_inline(text);
            blocks.push(Block::Blockquote {
                spans,
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Table: line starts with |
        if line.trim_start().starts_with('|') {
            let (block, consumed) = parse_table(&lines[i..]);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Ordered list: 1. / a. / i. etc
        if let Some((marker, level, rest)) = parse_ordered_list_item(line) {
            let spans = parse_inline(rest.trim());
            blocks.push(Block::OrderedListItem {
                level,
                marker,
                spans,
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Unordered list: - / * / ** etc
        if let Some((marker, level, rest)) = parse_unordered_list_item(line) {
            let spans = parse_inline(rest.trim());
            blocks.push(Block::UnorderedListItem {
                level,
                marker,
                spans,
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Paragraph: collect consecutive non-empty, non-special lines
        let (block, consumed) = parse_paragraph(&lines[i..]);
        blocks.push(block);
        i += consumed;
    }

    blocks
}

fn is_attribute_line(line: &str) -> bool {
    line.starts_with('[') && line.ends_with(']') && !is_admonition_kind(line)
}

fn parse_source_lang(line: &str) -> Option<String> {
    // [source,sql] -> Some("sql"), [source] -> None
    let t = line.trim();
    if t.starts_with("[source,") && t.ends_with(']') {
        Some(t[8..t.len()-1].to_string())
    } else if t == "[source]" {
        None
    } else {
        None
    }
}

fn is_doc_attribute(line: &str) -> bool {
    // :toc:, :source-highlighter: python, etc.
    if !line.starts_with(':') || !line.ends_with(':') && !line.contains(": ") {
        return false;
    }
    // Must match :name: or :name: value
    if let Some(end) = line[1..].find(':') {
        let name = &line[1..end + 1];
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    } else {
        false
    }
}

fn next_non_empty<'a>(lines: &'a [&str]) -> Option<&'a str> {
    lines.iter().find(|l| !l.trim().is_empty()).map(|l| l.trim())
}

fn is_admonition_kind(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('[') && t.ends_with(']') && ADMONITION_KINDS.contains(&&t[1..t.len()-1])
}

fn parse_admonition_block(kind: &str, lines: &[&str]) -> (Block, usize) {
    // lines[0] is the [KIND] line, lines[1] should be ====
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    // Consume [KIND] line
    raw_parts.push(lines[consumed].to_string());
    consumed += 1;

    // Consume ==== opening
    if consumed < lines.len() && lines[consumed].trim() == "====" {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;
    }

    // Collect content until ====
    let mut content_lines = Vec::new();
    while consumed < lines.len() {
        let line = lines[consumed];
        raw_parts.push(line.to_string());
        if line.trim() == "====" {
            consumed += 1;
            break;
        }
        content_lines.push(line.to_string());
        consumed += 1;
    }

    let raw = raw_parts.join("\n");
    let content_text = content_lines.join(" ");
    let spans = parse_inline(&content_text);
    (Block::Admonition { kind: kind.to_string(), spans, raw }, consumed)
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('=') {
        return None;
    }
    let mut level: u8 = 0;
    for c in trimmed.chars() {
        if c == '=' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level as usize..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some((level, rest))
}

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.chars().next().unwrap();
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    trimmed.chars().all(|c| c == first)
}

fn parse_delimited_block(first_line: &str, lines: &[&str], delimiter: &str, attr_lang: Option<String>) -> (Block, usize) {
    let trimmed_first = first_line.trim();
    // Language from attribute takes priority; fall back to inline (e.g. ----rust)
    let language = attr_lang.or_else(|| {
        if delimiter == "----" && trimmed_first.len() > delimiter.len() {
            Some(trimmed_first[delimiter.len()..].trim().to_string())
        } else {
            None
        }
    });

    let mut content_lines = Vec::new();
    let mut consumed = 1; // skip opening delimiter

    while consumed < lines.len() {
        let line = lines[consumed];
        if line.trim() == delimiter {
            consumed += 1; // skip closing delimiter
            break;
        }
        content_lines.push(line.to_string());
        consumed += 1;
    }

    // If we ran out of lines without finding closing delimiter, treat all as content
    let raw = if consumed <= lines.len() {
        format!("{}\n{}\n{}", delimiter, content_lines.join("\n"), delimiter)
    } else {
        format!("{}\n{}", delimiter, content_lines.join("\n"))
    };

    if delimiter == "----" {
        (Block::CodeBlock { language, lines: content_lines, raw }, consumed)
    } else {
        (Block::LiteralBlock { lines: content_lines, raw }, consumed)
    }
}

fn parse_table(lines: &[&str]) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    // Consume optional attribute lines before |===
    while consumed < lines.len() && is_attribute_line(lines[consumed].trim()) {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;
    }

    // Check for |=== delimited table
    if consumed < lines.len() && lines[consumed].trim() == "|===" {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;

        // Parse cells until |===
        // Cells accumulate into current_row; blank lines flush a row
        let mut current_row: Vec<Vec<InlineSpan>> = Vec::new();

        while consumed < lines.len() {
            let line = lines[consumed];
            raw_parts.push(line.to_string());

            if line.trim() == "|===" {
                consumed += 1;
                // Flush any pending row
                if !current_row.is_empty() {
                    rows.push(current_row);
                    current_row = Vec::new();
                }
                break;
            }

            if line.trim().is_empty() {
                // Blank line = row boundary
                if !current_row.is_empty() {
                    rows.push(current_row);
                    current_row = Vec::new();
                }
                consumed += 1;
                continue;
            }

            if line.trim_start().starts_with('|') {
                let mut cells = parse_table_row(line);
                current_row.append(&mut cells);
            }
            consumed += 1;
        }

        // Flush if table wasn't closed with |===
        if !current_row.is_empty() {
            rows.push(current_row);
        }
    } else {
        // Simple pipe-delimited table (no |=== delimiters)
        while consumed < lines.len() {
            let line = lines[consumed];
            if !line.trim_start().starts_with('|') {
                break;
            }
            raw_parts.push(line.to_string());
            rows.push(parse_table_row(line));
            consumed += 1;
        }
    }

    (Block::Table { rows, raw: raw_parts.join("\n") }, consumed)
}

fn parse_table_row(line: &str) -> Vec<Vec<InlineSpan>> {
    let mut cells: Vec<Vec<InlineSpan>> = line.split('|')
        .skip(1)
        .map(|s| parse_inline(s.trim()))
        .collect();
    // Remove trailing empty cell from trailing pipe
    while cells.last().map_or(false, |c| c.is_empty()) {
        cells.pop();
    }
    cells
}

fn parse_ordered_list_item(line: &str) -> Option<(String, u8, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let level = (indent / 2) as u8;

    // Match: 1. / a. / i. / A. / I. followed by space
    let re = regex::Regex::new(r"^(\d+\.|[a-zA-Z]+\.|[ivxIVX]+\.)\s").unwrap();
    re.captures(trimmed).map(|cap| {
        let marker = cap[1].to_string();
        let rest = &trimmed[cap[0].len()..];
        (marker, level, rest)
    })
}

fn parse_unordered_list_item(line: &str) -> Option<(String, u8, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let level = (indent / 2) as u8;

    // Match: - or * followed by space
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let marker = trimmed[..1].to_string();
        let rest = &trimmed[2..];
        Some((marker, level, rest))
    } else {
        None
    }
}

fn parse_paragraph(lines: &[&str]) -> (Block, usize) {
    let mut text_lines = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        if line.trim().is_empty() {
            break;
        }
        // Stop if we hit a special line
        if is_heading(line) || is_horizontal_rule(line) || is_delimiter(line)
            || line.starts_with('>') || line.trim_start().starts_with('|')
            || parse_ordered_list_item(line).is_some()
            || parse_unordered_list_item(line).is_some()
        {
            break;
        }
        text_lines.push(line);
        consumed += 1;
    }

    let raw = text_lines.join("\n");
    let full_text = text_lines.join(" ");
    let spans = parse_inline(&full_text);

    (Block::Paragraph { spans, raw }, consumed)
}

fn is_heading(line: &str) -> bool {
    parse_heading(line).is_some()
}

fn is_delimiter(line: &str) -> bool {
    let t = line.trim();
    t == "----" || t == "...." || t == "====" || t.starts_with("----") || t.starts_with("....")
}

/// Serialize blocks back to AsciiDoc text.
pub fn blocks_to_adoc(blocks: &[Block]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        parts.push(block.raw_text().to_string());
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_heading_level_1() {
        let blocks = parse_blocks("= Title");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_heading_level_3() {
        let blocks = parse_blocks("=== Sub heading");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { level, raw, .. } => {
                assert_eq!(*level, 3);
                assert_eq!(raw, "=== Sub heading");
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_paragraph_single_line() {
        let blocks = parse_blocks("Hello world");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Paragraph { raw, .. } => assert_eq!(raw, "Hello world"),
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_paragraph_multi_line() {
        let blocks = parse_blocks("Line one\nLine two\nLine three");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Paragraph { raw, .. } => assert_eq!(raw, "Line one\nLine two\nLine three"),
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_empty_lines() {
        let blocks = parse_blocks("hello\n\nworld");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type(), "paragraph");
        assert_eq!(blocks[1].block_type(), "empty_line");
        assert_eq!(blocks[2].block_type(), "paragraph");
    }

    #[test]
    fn parse_horizontal_rule() {
        let blocks = parse_blocks("---");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "horizontal_rule");
    }

    #[test]
    fn parse_horizontal_rule_stars() {
        let blocks = parse_blocks("***");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "horizontal_rule");
    }

    #[test]
    fn parse_code_block() {
        let text = "----\nfn main() {\n    println!(\"hi\");\n}\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { lines, .. } => assert_eq!(lines.len(), 3),
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_code_block_with_language() {
        let text = "----rust\nfn main() {}\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { language, .. } => assert_eq!(language.as_deref(), Some("rust")),
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_literal_block() {
        let text = "....\nliteral text\n....";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "literal_block");
    }

    #[test]
    fn parse_blockquote() {
        let blocks = parse_blocks("> quoted text");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Blockquote { raw, .. } => assert_eq!(raw, "> quoted text"),
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn parse_unordered_list() {
        let blocks = parse_blocks("- item one\n- item two");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type(), "unordered_list_item");
        assert_eq!(blocks[1].block_type(), "unordered_list_item");
    }

    #[test]
    fn parse_ordered_list() {
        let blocks = parse_blocks("1. first\n2. second");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type(), "ordered_list_item");
        assert_eq!(blocks[1].block_type(), "ordered_list_item");
    }

    #[test]
    fn parse_table() {
        let text = "| Name | Value |\n| foo  | bar   |";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_delimited() {
        // Cell-per-line format: blank lines separate rows
        let text = "|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[1].len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_cell_per_line() {
        // Matches the format used in invoice.adoc / technical-doc.adoc
        let text = "|===\n| Description | Amount\n\n| Website Design\n| $2,500.00\n\n| Hosting\n| $120.00\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 3); // header + 2 data rows
                assert_eq!(rows[0].len(), 2); // Description, Amount
                assert_eq!(rows[1].len(), 2); // Website Design, $2,500.00
                assert_eq!(rows[2].len(), 2); // Hosting, $120.00
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_with_cols_attribute() {
        // Real AsciiDoc format: [cols] BEFORE |===
        let text = "[cols=\"1,2\"]\n|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_cols_attribute_not_as_paragraph() {
        // [cols] before |=== must NOT appear as a paragraph
        let text = "== Items\n\n[cols=\"2,1\"]\n|===\n| Name\n| Value\n|===";
        let blocks = parse_blocks(text);
        for b in &blocks {
            if let Block::Paragraph { raw, .. } = b {
                assert!(!raw.contains("[cols"), "cols attribute leaked into paragraph: {}", raw);
            }
        }
    }

    #[test]
    fn parse_admonition_block() {
        let text = "[WARNING]\n====\nBe careful!\n====";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, spans, .. } => {
                assert_eq!(kind, "WARNING");
                assert!(!spans.is_empty());
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_admonition_with_inline_code() {
        let text = "[WARNING]\n====\nAlways run `cargo test` first.\n====";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Admonition { spans, .. } => {
                // Should have a Code span for the backtick part
                assert!(spans.iter().any(|s| matches!(s, InlineSpan::Code(_))));
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_table_with_inline_formatting() {
        let text = "| *bold* | _italic_ |";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
                // First cell should have a Bold span
                assert!(rows[0][0].iter().any(|s| matches!(s, InlineSpan::Bold { .. })));
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_source_attribute_with_lang() {
        let text = "[source,sql]\n----\nSELECT * FROM users;\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { language, lines, .. } => {
                assert_eq!(language.as_deref(), Some("sql"));
                assert_eq!(lines.len(), 1);
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_source_attribute_not_as_paragraph() {
        let text = "[source,sql]\n----\ncode\n----";
        let blocks = parse_blocks(text);
        for b in &blocks {
            if let Block::Paragraph { raw, .. } = b {
                assert!(!raw.contains("[source"), "source attribute leaked into paragraph: {}", raw);
            }
        }
    }

    #[test]
    fn parse_mixed_document() {
        let text = "= Title\n\nSome text\n\n- item 1\n- item 2\n\n----\ncode\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 8); // heading, empty, paragraph, empty, list item, list item, empty, code
    }

    #[test]
    fn round_trip_heading() {
        let text = "== Hello World";
        let blocks = parse_blocks(text);
        let result = blocks_to_adoc(&blocks);
        assert_eq!(result, text);
    }

    #[test]
    fn round_trip_paragraph() {
        let text = "Hello world";
        let blocks = parse_blocks(text);
        let result = blocks_to_adoc(&blocks);
        assert_eq!(result, text);
    }
}
