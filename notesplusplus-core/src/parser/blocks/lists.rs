use crate::block::Block;
use crate::inline::parse_inline;
use crate::parser::attributes::{is_attribute_line, is_doc_attribute};
use crate::parser::blocks::headings::{is_heading, parse_heading};
use crate::parser::blocks::quotes::{parse_admonition_block, parse_quote_block};
use crate::parser::blocks::tables::parse_table;
use crate::parser::blocks::verbatim::{
    parse_delimited_block, parse_example_block, parse_image_block, parse_open_block,
    parse_sidebar_block,
};
use crate::parser::delimiters::{
    is_admonition_kind, is_code_delimiter, is_delimiter, is_example_delimiter, is_horizontal_rule,
    is_literal_delimiter, is_open_delimiter, is_quote_delimiter, is_sidebar_delimiter,
    is_table_delimiter,
};
use crate::parser::parse_blocks;

pub fn parse_marker_num(marker: &str, level: u8) -> usize {
    let clean = marker.trim_end_matches('.');
    if let Ok(n) = clean.parse::<usize>() {
        return n;
    }
    match level % 5 {
        0 => clean.parse::<usize>().unwrap_or(0),
        1 => {
            if let Some(ch) = clean.chars().next() {
                if ch.is_ascii_lowercase() {
                    return (ch as usize) - ('a' as usize) + 1;
                }
            }
            clean.parse::<usize>().unwrap_or(0)
        }
        2 => {
            from_roman(&clean.to_uppercase()).unwrap_or_else(|| clean.parse::<usize>().unwrap_or(0))
        }
        3 => {
            if let Some(ch) = clean.chars().next() {
                if ch.is_ascii_uppercase() {
                    return (ch as usize) - ('A' as usize) + 1;
                }
            }
            clean.parse::<usize>().unwrap_or(0)
        }
        4 => from_roman(clean).unwrap_or_else(|| clean.parse::<usize>().unwrap_or(0)),
        _ => clean.parse::<usize>().unwrap_or(0),
    }
}

pub fn to_roman(mut num: usize) -> String {
    if num == 0 {
        return "0".to_string();
    }
    let vals = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut res = String::new();
    for &(val, sym) in &vals {
        while num >= val {
            res.push_str(sym);
            num -= val;
        }
    }
    res
}

pub fn from_roman(s: &str) -> Option<usize> {
    let mut total = 0;
    let mut prev = 0;
    for ch in s.chars().rev() {
        let val = match ch {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => return None,
        };
        if val < prev {
            total -= val;
        } else {
            total += val;
            prev = val;
        }
    }
    if total > 0 {
        Some(total)
    } else {
        None
    }
}

pub fn format_ordered_marker(level: u8, num: usize) -> String {
    match level % 5 {
        0 => format!("{}.", num),
        1 => {
            if (1..=26).contains(&num) {
                let ch = ((num - 1) as u8 + b'a') as char;
                format!("{}.", ch)
            } else {
                format!("{}.", num)
            }
        }
        2 => {
            format!("{}.", to_roman(num).to_lowercase())
        }
        3 => {
            if (1..=26).contains(&num) {
                let ch = ((num - 1) as u8 + b'A') as char;
                format!("{}.", ch)
            } else {
                format!("{}.", num)
            }
        }
        4 => {
            format!("{}.", to_roman(num))
        }
        _ => format!("{}.", num),
    }
}

pub fn parse_callout_list_item(line: &str, next_lines: &[&str]) -> Option<(Block, usize)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('<') {
        return None;
    }
    if let Some(close) = trimmed.find('>') {
        let num_str = &trimmed[1..close];
        if let Ok(num) = num_str.parse::<usize>() {
            let rest = trimmed[close + 1..].trim();
            let mut raw_lines = vec![line.to_string()];
            let mut consumed = 1;
            let mut item_lines = Vec::new();
            if !rest.is_empty() {
                item_lines.push(rest.to_string());
            }
            let mut j = 0;
            while j < next_lines.len() {
                let next_line = next_lines[j];
                if next_line.trim().is_empty()
                    || is_delimiter(next_line)
                    || is_heading(next_line)
                    || is_horizontal_rule(next_line)
                {
                    break;
                }
                if next_line.trim().starts_with('<') && next_line.trim().contains('>') {
                    break;
                }
                item_lines.push(next_line.trim().to_string());
                raw_lines.push(next_line.to_string());
                consumed += 1;
                j += 1;
            }
            let children = if item_lines.is_empty() {
                Vec::new()
            } else {
                parse_blocks(&item_lines.join("\n"))
            };
            return Some((
                Block::CalloutListItem {
                    number: num,
                    children,
                    raw: raw_lines.join("\n"),
                },
                consumed,
            ));
        }
    }
    None
}

