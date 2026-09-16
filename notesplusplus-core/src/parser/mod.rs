pub mod attributes;
pub mod blocks;
pub mod delimiters;
pub mod preprocessor;

#[cfg(test)]
mod tests;

pub use attributes::*;
pub use blocks::*;
pub use delimiters::*;
pub use preprocessor::*;

use crate::block::Block;
use crate::inline::parse_inline;

/// Parse AsciiDoc text into a list of blocks. Guaranteed not to panic.
pub fn parse_blocks(text: &str) -> Vec<Block> {
    parse_blocks_with_options(text, true)
}

/// Parse early-stopping preview blocks up to limit non-empty blocks.
pub fn parse_preview_blocks(text: &str, limit: usize, drop_comments: bool) -> Vec<Block> {
    let preprocessed = preprocess_asciidoc(text, drop_comments);
    let lines: Vec<&str> = preprocessed.iter().map(|s| s.as_str()).collect();
    let all = parse_blocks_from_lines(&lines);
    all.into_iter().filter(|b| !matches!(b, Block::EmptyLine)).take(limit).collect()
}

/// Parse AsciiDoc text into a list of blocks with configurable comment dropping. Guaranteed not to panic.
pub fn parse_blocks_with_options(text: &str, drop_comments: bool) -> Vec<Block> {
    // Top-level panic guard — prevents UB when called across FFI boundary (e.g. qt_method)
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parse_blocks_inner(text, drop_comments)
    })) {
        Ok(blocks) => blocks,
        Err(e) => {
            log::warn!("parse_blocks panicked: {:?}", e);
            vec![]
        }
    }
}

fn parse_blocks_inner(text: &str, drop_comments: bool) -> Vec<Block> {
    let preprocessed = preprocess_asciidoc(text, drop_comments);
    let lines: Vec<&str> = preprocessed.iter().map(|s| s.as_str()).collect();
    parse_blocks_from_lines(&lines)
}

pub fn parse_blocks_from_lines(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Phase 1: Simple single-line blocks
        if let Some(block) = try_parse_simple_block(lines[i]) {
            blocks.push(block);
            i += 1;
            continue;
        }

        // Non-toc document attributes: skip silently (e.g. :source-highlighter: python)
        if is_doc_attribute(lines[i].trim()) {
            i += 1;
            continue;
        }

        // Phase 2: Blockquote (> text) — collect consecutive > lines
        if let Some((block, consumed)) = collect_blockquote(lines, i) {
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Phase 3: Collect block title (.Title) and attribute lines ([source,...], [NOTE], etc.)
        let (title, attr_lines, cur) = collect_title_and_attrs(lines, i);

        // Phase 4: Block type handlers
        if let Some((block, new_i)) =
            try_parse_block(lines, i, cur, title, &attr_lines, &blocks)
        {
            blocks.push(block);
            i = new_i;
            continue;
        }

        // Fallback: normal paragraph
        let (mut block, consumed) = parse_paragraph_with_attrs(&lines[cur..], &attr_lines);
        if cur > i {
            let prefix_raw = lines[i..cur].join("\n");
            if let Block::Paragraph { ref mut raw, .. } = block {
                *raw = format!("{}\n{}", prefix_raw, raw);
            }
        }
        blocks.push(block);
        i = cur + consumed;
    }

    blocks
}

/// Phase 1: Try to parse a simple single-line block from the current line.
/// Handles: empty lines, comments, toc directives, page breaks, horizontal rules, and headings.
fn try_parse_simple_block(line: &str) -> Option<Block> {
    let trimmed = line.trim();

    // Empty line
    if trimmed.is_empty() {
        return Some(Block::EmptyLine);
    }

    // Comment line: // ...
    if trimmed.starts_with("//") {
        return Some(Block::Comment {
            text: trimmed[2..].trim().to_string(),
            raw: line.to_string(),
        });
    }

    // TOC: :toc:, :toc:N, toc::[], toc::[...]
    if trimmed == ":toc:" || trimmed.starts_with(":toc:") {
        let depth = if trimmed == ":toc:" {
            None
        } else {
            trimmed
                .strip_prefix(":toc:")
                .unwrap()
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|&d| d >= 1 && d <= 5)
        };
        return Some(Block::Toc {
            raw: trimmed.to_string(),
            depth,
        });
    }
    if trimmed == "toc::[]" || trimmed.starts_with("toc::[") {
        let depth =
            if let Some(inner) = trimmed.strip_prefix("toc::[").and_then(|s| s.strip_suffix(']'))
            {
                let arg = inner.trim();
                if let Some(val) = arg.strip_prefix("levels=") {
                    val.trim().parse::<u8>().ok().filter(|&d| d >= 1 && d <= 5)
                } else {
                    arg.parse::<u8>().ok().filter(|&d| d >= 1 && d <= 5)
                }
            } else {
                None
            };
        return Some(Block::Toc {
            raw: trimmed.to_string(),
            depth,
        });
    }

    // Page break: <<<
    if trimmed == "<<<" {
        return Some(Block::PageBreak {
            raw: line.to_string(),
        });
    }

    // Horizontal rule: ---, ***, ___ (3+ repeated chars)
    if is_horizontal_rule(line) {
        return Some(Block::HorizontalRule {
            raw: line.to_string(),
        });
    }

    // Heading: = to ======
    if let Some((level, rest)) = parse_heading(line) {
        let spans = parse_inline(rest.trim());
        return Some(Block::Heading {
            level,
            spans,
            raw: line.to_string(),
        });
    }

    None
}

