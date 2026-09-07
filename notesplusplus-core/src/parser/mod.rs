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
        let line = lines[i];

        // Empty line
        if line.trim().is_empty() {
            blocks.push(Block::EmptyLine);
            i += 1;
            continue;
        }

        // Comment line: // ...
        if line.trim().starts_with("//") {
            let comment_text = line.trim()[2..].trim().to_string();
            blocks.push(Block::Comment {
                text: comment_text,
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Document attribute or block macro: :toc:, toc::[], :source-highlighter:, etc.
        if is_doc_attribute(line.trim()) || line.trim() == "toc::[]" || line.trim().starts_with("toc::[") {
            let trimmed = line.trim();
            if trimmed == ":toc:" || trimmed.starts_with(":toc:") || trimmed == "toc::[]" || trimmed.starts_with("toc::[") {
                blocks.push(Block::Toc {
                    raw: trimmed.to_string(),
                });
            }
            i += 1;
            continue;
        }

        // Page break: <<<
        if line.trim() == "<<<" {
            blocks.push(Block::PageBreak {
                raw: line.to_string(),
            });
            i += 1;
            continue;
        }

        // Horizontal rule: ---, ***, ___ (3+ repeated chars)
        if is_horizontal_rule(line) {
            blocks.push(Block::HorizontalRule {
                raw: line.to_string(),
            });
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

        // Blockquote: > text — collect consecutive > lines
        if line.starts_with('>') {
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
            blocks.push(Block::Blockquote {
                title: None,
                attribution: None,
                citation: None,
                children,
                raw: raw_lines.join("\n"),
            });
            i = j;
            continue;
        }

        // Check for block title (.Title) or attribute lines ([source,...], [NOTE], etc.)
        // These can appear before code blocks, tables, images, sidebars, examples, open blocks, quotes, or admonitions.
        let mut cur = i;
        let mut title: Option<String> = None;
        let mut attr_lines: Vec<&str> = Vec::new();

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

        let attr_start = cur;
        while cur < lines.len() {
            let l = lines[cur].trim();
            if is_attribute_line(l) {
                attr_lines.push(lines[cur]);
                cur += 1;
            } else {
                break;
            }
        }

        if cur < lines.len() {
            let next = lines[cur].trim();

            // Table: |=== or line starting with |
            if is_table_delimiter(next) || next.starts_with('|') {
                let (mut block, consumed) = parse_table(&lines[attr_start..], title);
                if attr_start > i {
                    let prefix_raw = lines[i..attr_start].join("\n");
                    if let Block::Table { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = attr_start + consumed;
                continue;
            }

            // Code block: ----
            if is_code_delimiter(next) {
                let lang = attr_lines.first().and_then(|a| parse_source_lang(a.trim()));
                let (mut block, consumed) =
                    parse_delimited_block(lines[cur], &lines[cur..], "----", lang, title);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::CodeBlock { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
            }

            // Literal block: ....
            if is_literal_delimiter(next) {
                let (mut block, consumed) =
                    parse_delimited_block(lines[cur], &lines[cur..], "....", None, title);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::LiteralBlock { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
            }

            // Sidebar: ****
            if is_sidebar_delimiter(next) {
                let (mut block, consumed) = parse_sidebar_block(&lines[cur..], title);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::Sidebar { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
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
                    let (mut block, consumed) = parse_admonition_block(&kind, &lines[cur..], title);
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Admonition { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                } else {
                    let (mut block, consumed) =
                        parse_admonition_paragraph(&kind, &lines[cur..], title);
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Admonition { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                }
            }

            // Example block: ====
            if is_example_delimiter(next) {
                let (mut block, consumed) = parse_example_block(&lines[cur..], title);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::Example { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
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
                    let (mut block, consumed) = parse_verse_block(
                        &lines[cur..],
                        title,
                        attr_attribution,
                        attr_citation,
                    );
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Verse { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                } else {
                    let (mut block, consumed) = parse_verse_paragraph(
                        &lines[cur..],
                        title,
                        attr_attribution,
                        attr_citation,
                    );
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Verse { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                }
            }

            // Quote block with attribute: [quote,...]
            if is_quote_attr {
                if is_quote_delimiter(next) {
                    let (mut block, consumed) = parse_quote_block(
                        &lines[cur..],
                        title,
                        attr_attribution,
                        attr_citation,
                    );
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Blockquote { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                } else {
                    let (mut block, consumed) = parse_quote_paragraph(
                        &lines[cur..],
                        title,
                        attr_attribution,
                        attr_citation,
                    );
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Blockquote { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + consumed;
                    continue;
                }
            }

            // Quote block: ____
            if is_quote_delimiter(next) {
                let (mut block, consumed) =
                    parse_quote_block(&lines[cur..], title, None, None);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::Blockquote { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
            }

            // Open block: --
            if is_open_delimiter(next) {
                let (mut block, consumed) = parse_open_block(&lines[cur..], title);
                if cur > i {
                    let prefix_raw = lines[i..cur].join("\n");
                    if let Block::Open { ref mut raw, .. } = block {
                        *raw = format!("{}\n{}", prefix_raw, raw);
                    }
                }
                blocks.push(block);
                i = cur + consumed;
                continue;
            }

            // Block image: image::path.png[Alt text, width=300]
            if next.starts_with("image::") {
                if let Some(mut block) = parse_image_block(lines[cur], title) {
                    if cur > i {
                        let prefix_raw = lines[i..cur].join("\n");
                        if let Block::Image { ref mut raw, .. } = block {
                            *raw = format!("{}\n{}", prefix_raw, raw);
                        }
                    }
                    blocks.push(block);
                    i = cur + 1;
                    continue;
                }
            }

            // Heading: = ...
            if is_heading(next) {
                if let Some((level, rest)) = parse_heading(next) {
                    let spans = parse_inline(rest.trim());
                    blocks.push(Block::Heading {
                        level,
                        spans,
                        raw: lines[cur].to_string(),
                    });
                    i = cur + 1;
                    continue;
                }
            }

            // Ordered list: 1. ... or . ...
            if let Some((mut marker, level, rest)) = parse_ordered_list_item(next) {
                let is_dot = next.trim_start().starts_with('.');
                let (start_num, mut is_reversed, numbering_style) =
                    parse_ordered_list_attributes(&attr_lines);

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
                blocks.push(Block::OrderedListItem {
                    level,
                    marker,
                    reversed: is_reversed,
                    children,
                    raw: raw_lines,
                });
                i = cur + child_consumed;
                continue;
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
                blocks.push(Block::UnorderedListItem {
                    level,
                    marker,
                    checked,
                    children,
                    raw: raw_lines,
                });
                i = cur + child_consumed;
                continue;
            }

            // Description list: Term:: Definition
            if let Some((block, consumed)) =
                parse_description_list_item(next, &lines[cur + 1..])
            {
                blocks.push(block);
                i = cur + consumed;
                continue;
            }

            // Callout list item: <1> explanation
            if let Some((block, consumed)) = parse_callout_list_item(next, &lines[cur + 1..]) {
                blocks.push(block);
                i = cur + consumed;
                continue;
            }
        }

        // Normal paragraph
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