pub fn parse_description_list_item(line: &str, next_lines: &[&str]) -> Option<(Block, usize)> {
    let trimmed = line.trim();
    if trimmed.starts_with("image::") || trimmed.starts_with(':') {
        return None;
    }
    let dcol_pos = trimmed.find("::")?;
    if dcol_pos == 0 {
        return None;
    }

    let term = trimmed[..dcol_pos].trim().to_string();
    if term.is_empty() {
        return None;
    }
    let term_spans = parse_inline(&term);

    let rest = trimmed[dcol_pos + 2..].trim();
    let mut raw_lines = vec![line.to_string()];
    let mut consumed = 1;
    let mut desc_lines = Vec::new();
    if !rest.is_empty() {
        desc_lines.push(rest.to_string());
    }

    let mut j = 0;
    while j < next_lines.len() {
        let next_line = next_lines[j];
        if next_line.trim().is_empty()
            || is_delimiter(next_line)
            || is_heading(next_line)
            || is_horizontal_rule(next_line)
        {
            break;
        }
        if next_line.trim().contains("::")
            && !next_line.trim().starts_with("image::")
            && !next_line.trim().starts_with(':')
        {
            break;
        }
        desc_lines.push(next_line.trim().to_string());
        raw_lines.push(next_line.to_string());
        consumed += 1;
        j += 1;
    }
    let children = if desc_lines.is_empty() {
        Vec::new()
    } else {
        parse_blocks(&desc_lines.join("\n"))
    };
    Some((
        Block::DescriptionListItem {
            term,
            term_spans,
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    ))
}

pub fn parse_ordered_list_item(line: &str) -> Option<(String, u8, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let level = (indent / 2) as u8;

    if trimmed.starts_with('.') {
        let dot_count = trimmed.chars().take_while(|&c| c == '.').count();
        if dot_count <= 5 && trimmed[dot_count..].starts_with(' ') {
            let marker = ".".to_string();
            let dot_level = (dot_count - 1) as u8 + level;
            let rest = trimmed[dot_count..].trim_start();
            return Some((marker, dot_level, rest));
        }
    }

    // Match e.g. "1. ", "12. ", "a. ", "iv. "
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        if !prefix.is_empty()
            && (prefix.chars().all(|c| c.is_ascii_digit())
                || (prefix.len() <= 4 && prefix.chars().all(|c| c.is_ascii_alphabetic())))
        {
            let marker = format!("{}.", prefix);
            let rest = &trimmed[dot_pos + 2..];
            return Some((marker, level, rest));
        }
    }

    None
}

pub fn count_ordered_list_items(lines: &[&str], target_level: u8) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next_line = lines[j];
                if let Some((_, lvl, _)) = parse_ordered_list_item(next_line) {
                    if lvl < target_level {
                        break;
                    }
                    i = j;
                    continue;
                } else if next_line.starts_with("  ") {
                    i = j + 1;
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if trimmed.starts_with("//") {
            i += 1;
            continue;
        }

        if is_delimiter(line)
            || is_heading(line)
            || is_horizontal_rule(line)
            || is_doc_attribute(trimmed)
            || is_attribute_line(trimmed)
        {
            break;
        }

        if let Some((_, lvl, _)) = parse_ordered_list_item(line) {
            if lvl == target_level {
                count += 1;
            } else if lvl < target_level {
                break;
            }
            i += 1;
            continue;
        }

        if parse_unordered_list_item(line).is_some()
            || parse_description_list_item(line, &[]).is_some()
            || parse_callout_list_item(line, &[]).is_some()
        {
            break;
        }

        if line.starts_with("  ") || line.starts_with('+') {
            i += 1;
            continue;
        }

        break;
    }
    count.max(1)
}