/// Phase 2: Collect consecutive blockquote lines (> text) starting at position `i`.
/// Returns the blockquote block and the number of lines consumed, or `None` if
/// the line at `i` doesn't start with `>`.
fn collect_blockquote(lines: &[&str], i: usize) -> Option<(Block, usize)> {
    if !lines[i].starts_with('>') {
        return None;
    }
    let mut bq_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut j = i;
    while j < lines.len() && lines[j].starts_with('>') {
        let text = lines[j].strip_prefix('>').unwrap_or(lines[j]).trim();
        bq_lines.push(text.to_string());
        raw_lines.push(lines[j].to_string());
        j += 1;
    }
    let content = bq_lines.join("\n");
    let children = parse_blocks(&content);
    Some((
        Block::Blockquote {
            title: None,
            attribution: None,
            citation: None,
            children,
            raw: raw_lines.join("\n"),
        },
        j - i,
    ))
}

/// Phase 3: Scan for an optional block title (.Title) and attribute lines ([...])
/// starting at position `i`. Returns the title, attribute lines, and the position
/// of the first content line (past title and attributes).
fn collect_title_and_attrs<'a>(lines: &'a [&str], i: usize) -> (Option<String>, Vec<&'a str>, usize) {
    let mut cur = i;
    let mut title: Option<String> = None;
    let mut attr_lines: Vec<&str> = Vec::new();

    // Check for block title (.Title)
    if cur < lines.len() {
        let l = lines[cur].trim();
        if l.starts_with('.')
            && !l.starts_with(". ")
            && !l.starts_with("..")
            && !l.chars().skip(1).all(|c| c.is_ascii_digit())
        {
            title = Some(l[1..].trim().to_string());
            cur += 1;
        }
    }

    // Collect attribute lines ([source,...], [NOTE], etc.)
    while cur < lines.len() {
        let l = lines[cur].trim();
        if is_attribute_line(l) {
            attr_lines.push(lines[cur]);
            cur += 1;
        } else {
            break;
        }
    }

    (title, attr_lines, cur)
}

