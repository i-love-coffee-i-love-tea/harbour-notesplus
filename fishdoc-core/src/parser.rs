use crate::block::Block;
use crate::inline::parse_inline;

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

        // Code block: ---- (must check before horizontal rule)
        if line.trim() == "----" || line.trim().starts_with("----") {
            let (block, consumed) = parse_delimited_block(line, &lines[i..], "----", true);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Literal block: .... (must check before horizontal rule)
        if line.trim() == "...." || line.trim().starts_with("....") {
            let (block, consumed) = parse_delimited_block(line, &lines[i..], "....", false);
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

fn parse_delimited_block(first_line: &str, lines: &[&str], delimiter: &str, with_lang: bool) -> (Block, usize) {
    let trimmed_first = first_line.trim();
    let language = if with_lang && trimmed_first.len() > delimiter.len() {
        Some(trimmed_first[delimiter.len()..].trim().to_string())
    } else {
        None
    };

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

    if with_lang {
        (Block::CodeBlock { language, lines: content_lines, raw }, consumed)
    } else {
        (Block::LiteralBlock { lines: content_lines, raw }, consumed)
    }
}

fn parse_table(lines: &[&str]) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    while consumed < lines.len() {
        let line = lines[consumed];
        if !line.trim_start().starts_with('|') {
            break;
        }
        raw_parts.push(line.to_string());
        let cells: Vec<String> = line.split('|')
            .skip(1) // skip empty before first |
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        rows.push(cells);
        consumed += 1;
    }

    (Block::Table { rows, raw: raw_parts.join("\n") }, consumed)
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
    t == "----" || t == "...." || t.starts_with("----") || t.starts_with("....")
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