pub fn parse_unordered_list_item(line: &str) -> Option<(String, u8, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let space_level = (indent / 2) as u8;

    if let Some(rest) = trimmed.strip_prefix("- ") {
        return Some(("-".to_string(), space_level, rest));
    } else if trimmed.starts_with('*') {
        let star_count = trimmed.chars().take_while(|&c| c == '*').count();
        if star_count <= 5 && trimmed.as_bytes().get(star_count) == Some(&b' ') {
            let marker = "*".to_string();
            let rest = &trimmed[star_count + 1..];
            let star_level = (star_count - 1) as u8;
            return Some((marker, star_level.max(space_level), rest));
        }
    }

    None
}

pub fn parse_checkbox(text: &str) -> (Option<bool>, &str) {
    let t = text.trim_start();
    if let Some(stripped) = t.strip_prefix("[ ] ") {
        (Some(false), stripped)
    } else if let Some(stripped) = t.strip_prefix("[x] ") {
        (Some(true), stripped)
    } else if let Some(stripped) = t.strip_prefix("[X] ") {
        (Some(true), stripped)
    } else {
        (None, text)
    }
}

pub fn parse_list_item_children(first_line_text: &str, remaining: &[&str]) -> (Vec<Block>, usize) {
    let mut children = Vec::new();

    // First child: the marker text as a paragraph
    let spans = parse_inline(first_line_text);
    if !spans.is_empty() {
        children.push(Block::Paragraph {
            spans,
            raw: first_line_text.to_string(),
        });
    }

    // Collect indented continuation lines
    let mut cont_lines: Vec<String> = Vec::new();
    let mut consumed = 0;

    while consumed < remaining.len() {
        let line = remaining[consumed];
        // An indented line (2+ spaces) is continuation
        if line.len() >= 2 && line.starts_with("  ") {
            cont_lines.push(line[2..].to_string()); // strip 2-space indent
            consumed += 1;
        } else if line.trim().is_empty() && consumed + 1 < remaining.len() {
            // Blank line — check if next non-blank line is still indented
            let mut peek = consumed + 1;
            while peek < remaining.len() && remaining[peek].trim().is_empty() {
                peek += 1;
            }
            if peek < remaining.len()
                && remaining[peek].len() >= 2
                && remaining[peek].starts_with("  ")
            {
                // Include blank line and continue
                cont_lines.push(String::new());
                consumed += 1;
            } else {
                break; // end of continuation
            }
        } else {
            break; // not indented, end of list item
        }
    }

    // Parse collected continuation lines as blocks and adjust nesting levels
    if !cont_lines.is_empty() {
        let content = cont_lines.join("\n");
        let mut cont_blocks = parse_blocks(&content);
        // Continuation content was stripped of 2-space indent — bump all list items by 1
        bump_list_levels(&mut cont_blocks, 1);
        children.append(&mut cont_blocks);
    }

    (children, consumed + 1) // +1 for the marker line itself
}

pub fn bump_list_levels(blocks: &mut [Block], amount: u8) {
    for block in blocks {
        match block {
            Block::OrderedListItem {
                level, children, ..
            } => {
                *level += amount;
                bump_list_levels(children, amount);
            }
            Block::UnorderedListItem {
                level, children, ..
            } => {
                *level += amount;
                bump_list_levels(children, amount);
            }
            _ => {}
        }
    }
}

pub fn toggle_line_checkbox_marker(line: &str) -> Option<String> {
    if line.contains("[ ] ") {
        Some(line.replacen("[ ] ", "[x] ", 1))
    } else if line.contains("[x] ") {
        Some(line.replacen("[x] ", "[ ] ", 1))
    } else if line.contains("[X] ") {
        Some(line.replacen("[X] ", "[ ] ", 1))
    } else {
        None
    }
}

