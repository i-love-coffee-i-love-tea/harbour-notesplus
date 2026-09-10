use crate::block::Block;
use crate::inline::parse_inline;
use crate::parser::attributes::{is_attribute_line, is_doc_attribute};
use crate::parser::blocks::headings::is_heading;
use crate::parser::blocks::lists::{parse_ordered_list_item, parse_unordered_list_item};
use crate::parser::delimiters::{
    is_admonition_kind, is_code_delimiter, is_delimiter, is_example_delimiter, is_horizontal_rule,
    is_quote_delimiter,
};
use crate::parser::parse_blocks;

fn is_paragraph_break(line: &str, consumed: usize) -> bool {
    line.trim().is_empty()
        || (consumed > 0 && is_attribute_line(line.trim()))
        || is_heading(line)
        || is_horizontal_rule(line)
        || is_delimiter(line)
        || is_doc_attribute(line.trim())
        || line.trim() == "<<<"
        || line.starts_with('>')
        || line.trim_start().starts_with('|')
        || parse_ordered_list_item(line).is_some()
        || parse_unordered_list_item(line).is_some()
}

pub fn parse_admonition_block(kind: &str, lines: &[&str], title: Option<String>) -> (Block, usize) {
    // lines[0] is the [KIND] line, lines[1] should be ====
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    // Consume [KIND] line
    raw_parts.push(lines[consumed].to_string());
    consumed += 1;

    // Consume ==== opening
    if consumed < lines.len() && is_example_delimiter(lines[consumed].trim()) {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;
    }

    let mut content_lines = Vec::new();
    let mut closed = false;
    let mut last_content_blank = true;
    while consumed < lines.len() {
        let line = lines[consumed];
        let trimmed = line.trim();
        if is_example_delimiter(trimmed) {
            raw_parts.push(line.to_string());
            consumed += 1;
            closed = true;
            break;
        }
        let is_hr = is_horizontal_rule(line) && !is_code_delimiter(trimmed);
        if last_content_blank && (is_heading(line) || is_hr || is_admonition_kind(line)) {
            break;
        }
        raw_parts.push(line.to_string());
        content_lines.push(line.to_string());
        consumed += 1;
        last_content_blank = trimmed.is_empty();
    }

    if !closed {
        log::warn!(
            "admonition {} missing closing '===='; block closed at next structural boundary",
            kind
        );
    }

    let raw = raw_parts.join("\n");
    let content = content_lines.join("\n");
    let children = parse_blocks(&content);
    (
        Block::Admonition {
            title,
            kind: kind.to_string(),
            children,
            raw,
        },
        consumed,
    )
}

pub fn parse_admonition_paragraph(
    kind: &str,
    lines: &[&str],
    title: Option<String>,
) -> (Block, usize) {
    let mut text_lines = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        if is_paragraph_break(line, consumed) {
            if consumed == 0 && !line.trim().is_empty() {
                text_lines.push(line);
                consumed += 1;
            }
            break;
        }
        text_lines.push(line);
        consumed += 1;
    }

    if consumed == 0 && !lines.is_empty() {
        text_lines.push(lines[0]);
        consumed = 1;
    }

    let raw = text_lines.join("\n");
    let full_text = text_lines.join(" ");
    let spans = parse_inline(&full_text);
    let children = vec![Block::Paragraph {
        spans,
        raw: raw.clone(),
    }];

    (
        Block::Admonition {
            title,
            kind: kind.to_string(),
            children,
            raw,
        },
        consumed,
    )
}

pub fn parse_quote_block(
    lines: &[&str],
    title: Option<String>,
    attribution: Option<String>,
    citation: Option<String>,
) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening ____
        }
        if is_quote_delimiter(line.trim()) {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (
        Block::Blockquote {
            title,
            attribution,
            citation,
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    )
}

pub fn parse_quote_paragraph(
    lines: &[&str],
    title: Option<String>,
    attribution: Option<String>,
    citation: Option<String>,
) -> (Block, usize) {
    let mut text_lines = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        if is_paragraph_break(line, consumed) {
            if consumed == 0 && !line.trim().is_empty() {
                text_lines.push(line);
                consumed += 1;
            }
            break;
        }
        text_lines.push(line);
        consumed += 1;
    }

    if consumed == 0 && !lines.is_empty() {
        text_lines.push(lines[0]);
        consumed = 1;
    }

    let raw = text_lines.join("\n");
    let full_text = text_lines.join(" ");
    let spans = parse_inline(&full_text);
    let children = vec![Block::Paragraph {
        spans,
        raw: raw.clone(),
    }];

    (
        Block::Blockquote {
            title,
            attribution,
            citation,
            children,
            raw,
        },
        consumed,
    )
}

fn build_verse_block(
    verse_lines: Vec<String>,
    raw_lines: Vec<String>,
    title: Option<String>,
    attribution: Option<String>,
    citation: Option<String>,
) -> Block {
    let spans = verse_lines.iter().map(|l| parse_inline(l)).collect();
    Block::Verse {
        title,
        attribution,
        citation,
        lines: verse_lines,
        spans,
        raw: raw_lines.join("\n"),
    }
}

pub fn parse_verse_block(
    lines: &[&str],
    title: Option<String>,
    attribution: Option<String>,
    citation: Option<String>,
) -> (Block, usize) {
    let mut verse_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening ____
        }
        if is_quote_delimiter(line.trim()) {
            break;
        }
        verse_lines.push(line.to_string());
    }

    (
        build_verse_block(verse_lines, raw_lines, title, attribution, citation),
        consumed,
    )
}

pub fn parse_verse_paragraph(
    lines: &[&str],
    title: Option<String>,
    attribution: Option<String>,
    citation: Option<String>,
) -> (Block, usize) {
    let mut verse_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        if is_paragraph_break(line, consumed) {
            if consumed == 0 && !line.trim().is_empty() {
                verse_lines.push(line.to_string());
                raw_lines.push(line.to_string());
                consumed += 1;
            }
            break;
        }
        verse_lines.push(line.to_string());
        raw_lines.push(line.to_string());
        consumed += 1;
    }

    if consumed == 0 && !lines.is_empty() {
        verse_lines.push(lines[0].to_string());
        raw_lines.push(lines[0].to_string());
        consumed = 1;
    }

    (
        build_verse_block(verse_lines, raw_lines, title, attribution, citation),
        consumed,
    )
}