/// Phase 4: Try to parse a block at position `cur` (after title/attributes).
/// `i` is the start of the current block (before title/attrs).
/// Returns `Some((block, new_i))` if a block type matched, or `None` to fall
/// through to the paragraph handler.
fn try_parse_block(
    lines: &[&str],
    i: usize,
    cur: usize,
    title: Option<String>,
    attr_lines: &[&str],
    blocks: &[Block],
) -> Option<(Block, usize)> {
    if cur >= lines.len() {
        return None;
    }

    let next = lines[cur].trim();
    // attr_start is where attribute lines begin (after optional title)
    let attr_start = cur - attr_lines.len();

    // Helper: compute the prefix raw from title/attr lines (if any)
    let prefix_raw = |up_to: usize| -> Option<String> {
        if up_to > i {
            Some(lines[i..up_to].join("\n"))
        } else {
            None
        }
    };

    // Table: |=== or line starting with |
    if is_table_delimiter(next) || next.starts_with('|') {
        let (mut block, consumed) = parse_table(&lines[attr_start..], title);
        if let Some(prefix) = prefix_raw(attr_start) {
            block.prepend_raw(&prefix);
        }
        return Some((block, attr_start + consumed));
    }

    // Code block: ----
    if is_code_delimiter(next) {
        let lang = attr_lines.first().and_then(|a| parse_source_lang(a.trim()));
        let (mut block, consumed) =
            parse_delimited_block(lines[cur], &lines[cur..], "----", lang, title);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Literal block: ....
    if is_literal_delimiter(next) {
        let (mut block, consumed) =
            parse_delimited_block(lines[cur], &lines[cur..], "....", None, title);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Sidebar: ****
    if is_sidebar_delimiter(next) {
        let (mut block, consumed) = parse_sidebar_block(&lines[cur..], title);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Admonition block or paragraph: [NOTE] / [NOTE%unbreakable] / [WARNING]
    let admonition_attr = attr_lines.iter().find(|a| is_admonition_kind(a));
    if admonition_attr.is_some()
        || (is_admonition_kind(next)
            && cur + 1 < lines.len()
            && is_example_delimiter(lines[cur + 1].trim()))
    {
        let kind = if let Some(a) = admonition_attr {
            extract_admonition_kind(a)
        } else {
            extract_admonition_kind(next)
        };

        if is_example_delimiter(next)
            || (is_admonition_kind(next)
                && cur + 1 < lines.len()
                && is_example_delimiter(lines[cur + 1].trim()))
        {
            let (mut block, consumed) = parse_admonition_block(kind, &lines[cur..], title);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        } else {
            let (mut block, consumed) = parse_admonition_paragraph(kind, &lines[cur..], title);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        }
    }

    // Example block: ====
    if is_example_delimiter(next) {
        let (mut block, consumed) = parse_example_block(&lines[cur..], title);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Check quote / verse attributes
    let (is_quote_attr, is_verse_attr, attr_attribution, attr_citation) = attr_lines
        .iter()
        .find_map(|a| {
            let (q, v, attr, cit) = parse_quote_or_verse_attr(a);
            if q || v {
                Some((q, v, attr, cit))
            } else {
                None
            }
        })
        .unwrap_or((false, false, None, None));

    // Verse block: [verse,...] followed by ____ or verse paragraph
    if is_verse_attr {
        if is_quote_delimiter(next) {
            let (mut block, consumed) =
                parse_verse_block(&lines[cur..], title, attr_attribution, attr_citation);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        } else {
            let (mut block, consumed) =
                parse_verse_paragraph(&lines[cur..], title, attr_attribution, attr_citation);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        }
    }

    // Quote block with attribute: [quote,...]
    if is_quote_attr {
        if is_quote_delimiter(next) {
            let (mut block, consumed) =
                parse_quote_block(&lines[cur..], title, attr_attribution, attr_citation);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        } else {
            let (mut block, consumed) =
                parse_quote_paragraph(&lines[cur..], title, attr_attribution, attr_citation);
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + consumed));
        }
    }

    // Quote block: ____
    if is_quote_delimiter(next) {
        let (mut block, consumed) = parse_quote_block(&lines[cur..], title, None, None);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Open block: --
    if is_open_delimiter(next) {
        let (mut block, consumed) = parse_open_block(&lines[cur..], title);
        if let Some(prefix) = prefix_raw(cur) {
            block.prepend_raw(&prefix);
        }
        return Some((block, cur + consumed));
    }

    // Block image: image::path.png[Alt text, width=300]
    if next.starts_with("image::") {
        if let Some(mut block) = parse_image_block(lines[cur], title) {
            if let Some(prefix) = prefix_raw(cur) {
                block.prepend_raw(&prefix);
            }
            return Some((block, cur + 1));
        }
    }

    // Heading: = ... (after title/attrs — no raw prefix prepending)
    if is_heading(next) {
        if let Some((level, rest)) = parse_heading(next) {
            let spans = parse_inline(rest.trim());
            return Some((
                Block::Heading {
                    level,
                    spans,
                    raw: lines[cur].to_string(),
                },
                cur + 1,
            ));
        }
    }

    // Ordered list: 1. ... or . ...
    if let Some((mut marker, level, rest)) = parse_ordered_list_item(next) {
        let is_dot = next.trim_start().starts_with('.');
        let (start_num, mut is_reversed, numbering_style) =
            parse_ordered_list_attributes(attr_lines);

        let mut prev_num = 0;
        let mut prev_reversed = false;
        for prev in blocks.iter().rev() {
            if let Block::EmptyLine = prev {
                continue;
            }
            if let Block::OrderedListItem {
                level: pl,
                marker: pm,
                reversed: pr,
                ..
            } = prev
            {
                if *pl < level {
                    break;
                }
                if *pl == level {
                    prev_num = parse_marker_num(pm, level);
                    prev_reversed = *pr;
                    break;
                }
            } else {
                break;
            }
        }

        if attr_lines.is_empty() && prev_reversed {
            is_reversed = true;
        }

        if is_dot || is_reversed || start_num.is_some() || numbering_style.is_some() {
            let next_num = if let Some(sn) = start_num {
                sn
            } else if is_reversed && prev_num == 0 {
                count_ordered_list_items(&lines[cur..], level)
            } else if is_reversed {
                prev_num.saturating_sub(1).max(1)
            } else if prev_num == 0 {
                1
            } else {
                prev_num + 1
            };
            let effective_level = numbering_style.unwrap_or(level);
            marker = format_ordered_marker(effective_level, next_num);
        }
        let (children, child_consumed) =
            parse_list_item_children(rest.trim(), &lines[cur + 1..]);
        let raw_lines = if child_consumed > 1 {
            lines[i..cur + child_consumed].join("\n")
        } else {
            lines[i..=cur].join("\n")
        };
        return Some((
            Block::OrderedListItem {
                level,
                marker,
                reversed: is_reversed,
                children,
                raw: raw_lines,
            },
            cur + child_consumed,
        ));
    }

    // Unordered list: - or *
    if let Some((marker, level, rest)) = parse_unordered_list_item(next) {
        let (checked, item_text) = parse_checkbox(rest);
        let (children, child_consumed) =
            parse_list_item_children(item_text.trim(), &lines[cur + 1..]);
        let raw_lines = if child_consumed > 1 {
            lines[i..cur + child_consumed].join("\n")
        } else {
            lines[i..=cur].join("\n")
        };
        return Some((
            Block::UnorderedListItem {
                level,
                marker,
                checked,
                children,
                raw: raw_lines,
            },
            cur + child_consumed,
        ));
    }

    // Description list: Term:: Definition
    if let Some((block, consumed)) = parse_description_list_item(next, &lines[cur + 1..]) {
        return Some((block, cur + consumed));
    }

    // Callout list item: <1> explanation
    if let Some((block, consumed)) = parse_callout_list_item(next, &lines[cur + 1..]) {
        return Some((block, cur + consumed));
    }

    None
}

pub fn parse_paragraph(lines: &[&str]) -> (Block, usize) {
    parse_paragraph_with_attrs(lines, &[])
}

pub fn parse_paragraph_with_attrs(lines: &[&str], attr_lines: &[&str]) -> (Block, usize) {
    let has_hardbreaks = attr_lines.iter().any(|a| {
        let t = a.trim();
        t == "[%hardbreaks]"
            || t == "[hardbreaks]"
            || t == "[options=\"hardbreaks\"]"
            || t == "[options='hardbreaks']"
            || t == "[opts=hardbreaks]"
            || t == "[opts=\"hardbreaks\"]"
    });

    let mut text_lines = Vec::new();
    let mut consumed = 0;

    while consumed < lines.len() {
        let line = lines[consumed];
        if line.trim().is_empty() {
            break;
        }
        if consumed > 0 && is_attribute_line(line.trim()) {
            break;
        }
        // Stop if we hit a special line
        if is_heading(line)
            || is_horizontal_rule(line)
            || is_delimiter(line)
            || is_doc_attribute(line.trim())
            || line.trim() == "<<<"
            || line.starts_with('>')
            || line.trim_start().starts_with('|')
            || parse_ordered_list_item(line).is_some()
            || parse_unordered_list_item(line).is_some()
        {
            if consumed == 0 {
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

    let spans = if has_hardbreaks {
        let mut all_spans = Vec::new();
        for (idx, line) in text_lines.iter().enumerate() {
            if idx > 0 {
                all_spans.push(crate::inline::InlineSpan::Pass("<br/>".to_string()));
            }
            all_spans.extend(parse_inline(line.trim_end()));
        }
        all_spans
    } else {
        let has_line_breaks = text_lines.iter().any(|l| l.trim_end().ends_with(" +"));
        if has_line_breaks {
            let mut all_spans = Vec::new();
            for (idx, line) in text_lines.iter().enumerate() {
                let trimmed = line.trim_end();
                if idx > 0 {
                    all_spans.push(crate::inline::InlineSpan::Text(" ".to_string()));
                }
                if let Some(stripped) = trimmed.strip_suffix(" +") {
                    all_spans.extend(parse_inline(stripped.trim_end()));
                    all_spans.push(crate::inline::InlineSpan::Pass("<br/>".to_string()));
                } else {
                    all_spans.extend(parse_inline(trimmed));
                }
            }
            all_spans
        } else {
            let full_text = text_lines.join(" ");
            parse_inline(&full_text)
        }
    };

    (Block::Paragraph { spans, raw }, consumed)
}

/// Serialize blocks back to AsciiDoc text.
pub fn blocks_to_adoc(blocks: &[Block]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        parts.push(block.raw_text().to_string());
    }
    parts.join("\n")
}