fn consume_non_list_block(lines: &[String], line_strs: &[&str], i: usize) -> Option<usize> {
    let line = &lines[i];

    // Title line before sidebar, example, code block, or table: .Title
    if line.starts_with('.')
        && !line.starts_with(". ")
        && !line.starts_with("..")
        && i + 1 < lines.len()
    {
        let next = lines[i + 1].trim();
        if is_sidebar_delimiter(next) {
            let (_, consumed) = parse_sidebar_block(&line_strs[i + 1..], None);
            return Some(1 + consumed);
        }
        if is_example_delimiter(next) && !is_admonition_kind(line.trim()) {
            let (_, consumed) = parse_example_block(&line_strs[i + 1..], None);
            return Some(1 + consumed);
        }
    }

    // Attribute line before table or code block
    if is_attribute_line(line.trim()) {
        if let Some(next) = lines[i + 1..].iter().find(|l| !l.trim().is_empty()).map(|l| l.trim()) {
            if is_table_delimiter(next) {
                let (_, consumed) = parse_table(&line_strs[i..], None);
                return Some(consumed);
            }
            if is_code_delimiter(next) && i + 1 < lines.len() {
                let (_, consumed) =
                    parse_delimited_block(lines[i + 1].as_str(), &line_strs[i + 1..], "----", None, None);
                return Some(1 + consumed);
            }
            if is_sidebar_delimiter(next) && i + 1 < lines.len() {
                let (_, consumed) = parse_sidebar_block(&line_strs[i + 1..], None);
                return Some(1 + consumed);
            }
            if is_example_delimiter(next) && !is_admonition_kind(line.trim()) && i + 1 < lines.len() {
                let (_, consumed) = parse_example_block(&line_strs[i + 1..], None);
                return Some(1 + consumed);
            }
        }
    }

    // Sidebar block
    if is_sidebar_delimiter(line.trim()) {
        let (_, consumed) = parse_sidebar_block(&line_strs[i..], None);
        return Some(consumed);
    }

    // Example block
    if is_example_delimiter(line.trim()) {
        let (_, consumed) = parse_example_block(&line_strs[i..], None);
        return Some(consumed);
    }

    // Code block
    if is_code_delimiter(line.trim()) {
        let (_, consumed) =
            parse_delimited_block(line.as_str(), &line_strs[i..], "----", None, None);
        return Some(consumed);
    }

    // Literal block
    if is_literal_delimiter(line.trim()) {
        let (_, consumed) =
            parse_delimited_block(line.as_str(), &line_strs[i..], "....", None, None);
        return Some(consumed);
    }

    // Block image
    if line.trim().starts_with("image::") && parse_image_block(line, None).is_some() {
        return Some(1);
    }

    // Admonition block
    if is_admonition_kind(line.trim())
        && i + 1 < lines.len()
        && is_example_delimiter(lines[i + 1].trim())
    {
        let kind = line
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        let (_, consumed) = parse_admonition_block(&kind, &line_strs[i..], None);
        return Some(consumed);
    }

    // Horizontal rule
    if is_horizontal_rule(line) {
        return Some(1);
    }

    // Heading
    if parse_heading(line).is_some() {
        return Some(1);
    }

    // Blockquote
    if line.starts_with('>') {
        let mut j = i;
        while j < lines.len() && lines[j].starts_with('>') {
            j += 1;
        }
        return Some(j - i);
    }

    // Quote block
    if is_quote_delimiter(line.trim()) {
        let (_, consumed) = parse_quote_block(&line_strs[i..], None, None, None);
        return Some(consumed);
    }

    // Open block
    if is_open_delimiter(line.trim()) {
        let (_, consumed) = parse_open_block(&line_strs[i..], None);
        return Some(consumed);
    }

    // Table
    if line.trim_start().starts_with('|') {
        let (_, consumed) = parse_table(&line_strs[i..], None);
        return Some(consumed);
    }

    None
}

