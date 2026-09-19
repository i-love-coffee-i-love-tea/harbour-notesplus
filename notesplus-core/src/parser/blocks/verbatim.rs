use crate::block::Block;
use crate::parser::delimiters::{
    is_code_delimiter, is_example_delimiter, is_literal_delimiter, is_open_delimiter,
    is_sidebar_delimiter,
};
use crate::parser::parse_blocks;

pub fn parse_image_block(line: &str, title: Option<String>) -> Option<Block> {
    let trimmed = line.trim();
    if !trimmed.starts_with("image::") {
        return None;
    }
    if let Some(open) = trimmed.find('[') {
        if trimmed.ends_with(']') {
            let target = trimmed[7..open].trim().to_string();
            let attr_str = trimmed[open + 1..trimmed.len() - 1].trim();
            let mut alt = String::new();
            let mut width = None;
            let mut height = None;
            if !attr_str.is_empty() {
                let parts: Vec<&str> = attr_str.split(',').map(|s| s.trim()).collect();
                alt = parts[0].to_string();
                if alt.starts_with("alt=") {
                    alt = alt
                        .strip_prefix("alt=")
                        .unwrap()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                }
                for part in &parts[1..] {
                    if part.starts_with("width=") {
                        width = Some(
                            part.strip_prefix("width=")
                                .unwrap()
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_string(),
                        );
                    } else if part.starts_with("height=") {
                        height = Some(
                            part.strip_prefix("height=")
                                .unwrap()
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_string(),
                        );
                    } else if part.chars().all(|c| c.is_ascii_digit()) {
                        if width.is_none() {
                            width = Some(part.to_string());
                        } else if height.is_none() {
                            height = Some(part.to_string());
                        }
                    }
                }
            }
            return Some(Block::Image {
                title,
                target,
                alt,
                width,
                height,
                raw: line.to_string(),
            });
        }
    }
    None
}

fn parse_container_block(
    lines: &[&str],
    title: Option<String>,
    is_closing_delimiter: fn(&str) -> bool,
    make_block: fn(Option<String>, Vec<Block>, String) -> Block,
) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // skip opening delimiter
        }
        if is_closing_delimiter(line.trim()) {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (make_block(title, children, raw_lines.join("\n")), consumed)
}

pub fn parse_open_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    parse_container_block(lines, title, is_open_delimiter, |title, children, raw| {
        Block::Open { title, children, raw }
    })
}

pub fn parse_sidebar_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    parse_container_block(lines, title, is_sidebar_delimiter, |title, children, raw| {
        Block::Sidebar { title, children, raw }
    })
}

pub fn parse_example_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    parse_container_block(lines, title, is_example_delimiter, |title, children, raw| {
        Block::Example { title, children, raw }
    })
}

pub fn parse_delimited_block(
    first_line: &str,
    lines: &[&str],
    delimiter: &str,
    attr_lang: Option<String>,
    title: Option<String>,
) -> (Block, usize) {
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
        let line_trim = line.trim();
        let is_closing = if delimiter == "----" {
            is_code_delimiter(line_trim)
        } else {
            is_literal_delimiter(line_trim)
        };
        if is_closing {
            consumed += 1; // skip closing delimiter
            break;
        }
        content_lines.push(line.to_string());
        consumed += 1;
    }

    // If we ran out of lines without finding closing delimiter, treat all as content
    let raw = if consumed <= lines.len() {
        format!(
            "{}\n{}\n{}",
            delimiter,
            content_lines.join("\n"),
            delimiter
        )
    } else {
        format!("{}\n{}", delimiter, content_lines.join("\n"))
    };

    if delimiter == "----" {
        (
            Block::CodeBlock {
                title,
                language,
                lines: content_lines,
                raw,
            },
            consumed,
        )
    } else {
        (
            Block::LiteralBlock {
                title,
                lines: content_lines,
                raw,
            },
            consumed,
        )
    }
}