pub fn toggle_checkbox(
    content: &str,
    target_block_idx: usize,
    sub_path: &str,
) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut current_block_idx = 0;
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];

        // Empty line
        if line.trim().is_empty() {
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += 1;
            continue;
        }

        // Document attribute
        if is_doc_attribute(line.trim()) {
            let trimmed = line.trim();
            if trimmed == ":toc:" || trimmed.starts_with(":toc:") {
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
            }
            i += 1;
            continue;
        }

        // Non-list block types (headings, tables, quotes, code, admonitions, etc.)
        {
            let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            if let Some(consumed) = consume_non_list_block(&lines, &line_strs, i) {
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += consumed;
                continue;
            }
        }

        // Ordered list
        if parse_ordered_list_item(line).is_some() {
            let consumed = {
                let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                let (_, c) = parse_list_item_children("", &line_strs[i + 1..]);
                c
            };
            if current_block_idx == target_block_idx {
                if sub_path.is_empty() {
                    return None;
                }
                return toggle_nested_continuation_lines(
                    &mut lines,
                    i,
                    consumed,
                    sub_path,
                    content.ends_with('\n'),
                );
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Unordered list
        if let Some((_, _, rest)) = parse_unordered_list_item(line) {
            let (checked, item_text) = parse_checkbox(rest);
            let consumed = {
                let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                let (_, c) = parse_list_item_children(item_text.trim(), &line_strs[i + 1..]);
                c
            };
            if current_block_idx == target_block_idx {
                if sub_path.is_empty() {
                    if checked.is_some() {
                        let toggled = toggle_line_checkbox_marker(&lines[i])?;
                        lines[i] = toggled;
                        let mut result = lines.join("\n");
                        if content.ends_with('\n') {
                            result.push('\n');
                        }
                        return Some(result);
                    }
                    return None;
                } else {
                    return toggle_nested_continuation_lines(
                        &mut lines,
                        i,
                        consumed,
                        sub_path,
                        content.ends_with('\n'),
                    );
                }
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Callout list item
        {
            let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            if let Some((_, consumed)) = parse_callout_list_item(line, &line_strs[i + 1..]) {
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += consumed;
                continue;
            }
        }

        // Description list item
        {
            let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            if let Some((_, consumed)) = parse_description_list_item(line, &line_strs[i + 1..]) {
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += consumed;
                continue;
            }
        }

        // Paragraph
        let mut consumed_p = 0;
        let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        while consumed_p < line_strs[i..].len() {
            let pline = line_strs[i + consumed_p];
            if pline.trim().is_empty() {
                break;
            }
            if consumed_p > 0 && is_attribute_line(pline.trim()) {
                break;
            }
            if is_heading(pline)
                || is_horizontal_rule(pline)
                || is_delimiter(pline)
                || is_doc_attribute(pline.trim())
                || pline.trim() == "<<<"
                || pline.starts_with('>')
                || pline.trim_start().starts_with('|')
                || parse_ordered_list_item(pline).is_some()
                || parse_unordered_list_item(pline).is_some()
            {
                if consumed_p == 0 {
                    consumed_p += 1;
                }
                break;
            }
            consumed_p += 1;
        }
        if consumed_p == 0 && !line_strs[i..].is_empty() {
            consumed_p = 1;
        }

        if current_block_idx == target_block_idx {
            return None;
        }
        current_block_idx += 1;
        i += consumed_p;
    }

    None
}

pub fn toggle_nested_continuation_lines(
    lines: &mut [String],
    line_idx: usize,
    consumed: usize,
    sub_path: &str,
    ends_with_newline: bool,
) -> Option<String> {
    if consumed <= 1 {
        return None;
    }

    let (first_str, rest) = match sub_path.split_once('.') {
        Some((f, r)) => (f, r),
        None => (sub_path, ""),
    };
    let child_idx: usize = first_str.parse().ok()?;
    // child_idx 0 is paragraph text of the item itself; continuation blocks start at index 1 -> cont_block_idx = child_idx - 1
    let cont_block_idx = child_idx.checked_sub(1)?;

    let remaining: Vec<String> = lines[line_idx + 1..line_idx + consumed].to_vec();
    let mut cont_lines: Vec<String> = Vec::new();
    let mut indent_prefixes: Vec<String> = Vec::new();

    for rem_line in &remaining {
        if rem_line.len() >= 2 && rem_line.starts_with("  ") {
            indent_prefixes.push(rem_line[..2].to_string());
            cont_lines.push(rem_line[2..].to_string());
        } else if rem_line.trim().is_empty() {
            indent_prefixes.push(String::new());
            cont_lines.push(String::new());
        } else {
            indent_prefixes.push(String::new());
            cont_lines.push(rem_line.clone());
        }
    }

    let cont_content = cont_lines.join("\n");
    let toggled_cont = toggle_checkbox(&cont_content, cont_block_idx, rest)?;
    let new_cont_lines: Vec<&str> = toggled_cont.lines().collect();

    if new_cont_lines.len() != cont_lines.len() {
        return None;
    }

    for k in 0..new_cont_lines.len() {
        if indent_prefixes[k].is_empty() && new_cont_lines[k].is_empty() {
            lines[line_idx + 1 + k] = String::new();
        } else {
            lines[line_idx + 1 + k] =
                format!("{}{}", indent_prefixes[k], new_cont_lines[k]);
        }
    }

    let mut result = lines.join("\n");
    if ends_with_newline {
        result.push('\n');
    }
    Some(result)
}
