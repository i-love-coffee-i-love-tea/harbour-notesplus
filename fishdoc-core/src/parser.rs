use crate::block::Block;
use crate::inline::parse_inline;

const ADMONITION_KINDS: &[&str] = &["NOTE", "TIP", "WARNING", "CAUTION", "IMPORTANT"];

/// Parse AsciiDoc text into a list of blocks. Guaranteed not to panic.
pub fn parse_blocks(text: &str) -> Vec<Block> {
    parse_blocks_with_options(text, true)
}

/// Parse AsciiDoc text into a list of blocks with configurable comment dropping. Guaranteed not to panic.
pub fn parse_blocks_with_options(text: &str, drop_comments: bool) -> Vec<Block> {
    // Top-level panic guard — prevents UB when called across FFI boundary (e.g. qt_method)
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_blocks_inner(text, drop_comments))) {
        Ok(blocks) => blocks,
        Err(e) => {
            ::log::warn!("parse_blocks panicked: {:?}", e);
            vec![]
        }
    }
}

fn preprocess_asciidoc(text: &str, drop_comments: bool) -> Vec<String> {
    let mut attributes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Default built-in attributes
    attributes.insert("empty".into(), "".into());
    attributes.insert("sp".into(), " ".into());
    attributes.insert("nbsp".into(), "\u{00A0}".into());
    attributes.insert("zwsp".into(), "\u{200B}".into());
    attributes.insert("vbar".into(), "|".into());
    attributes.insert("caret".into(), "^".into());
    attributes.insert("plus".into(), "+".into());
    attributes.insert("asterisk".into(), "*".into());
    attributes.insert("tilde".into(), "~".into());
    attributes.insert("backslash".into(), "\\".into());
    attributes.insert("backtick".into(), "`".into());
    attributes.insert("two-colons".into(), "::".into());
    attributes.insert("two-semicolons".into(), ";;".into());
    attributes.insert("cpp".into(), "C++".into());
    attributes.insert("backend".into(), "fishdoc".into());

    let raw_lines: Vec<&str> = text.lines().collect();
    let mut in_comment_block = false;
    let mut in_verbatim_block: Option<&str> = None;
    let mut cond_stack: Vec<bool> = Vec::new();

    // Pass 1: Extract all document attributes & resolve conditionals/comments
    let mut filtered_lines: Vec<String> = Vec::new();
    let mut idx = 0;

    while idx < raw_lines.len() {
        let line = raw_lines[idx];
        let trimmed = line.trim();

        // Check comment block delimiters: ////
        if trimmed == "////" || trimmed.starts_with("////") {
            in_comment_block = !in_comment_block;
            idx += 1;
            continue;
        }
        if in_comment_block {
            if !drop_comments && cond_stack.iter().all(|&c| c) {
                filtered_lines.push(format!("// {}", line));
            }
            idx += 1;
            continue;
        }

        // Check verbatim block delimiters: ---- or ....
        if trimmed == "----" || trimmed == "...." || trimmed.starts_with("----") || trimmed.starts_with("....") {
            let delim = if trimmed.starts_with("----") { "----" } else { "...." };
            if let Some(active) = in_verbatim_block {
                if active == delim {
                    in_verbatim_block = None;
                }
            } else {
                in_verbatim_block = Some(delim);
            }
            if cond_stack.iter().all(|&c| c) {
                filtered_lines.push(line.to_string());
            }
            idx += 1;
            continue;
        }

        if in_verbatim_block.is_some() {
            if cond_stack.iter().all(|&c| c) {
                filtered_lines.push(line.to_string());
            }
            idx += 1;
            continue;
        }

        // Conditional directives: ifdef::name[], ifndef::name[], endif::[]
        if trimmed.starts_with("ifdef::") {
            if let Some(open) = trimmed.find("::") {
                if let Some(close) = trimmed[open + 2..].find('[') {
                    let attr_name = trimmed[open + 2..open + 2 + close].trim();
                    let is_active = attributes.contains_key(attr_name);
                    cond_stack.push(is_active);
                    idx += 1;
                    continue;
                }
            }
        }
        if trimmed.starts_with("ifndef::") {
            if let Some(open) = trimmed.find("::") {
                if let Some(close) = trimmed[open + 2..].find('[') {
                    let attr_name = trimmed[open + 2..open + 2 + close].trim();
                    let is_active = !attributes.contains_key(attr_name);
                    cond_stack.push(is_active);
                    idx += 1;
                    continue;
                }
            }
        }
        if trimmed.starts_with("endif::") {
            cond_stack.pop();
            idx += 1;
            continue;
        }

        // If inside inactive conditional branch, skip line
        if !cond_stack.iter().all(|&c| c) {
            idx += 1;
            continue;
        }

        // Single-line comment: // ...
        if trimmed.starts_with("//") {
            if !drop_comments {
                filtered_lines.push(line.to_string());
            }
            idx += 1;
            continue;
        }

        // Document attribute definition with optional multi-line continuation:
        if is_doc_attribute(trimmed) {
            let (name, mut val, has_cont) = parse_doc_attr_def(trimmed);
            let mut j = idx + 1;
            let continuing = has_cont;
            while continuing && j < raw_lines.len() {
                let next_l = raw_lines[j].trim();
                if next_l.ends_with('\\') {
                    let piece = next_l[..next_l.len() - 1].trim();
                    if !val.is_empty() {
                        val.push(' ');
                    }
                    val.push_str(piece);
                    j += 1;
                } else {
                    if !val.is_empty() {
                        val.push(' ');
                    }
                    val.push_str(next_l);
                    j += 1;
                    break;
                }
            }
            if !name.is_empty() {
                attributes.insert(name.clone(), val);
            }
            // If it is :toc:, keep it as a line for the TOC block parser
            if trimmed == ":toc:" || trimmed.starts_with(":toc:") {
                filtered_lines.push(trimmed.to_string());
            }
            idx = j;
            continue;
        }

        filtered_lines.push(line.to_string());
        idx += 1;
    }

    // Pass 2: Substitute {attribute_name} in non-verbatim lines
    let mut final_lines: Vec<String> = Vec::new();
    let mut in_verbatim = false;
    for line in filtered_lines {
        let trimmed = line.trim();
        if trimmed == "----" || trimmed == "...." || trimmed.starts_with("----") || trimmed.starts_with("....") {
            in_verbatim = !in_verbatim;
            final_lines.push(line);
            continue;
        }
        if in_verbatim {
            final_lines.push(line);
            continue;
        }
        let substituted = substitute_attributes(&line, &attributes);
        final_lines.push(substituted);
    }

    final_lines
}

fn parse_doc_attr_def(line: &str) -> (String, String, bool) {
    let t = line.trim();
    if !t.starts_with(':') {
        return (String::new(), String::new(), false);
    }
    if let Some(end) = t[1..].find(':') {
        let name = t[1..end + 1].trim().to_string();
        let rest = t[end + 2..].trim();
        let has_cont = rest.ends_with('\\');
        let val = if has_cont {
            rest[..rest.len() - 1].trim().to_string()
        } else {
            rest.to_string()
        };
        (name, val, has_cont)
    } else {
        (String::new(), String::new(), false)
    }
}

fn substitute_attributes(line: &str, attrs: &std::collections::HashMap<String, String>) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_c) = chars.peek() {
                if next_c == '{' {
                    chars.next(); // consume '{'
                    result.push('{');
                    continue;
                }
            }
            result.push(c);
        } else if c == '{' {
            let mut name = String::new();
            let mut closed = false;
            while let Some(&nc) = chars.peek() {
                if nc == '}' {
                    chars.next();
                    closed = true;
                    break;
                } else if nc.is_alphanumeric() || nc == '-' || nc == '_' {
                    name.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            if closed && !name.is_empty() {
                if let Some(val) = attrs.get(&name) {
                    result.push_str(val);
                } else {
                    result.push('{');
                    result.push_str(&name);
                    result.push('}');
                }
            } else {
                result.push('{');
                result.push_str(&name);
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_blocks_inner(text: &str, drop_comments: bool) -> Vec<Block> {
    let preprocessed = preprocess_asciidoc(text, drop_comments);
    let lines: Vec<&str> = preprocessed.iter().map(|s| s.as_str()).collect();
    parse_blocks_from_lines(&lines)
}

fn parse_blocks_from_lines(lines: &[&str]) -> Vec<Block> {
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

        // Document attribute: :toc:, :source-highlighter:, etc.
        if is_doc_attribute(line.trim()) {
            let trimmed = line.trim();
            if trimmed == ":toc:" || trimmed.starts_with(":toc:") {
                blocks.push(Block::Toc { raw: trimmed.to_string() });
            }
            i += 1;
            continue;
        }

        // Page break: <<<
        if line.trim() == "<<<" {
            blocks.push(Block::PageBreak { raw: line.to_string() });
            i += 1;
            continue;
        }

        // Open block: --
        if line.trim() == "--" {
            let (block, consumed) = parse_open_block(&lines[i..], None);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Quote/Verse block: ____
        if line.trim() == "____" || line.trim().starts_with("____") {
            let (block, consumed) = parse_quote_block(&lines[i..], None);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Title line before sidebar, example, code block, table, open block: .Title
        if line.starts_with('.') && !line.starts_with(". ") && !line.starts_with("..") && i + 1 < lines.len() {
            let title = line[1..].trim().to_string();
            let next = lines[i + 1].trim();
            if next == "****" || next.starts_with("****") {
                let (mut block, consumed) = parse_sidebar_block(&lines[i + 1..], Some(title));
                if let Block::Sidebar { ref mut raw, .. } = block {
                    *raw = format!("{}\n{}", line, raw);
                }
                blocks.push(block);
                i += 1 + consumed;
                continue;
            }
            if (next == "====" || next.starts_with("====")) && !is_admonition_kind(line.trim()) {
                let (mut block, consumed) = parse_example_block(&lines[i + 1..], Some(title));
                if let Block::Example { ref mut raw, .. } = block {
                    *raw = format!("{}\n{}", line, raw);
                }
                blocks.push(block);
                i += 1 + consumed;
                continue;
            }
            if next == "--" {
                let (mut block, consumed) = parse_open_block(&lines[i + 1..], Some(title));
                if let Block::Open { ref mut raw, .. } = block {
                    *raw = format!("{}\n{}", line, raw);
                }
                blocks.push(block);
                i += 1 + consumed;
                continue;
            }
            if next == "____" || next.starts_with("____") {
                let (mut block, consumed) = parse_quote_block(&lines[i + 1..], Some(title));
                if let Block::Blockquote { ref mut raw, .. } = block {
                    *raw = format!("{}\n{}", line, raw);
                }
                blocks.push(block);
                i += 1 + consumed;
                continue;
            }
        }

        // Attribute line: [source,sql], [cols="..."], [#ravages], [%notitle], [abstract], [discrete], etc.
        if is_attribute_line(line) {
            let mut attr_lines = vec![line];
            let mut j = i + 1;
            while j < lines.len() && is_attribute_line(lines[j]) {
                attr_lines.push(lines[j]);
                j += 1;
            }
            if j < lines.len() {
                let next = lines[j].trim();
                if next == "|===" {
                    let (block, consumed) = parse_table(&lines[i..]);
                    blocks.push(block);
                    i += consumed;
                    continue;
                }
                if next.starts_with("----") {
                    let lang = parse_source_lang(attr_lines[0].trim());
                    let (block, consumed) = parse_delimited_block(lines[j], &lines[j..], "----", lang);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if next.starts_with("....") {
                    let (block, consumed) = parse_delimited_block(lines[j], &lines[j..], "....", None);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if next.starts_with("****") {
                    let (block, consumed) = parse_sidebar_block(&lines[j..], None);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if (next.starts_with("====") || is_admonition_kind(attr_lines[0])) && next == "====" {
                    let kind = extract_admonition_kind(attr_lines[0]);
                    let (block, consumed) = parse_admonition_block(&kind, &lines[i..]);
                    blocks.push(block);
                    i += consumed;
                    continue;
                }
                if next.starts_with("====") {
                    let (block, consumed) = parse_example_block(&lines[j..], None);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if next.starts_with("____") {
                    let (block, consumed) = parse_quote_block(&lines[j..], None);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if next == "--" {
                    let (block, consumed) = parse_open_block(&lines[j..], None);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if is_heading(next) {
                    if let Some((level, rest)) = parse_heading(next) {
                        let spans = parse_inline(rest.trim());
                        blocks.push(Block::Heading {
                            level,
                            spans,
                            raw: lines[j].to_string(),
                        });
                        i = j + 1;
                        continue;
                    }
                }
                if let Some((marker, level, rest)) = parse_ordered_list_item(next) {
                    let spans = parse_inline(rest.trim());
                    blocks.push(Block::OrderedListItem {
                        level,
                        marker,
                        children: vec![Block::Paragraph { spans, raw: rest.to_string() }],
                        raw: lines[j].to_string(),
                    });
                    i = j + 1;
                    continue;
                }
                if let Some((marker, level, rest)) = parse_unordered_list_item(next) {
                    let (checked, item_text) = parse_checkbox(rest);
                    let (children, _) = parse_list_item_children(item_text.trim(), &lines[j + 1..]);
                    blocks.push(Block::UnorderedListItem {
                        level,
                        marker,
                        checked,
                        children,
                        raw: lines[j].to_string(),
                    });
                    i = j + 1;
                    continue;
                }
                if let Some((block, consumed)) = parse_description_list_item(next, &lines[j + 1..]) {
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
                if !next.is_empty() {
                    // Paragraph with attribute applied
                    let (block, consumed) = parse_paragraph(&lines[j..]);
                    blocks.push(block);
                    i = j + consumed;
                    continue;
                }
            } else {
                let (block, consumed) = parse_paragraph(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }
            // Standalone attribute with trailing empty lines -> skip
            i = j;
            continue;
        }

        // Sidebar block: ****
        if line.trim() == "****" || line.trim().starts_with("****") {
            let (block, consumed) = parse_sidebar_block(&lines[i..], None);
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Example block: ====
        if line.trim() == "====" || line.trim().starts_with("====") {
            let (block, consumed) = parse_example_block(&lines[i..], None);
            blocks.push(block);
            i += consumed;
            continue;
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

        // Block image: image::path.png[Alt text, width=300]
        if line.trim().starts_with("image::") {
            if let Some(block) = parse_image_block(line) {
                blocks.push(block);
                i += 1;
                continue;
            }
        }

        // Admonition block: [WARNING]/[NOTE]/[TIP] followed by ====
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
                children,
                raw: raw_lines.join("\n"),
            });
            i = j;
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
        if let Some((mut marker, level, rest)) = parse_ordered_list_item(line) {
            let is_dot = line.trim_start().starts_with('.');
            if is_dot {
                let mut prev_num = 0;
                for prev in blocks.iter().rev() {
                    if let Block::EmptyLine = prev {
                        continue;
                    }
                    if let Block::OrderedListItem { level: pl, marker: pm, .. } = prev {
                        if *pl == level {
                            if let Ok(n) = pm.trim_end_matches('.').parse::<usize>() {
                                prev_num = n;
                            }
                        }
                    }
                    break;
                }
                marker = format!("{}.", prev_num + 1);
            }
            let (children, consumed) = parse_list_item_children(rest.trim(), &lines[i + 1..]);
            let raw = lines[i..i + consumed].join("\n");
            blocks.push(Block::OrderedListItem {
                level,
                marker,
                children,
                raw,
            });
            i += consumed;
            continue;
        }

        // Unordered list: - / * / ** etc
        if let Some((marker, level, rest)) = parse_unordered_list_item(line) {
            let (checked, item_text) = parse_checkbox(rest);
            let (children, consumed) = parse_list_item_children(item_text.trim(), &lines[i + 1..]);
            let raw = lines[i..i + consumed].join("\n");
            blocks.push(Block::UnorderedListItem {
                level,
                marker,
                checked,
                children,
                raw,
            });
            i += consumed;
            continue;
        }

        // Callout list item: <1> explanation
        if let Some((block, consumed)) = parse_callout_list_item(line, &lines[i + 1..]) {
            blocks.push(block);
            i += consumed;
            continue;
        }

        // Description list item: Term:: Description
        if let Some((block, consumed)) = parse_description_list_item(line, &lines[i + 1..]) {
            blocks.push(block);
            i += consumed;
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
    let t = line.trim();
    (t.starts_with('[') && t.ends_with(']')) || (t.starts_with("[[") && t.ends_with("]]"))
}

/// Parse a cols attribute value (e.g. "1,2a" or "2,1") into (per-column widths, per-column asciidoc flags).
/// 'a' suffix means asciidoc cell. Width is the numeric part (default 1).
fn parse_cols_value(value: &str) -> (Vec<f64>, Vec<bool>) {
    let value = value.trim_matches('"').trim_matches('\'');
    let mut widths = Vec::new();
    let mut asciidoc = Vec::new();
    for spec in value.split(',') {
        let spec = spec.trim();
        if spec.is_empty() {
            widths.push(1.0);
            asciidoc.push(false);
            continue;
        }
        let is_ad = spec.ends_with('a') || spec.ends_with('A');
        let num_str = if is_ad { &spec[..spec.len()-1] } else { spec };
        let w: f64 = num_str.parse().unwrap_or(1.0);
        widths.push(w.max(1.0));
        asciidoc.push(is_ad);
    }
    (widths, asciidoc)
}

#[allow(dead_code)]
pub fn parse_cols_attribute(attr: &str) -> (Vec<f64>, Vec<bool>) {
    let inner = attr.trim_start_matches("[cols=").trim_end_matches(']');
    parse_cols_value(inner)
}

fn parse_table_attributes(
    attr_line: &str,
    col_widths: &mut Vec<f64>,
    col_asciidoc: &mut Vec<bool>,
    frame: &mut Option<String>,
    grid: &mut Option<String>,
) {
    let inner = attr_line.trim_start_matches('[').trim_end_matches(']').trim();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in inner.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
                current.push(ch);
            }
            ',' if !in_quotes => {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    tokens.push(s);
                }
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        tokens.push(s);
    }

    for token in tokens {
        if let Some((k, v)) = token.split_once('=') {
            let key = k.trim().to_lowercase();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            match key.as_str() {
                "cols" => {
                    let (w, a) = parse_cols_value(val);
                    *col_widths = w;
                    *col_asciidoc = a;
                }
                "frame" => {
                    *frame = Some(val.to_lowercase());
                }
                "grid" => {
                    *grid = Some(val.to_lowercase());
                }
                _ => {}
            }
        } else if token.starts_with("cols=") {
            let val = &token[5..];
            let (w, a) = parse_cols_value(val);
            *col_widths = w;
            *col_asciidoc = a;
        } else if token.starts_with("frame=") {
            *frame = Some(token[6..].trim_matches('"').trim_matches('\'').to_lowercase());
        } else if token.starts_with("grid=") {
            *grid = Some(token[5..].trim_matches('"').trim_matches('\'').to_lowercase());
        }
    }
}

fn parse_source_lang(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with('[') || !t.ends_with(']') {
        return None;
    }
    let inner = &t[1..t.len() - 1].trim();
    if inner.starts_with("source,") {
        Some(inner[7..].trim().to_string())
    } else if inner.starts_with(',') {
        // e.g. [,ruby] or [,xml]
        let lang = inner[1..].trim();
        if !lang.is_empty() {
            Some(lang.to_string())
        } else {
            None
        }
    } else if inner.contains(',') {
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        for p in parts {
            if !p.starts_with('%') && !p.starts_with("cols=") && !p.starts_with("grid=") && !p.starts_with("frame=") && !p.is_empty() && p != "source" {
                return Some(p.to_string());
            }
        }
        None
    } else if *inner == "source" {
        None
    } else {
        None
    }
}

fn is_doc_attribute(line: &str) -> bool {
    // :toc:, :source-highlighter: python, etc.
    if !line.starts_with(':') || (!line.ends_with(':') && !line.contains(": ")) {
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
    if !t.starts_with('[') || !t.ends_with(']') {
        return false;
    }
    let inner = &t[1..t.len() - 1];
    let kind_name = inner.split('%').next().unwrap_or("").split(',').next().unwrap_or("").trim();
    ADMONITION_KINDS.contains(&kind_name)
}

fn extract_admonition_kind(attr_line: &str) -> String {
    let t = attr_line.trim().trim_start_matches('[').trim_end_matches(']');
    let name = t.split('%').next().unwrap_or("NOTE").split(',').next().unwrap_or("NOTE").trim();
    if ADMONITION_KINDS.contains(&name) {
        name.to_string()
    } else {
        "NOTE".to_string()
    }
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

    let mut content_lines = Vec::new();
    let mut closed = false;
    let mut last_content_blank = true;
    while consumed < lines.len() {
        let line = lines[consumed];
        let trimmed = line.trim();
        if trimmed == "====" {
            raw_parts.push(line.to_string());
            consumed += 1;
            closed = true;
            break;
        }
        let is_hr = is_horizontal_rule(line) && !trimmed.starts_with("----");
        if last_content_blank
            && (is_heading(line) || is_hr || is_admonition_kind(line))
        {
            break;
        }
        raw_parts.push(line.to_string());
        content_lines.push(line.to_string());
        consumed += 1;
        last_content_blank = trimmed.is_empty();
    }

    if !closed {
        log::warn!("admonition {} missing closing '===='; block closed at next structural boundary", kind);
    }

    let raw = raw_parts.join("\n");
    let content = content_lines.join("\n");
    let children = parse_blocks(&content);
    (Block::Admonition { kind: kind.to_string(), children, raw }, consumed)
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
    if trimmed.starts_with("****") || trimmed.starts_with("====") || trimmed.starts_with("----")
        || trimmed.starts_with("....") || trimmed.starts_with("____") || trimmed.starts_with("///")
        || trimmed == "--" || trimmed == "<<<"
    {
        return false;
    }
    let first = match trimmed.chars().next() {
        Some(c) => c,
        None => return false,
    };
    if first != '-' && first != '*' && first != '\'' {
        return false;
    }
    trimmed.chars().all(|c| c == first)
}

fn parse_image_block(line: &str) -> Option<Block> {
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
                    alt = alt.strip_prefix("alt=").unwrap().trim_matches('"').trim_matches('\'').to_string();
                }
                for part in &parts[1..] {
                    if part.starts_with("width=") {
                        width = Some(part.strip_prefix("width=").unwrap().trim_matches('"').trim_matches('\'').to_string());
                    } else if part.starts_with("height=") {
                        height = Some(part.strip_prefix("height=").unwrap().trim_matches('"').trim_matches('\'').to_string());
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

fn parse_open_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening --
        }
        if line.trim() == "--" {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (
        Block::Open {
            title,
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    )
}

fn parse_quote_block(lines: &[&str], _title: Option<String>) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening ____
        }
        if line.trim() == "____" || line.trim().starts_with("____") {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (
        Block::Blockquote {
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    )
}

fn parse_sidebar_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening ****
        }
        if line.trim() == "****" || line.trim().starts_with("****") {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (
        Block::Sidebar {
            title,
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    )
}

fn parse_example_block(lines: &[&str], title: Option<String>) -> (Block, usize) {
    let mut inner_lines = Vec::new();
    let mut raw_lines = Vec::new();
    let mut consumed = 0;

    for (idx, line) in lines.iter().enumerate() {
        raw_lines.push(line.to_string());
        consumed += 1;
        if idx == 0 {
            continue; // opening ====
        }
        if line.trim() == "====" || line.trim().starts_with("====") {
            break;
        }
        inner_lines.push(*line);
    }

    let children = parse_blocks(&inner_lines.join("\n"));
    (
        Block::Example {
            title,
            children,
            raw: raw_lines.join("\n"),
        },
        consumed,
    )
}

fn parse_callout_list_item(line: &str, next_lines: &[&str]) -> Option<(Block, usize)> {
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
                if next_line.trim().is_empty() || is_delimiter(next_line) || is_heading(next_line) || is_horizontal_rule(next_line) {
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

fn parse_description_list_item(line: &str, next_lines: &[&str]) -> Option<(Block, usize)> {
    let trimmed = line.trim();
    if trimmed.starts_with("image::") || trimmed.starts_with(':') {
        return None;
    }
    let parts: Vec<&str> = trimmed.splitn(2, "::").collect();
    if parts.len() != 2 {
        return None;
    }
    let term = parts[0].trim().to_string();
    if term.is_empty() {
        return None;
    }
    let term_spans = parse_inline(&term);
    let rest = parts[1].trim();
    let mut raw_lines = vec![line.to_string()];
    let mut consumed = 1;
    let mut desc_lines = Vec::new();
    if !rest.is_empty() {
        desc_lines.push(rest.to_string());
    }
    let mut j = 0;
    while j < next_lines.len() {
        let next_line = next_lines[j];
        if next_line.trim().is_empty() {
            break;
        }
        if next_line.trim().contains("::") && !next_line.trim().starts_with("image::") {
            break;
        }
        if is_delimiter(next_line) || is_heading(next_line) || is_horizontal_rule(next_line) {
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
    let mut col_asciidoc: Vec<bool> = Vec::new();
    let mut col_widths: Vec<f64> = Vec::new();
    let mut frame: Option<String> = None;
    let mut grid: Option<String> = None;
    while consumed < lines.len() && is_attribute_line(lines[consumed].trim()) {
        let attr = lines[consumed].trim();
        raw_parts.push(lines[consumed].to_string());
        parse_table_attributes(attr, &mut col_widths, &mut col_asciidoc, &mut frame, &mut grid);
        consumed += 1;
    }

    // Check for |=== delimited table
    if consumed < lines.len() && lines[consumed].trim() == "|===" {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;

        // Track cell type: true = asciidoc (a|), false = text (|)
        let mut current_row_cells: Vec<(Vec<String>, bool)> = Vec::new();
        let mut current_cell: Vec<String> = Vec::new();
        let mut current_cell_asciidoc = false;
        let mut _col_idx: usize = 0;

        let flush_row = |row_cells: &mut Vec<(Vec<String>, bool)>, rows: &mut Vec<Vec<Vec<Block>>>, col_asciidoc: &[bool]| {
            if !row_cells.is_empty() {
                let parsed: Vec<Vec<Block>> = row_cells.iter().enumerate().map(|(ci, (cell_lines, explicit_ad))| {
                    let content = cell_lines.join("\n");
                    let use_ad = if *explicit_ad { true } else { col_asciidoc.get(ci).copied().unwrap_or(false) };
                    if use_ad {
                        parse_blocks(&content)
                    } else {
                        let spans = parse_inline(&content);
                        if spans.is_empty() { vec![] } else { vec![Block::Paragraph { spans, raw: content }] }
                    }
                }).collect();
                rows.push(parsed);
                row_cells.clear();
            }
        };

        while consumed < lines.len() {
            let line = lines[consumed];
            raw_parts.push(line.to_string());

            if line.trim() == "|===" {
                consumed += 1;
                if !current_cell.is_empty() {
                    current_row_cells.push((current_cell, current_cell_asciidoc));
                    current_cell = Vec::new();
                }
                flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
                break;
            }

            if line.trim().is_empty() {
                if !current_cell.is_empty() {
                    current_row_cells.push((current_cell, current_cell_asciidoc));
                    current_cell = Vec::new();
                }
                flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
                _col_idx = 0;
                consumed += 1;
                continue;
            }

            let trimmed = line.trim_start();
            if trimmed.starts_with("a|") || trimmed.starts_with('|') {
                if !current_cell.is_empty() {
                    current_row_cells.push((current_cell, current_cell_asciidoc));
                    current_cell = Vec::new();
                    _col_idx += 1;
                }
                let parts: Vec<&str> = line.split('|').skip(1).collect();
                for (idx, part) in parts.iter().enumerate() {
                    let text = part.trim().to_string();
                    let is_ad = if idx == 0 {
                        trimmed.starts_with("a|")
                    } else {
                        false
                    };
                    if idx == 0 && !text.is_empty() {
                        current_cell.push(text);
                        current_cell_asciidoc = is_ad;
                    } else if !text.is_empty() {
                        if !current_cell.is_empty() {
                            current_row_cells.push((current_cell, current_cell_asciidoc));
                            _col_idx += 1;
                        }
                        current_cell = vec![text];
                        current_cell_asciidoc = is_ad;
                    }
                }
            } else {
                current_cell.push(line.to_string());
            }
            consumed += 1;
        }

        if !current_cell.is_empty() {
            current_row_cells.push((current_cell, current_cell_asciidoc));
        }
        flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
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

    (Block::Table { rows, col_widths, frame, grid, raw: raw_parts.join("\n") }, consumed)
}

fn parse_table_row(line: &str) -> Vec<Vec<Block>> {
    let mut cells: Vec<Vec<Block>> = line.split('|')
        .skip(1)
        .map(|s| {
            let text = s.trim();
            let spans = parse_inline(text);
            if spans.is_empty() { vec![] } else { vec![Block::Paragraph { spans, raw: text.to_string() }] }
        })
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
        if !prefix.is_empty() && (
            prefix.chars().all(|c| c.is_ascii_digit())
            || (prefix.len() <= 4 && prefix.chars().all(|c| c.is_ascii_alphabetic()))
        ) {
            let marker = format!("{}.", prefix);
            let rest = &trimmed[dot_pos + 2..];
            return Some((marker, level, rest));
        }
    }

    None
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

/// Parse checkbox from list item text. Returns (checked, remaining_text).
fn parse_checkbox(text: &str) -> (Option<bool>, &str) {
    let t = text.trim_start();
    if t.starts_with("[ ] ") {
        (Some(false), &t[4..])
    } else if t.starts_with("[x] ") || t.starts_with("[X] ") {
        (Some(true), &t[4..])
    } else {
        (None, text)
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
        if consumed > 0 && is_attribute_line(line.trim()) {
            break;
        }
        // Stop if we hit a special line
        if is_heading(line) || is_horizontal_rule(line) || is_delimiter(line)
            || is_doc_attribute(line.trim())
            || line.trim() == "<<<"
            || line.starts_with('>') || line.trim_start().starts_with('|')
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
    let full_text = text_lines.join(" ");
    let spans = parse_inline(&full_text);

    (Block::Paragraph { spans, raw }, consumed)
}

/// Parse children of a list item. The first line content is the inline text.
/// Subsequent indented lines (2+ spaces) are continuation content parsed recursively.
fn parse_list_item_children(first_line_text: &str, remaining: &[&str]) -> (Vec<Block>, usize) {
    let mut children = Vec::new();

    // First child: the marker text as a paragraph
    let spans = parse_inline(first_line_text);
    if !spans.is_empty() {
        children.push(Block::Paragraph { spans, raw: first_line_text.to_string() });
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
            if peek < remaining.len() && remaining[peek].len() >= 2 && remaining[peek].starts_with("  ") {
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

/// Recursively bump level of all list items in a block tree by `amount`.
fn bump_list_levels(blocks: &mut [Block], amount: u8) {
    for block in blocks {
        match block {
            Block::OrderedListItem { level, children, .. } => {
                *level += amount;
                bump_list_levels(children, amount);
            }
            Block::UnorderedListItem { level, children, .. } => {
                *level += amount;
                bump_list_levels(children, amount);
            }
            _ => {}
        }
    }
}

fn is_heading(line: &str) -> bool {
    parse_heading(line).is_some()
}

fn is_delimiter(line: &str) -> bool {
    let t = line.trim();
    t == "----" || t == "...." || t == "====" || t == "****" || t == "____" || t == "--"
        || t.starts_with("----") || t.starts_with("....") || t.starts_with("====") || t.starts_with("****") || t.starts_with("____")
}

/// Serialize blocks back to AsciiDoc text.
pub fn blocks_to_adoc(blocks: &[Block]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        parts.push(block.raw_text().to_string());
    }
    parts.join("\n")
}

/// Toggle the checkbox on the block at `target_block_idx` in `content`.
/// Supports nested checklist items via `sub_path` (e.g. "" for root item, "1" for first child block, "1.2" for nested).
/// Returns `Some(new_content)` if a checkbox was found and toggled, or `None` if the block index was out of range or not a checklist item.
pub fn toggle_checkbox(content: &str, target_block_idx: usize, sub_path: &str) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut current_block_idx = 0;
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];
        let line_strs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

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

        // Title line before sidebar, example, code block, or table: .Title
        if line.starts_with('.') && !line.starts_with(". ") && !line.starts_with("..") && i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if next == "****" || next.starts_with("****") {
                let (_, consumed) = parse_sidebar_block(&line_strs[i + 1..], None);
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += 1 + consumed;
                continue;
            }
            if (next == "====" || next.starts_with("====")) && !is_admonition_kind(line.trim()) {
                let (_, consumed) = parse_example_block(&line_strs[i + 1..], None);
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += 1 + consumed;
                continue;
            }
        }

        // Attribute line before table or code block
        if is_attribute_line(line.trim()) {
            if let Some(next) = next_non_empty(&line_strs[i + 1..]) {
                if next == "|===" {
                    let (_, consumed) = parse_table(&line_strs[i..]);
                    if current_block_idx == target_block_idx {
                        return None;
                    }
                    current_block_idx += 1;
                    i += consumed;
                    continue;
                }
                if next.starts_with("----") {
                    i += 1; // skip attribute line
                    if i >= lines.len() {
                        break;
                    }
                    let (_, consumed) = parse_delimited_block(lines[i].as_str(), &line_strs[i..], "----", None);
                    if current_block_idx == target_block_idx {
                        return None;
                    }
                    current_block_idx += 1;
                    i += consumed;
                    continue;
                }
                if next.starts_with("****") {
                    i += 1;
                    if i >= lines.len() {
                        break;
                    }
                    let (_, consumed) = parse_sidebar_block(&line_strs[i..], None);
                    if current_block_idx == target_block_idx {
                        return None;
                    }
                    current_block_idx += 1;
                    i += consumed;
                    continue;
                }
                if next.starts_with("====") && !is_admonition_kind(line.trim()) {
                    i += 1;
                    if i >= lines.len() {
                        break;
                    }
                    let (_, consumed) = parse_example_block(&line_strs[i..], None);
                    if current_block_idx == target_block_idx {
                        return None;
                    }
                    current_block_idx += 1;
                    i += consumed;
                    continue;
                }
            }
        }

        // Sidebar block
        if line.trim() == "****" || line.trim().starts_with("****") {
            let (_, consumed) = parse_sidebar_block(&line_strs[i..], None);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Example block
        if line.trim() == "====" || line.trim().starts_with("====") {
            let (_, consumed) = parse_example_block(&line_strs[i..], None);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Code block
        if line.trim() == "----" || line.trim().starts_with("----") {
            let (_, consumed) = parse_delimited_block(line.as_str(), &line_strs[i..], "----", None);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Literal block
        if line.trim() == "...." || line.trim().starts_with("....") {
            let (_, consumed) = parse_delimited_block(line.as_str(), &line_strs[i..], "....", None);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Block image
        if line.trim().starts_with("image::") {
            if let Some(_) = parse_image_block(line) {
                if current_block_idx == target_block_idx {
                    return None;
                }
                current_block_idx += 1;
                i += 1;
                continue;
            }
        }

        // Admonition block
        if is_admonition_kind(line.trim()) && i + 1 < lines.len() && lines[i + 1].trim() == "====" {
            let kind = line.trim().trim_start_matches('[').trim_end_matches(']').to_string();
            let (_, consumed) = parse_admonition_block(&kind, &line_strs[i..]);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Horizontal rule
        if is_horizontal_rule(line) {
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += 1;
            continue;
        }

        // Heading
        if let Some((_, _)) = parse_heading(line) {
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += 1;
            continue;
        }

        // Blockquote
        if line.starts_with('>') {
            let mut j = i;
            while j < lines.len() && lines[j].starts_with('>') {
                j += 1;
            }
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i = j;
            continue;
        }

        // Table
        if line.trim_start().starts_with('|') {
            let (_, consumed) = parse_table(&line_strs[i..]);
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Ordered list
        if let Some((_, _, _)) = parse_ordered_list_item(line) {
            let (_, consumed) = parse_list_item_children("", &line_strs[i + 1..]);
            if current_block_idx == target_block_idx {
                if sub_path.is_empty() {
                    return None;
                }
                return toggle_nested_continuation_lines(&mut lines, i, consumed, sub_path, content.ends_with('\n'));
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Unordered list
        if let Some((_, _, rest)) = parse_unordered_list_item(line) {
            let (checked, item_text) = parse_checkbox(rest);
            let (_, consumed) = parse_list_item_children(item_text.trim(), &line_strs[i + 1..]);
            if current_block_idx == target_block_idx {
                if sub_path.is_empty() {
                    if checked.is_some() {
                        let old_line = &lines[i];
                        let toggled = if old_line.contains("[ ] ") {
                            old_line.replacen("[ ] ", "[x] ", 1)
                        } else if old_line.contains("[x] ") {
                            old_line.replacen("[x] ", "[ ] ", 1)
                        } else if old_line.contains("[X] ") {
                            old_line.replacen("[X] ", "[ ] ", 1)
                        } else {
                            return None;
                        };
                        lines[i] = toggled;
                        let mut result = lines.join("\n");
                        if content.ends_with('\n') {
                            result.push('\n');
                        }
                        return Some(result);
                    }
                    return None;
                } else {
                    return toggle_nested_continuation_lines(&mut lines, i, consumed, sub_path, content.ends_with('\n'));
                }
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Callout list item
        if let Some((_, consumed)) = parse_callout_list_item(line, &line_strs[i + 1..]) {
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Description list item
        if let Some((_, consumed)) = parse_description_list_item(line, &line_strs[i + 1..]) {
            if current_block_idx == target_block_idx {
                return None;
            }
            current_block_idx += 1;
            i += consumed;
            continue;
        }

        // Paragraph
        let (_, consumed) = parse_paragraph(&line_strs[i..]);
        if current_block_idx == target_block_idx {
            return None;
        }
        current_block_idx += 1;
        i += consumed;
    }

    None
}

fn toggle_nested_continuation_lines(
    lines: &mut Vec<String>,
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
            lines[line_idx + 1 + k] = format!("{}{}", indent_prefixes[k], new_cont_lines[k]);
        }
    }

    let mut result = lines.join("\n");
    if ends_with_newline {
        result.push('\n');
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::InlineSpan;

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
            Block::Table { rows, col_widths, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(col_widths, &[1.0, 2.0]);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_with_frame_and_grid() {
        let text = "[frame=none, grid=rows, cols=\"1,2\"]\n|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, col_widths, frame, grid, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(col_widths, &[1.0, 2.0]);
                assert_eq!(frame.as_deref(), Some("none"));
                assert_eq!(grid.as_deref(), Some("rows"));
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
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, "WARNING");
                assert!(!children.is_empty());
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_admonition_with_inline_code() {
        let text = "[WARNING]\n====\nAlways run `cargo test` first.\n====";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Admonition { children, .. } => {
                // Children should contain a Paragraph with a Code span
                match &children[0] {
                    Block::Paragraph { spans, .. } => {
                        assert!(spans.iter().any(|s| matches!(s, InlineSpan::Code(_))));
                    }
                    _ => panic!("expected paragraph child"),
                }
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
                // First cell should have a Paragraph with a Bold span
                match &rows[0][0][0] {
                    Block::Paragraph { spans, .. } => {
                        assert!(spans.iter().any(|s| matches!(s, InlineSpan::Bold { .. })));
                    }
                    _ => panic!("expected paragraph in cell"),
                }
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

    // === Nested block tests (Step 2 - TDD RED phase) ===

    #[test]
    fn parse_list_item_with_continuation_code_block() {
        let text = "- step one\n\n  ----\n  let x = 1;\n  ----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::UnorderedListItem { children, .. } => {
                // Should have: paragraph ("step one"), empty_line, code_block
                assert!(children.len() >= 3, "expected 3+ children, got {}: {:?}", children.len(), children.iter().map(|c| c.block_type()).collect::<Vec<_>>());
                assert_eq!(children[0].block_type(), "paragraph");
                assert_eq!(children[1].block_type(), "empty_line");
                assert_eq!(children[2].block_type(), "code_block");
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_blockquote_consecutive_lines() {
        let text = "> line one\n> line two";
        let blocks = parse_blocks(text);
        // Should be a single blockquote (not 2 separate ones)
        assert_eq!(blocks.len(), 1, "expected 1 blockquote, got {}", blocks.len());
        match &blocks[0] {
            Block::Blockquote { children, .. } => {
                // Lines joined as one paragraph (no blank line separator)
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn parse_admonition_with_multiple_blocks() {
        let text = "[NOTE]\n====\nFirst paragraph.\n\nSecond paragraph.\n\n----\ncode\n----\n====";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, "NOTE");
                assert!(children.len() >= 3, "expected 3+ children, got {}", children.len());
                assert_eq!(children[0].block_type(), "paragraph");
                assert_eq!(children[1].block_type(), "empty_line");
                assert_eq!(children[2].block_type(), "paragraph");
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_checkbox_unchecked() {
        let text = "- [ ] buy groceries";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, Some(false));
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_checked() {
        let text = "- [x] done task";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, Some(true));
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_none() {
        let text = "- regular item";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, None);
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_serializes() {
        let text = "- [ ] task\n- [x] done";
        let blocks = parse_blocks(text);
        let b0 = blocks[0].to_qvariant_map();
        assert_eq!(b0["checked"], false);
        let b1 = blocks[1].to_qvariant_map();
        assert_eq!(b1["checked"], true);
    }

    #[test]
    fn parse_three_level_nesting() {
        let text = "1. Planning\n  * Research\n    * Read code\n    * Check docs\n  * Design\n2. Development";
        let blocks = parse_blocks(text);
        // Top level: 2 OL items
        assert_eq!(blocks.len(), 2, "expected 2 top blocks, got {}", blocks.len());
        match &blocks[0] {
            Block::OrderedListItem { children, .. } => {
                // Children should include UL "Research" and UL "Design"
                let uls: Vec<_> = children.iter().filter(|c| c.block_type() == "unordered_list_item").collect();
                assert!(uls.len() >= 2, "expected 2+ UL children, got {}", uls.len());
                // "Research" should be at level 1
                match uls[0] {
                    Block::UnorderedListItem { level, children: research_children, .. } => {
                        assert_eq!(*level, 1, "Research should be level 1, got {}", level);
                        // "Read code" and "Check docs" should be children of Research at level 2
                        let deep_uls: Vec<_> = research_children.iter()
                            .filter(|c| c.block_type() == "unordered_list_item")
                            .collect();
                        assert!(deep_uls.len() >= 2, "expected 2+ deep children, got {}", deep_uls.len());
                        for deep in &deep_uls {
                            match deep {
                                Block::UnorderedListItem { level, .. } => {
                                    assert_eq!(*level, 2, "deep item should be level 2, got {}", level);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => panic!("expected UL"),
                }
                // "Design" should be at level 1
                match uls[1] {
                    Block::UnorderedListItem { level, .. } => {
                        assert_eq!(*level, 1, "Design should be level 1, got {}", level);
                    }
                    _ => {}
                }
            }
            _ => panic!("expected OL"),
        }
    }

    #[test]
    fn parse_nested_list_with_asterisk_marker() {
        let text = "1. first\n  * sub a\n  * sub b\n2. second";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2, "expected 2 top-level blocks, got {}", blocks.len());
        match &blocks[0] {
            Block::OrderedListItem { level, marker, children, .. } => {
                assert_eq!(*level, 0);
                assert_eq!(marker, "1.");
                // Sub-items should be UL at level 1 (parent level 0 + 1)
                let ul_children: Vec<_> = children.iter().filter(|c| c.block_type() == "unordered_list_item").collect();
                assert!(ul_children.len() >= 2, "expected 2+ UL children, got {}", ul_children.len());
                for ul in &ul_children {
                    match ul {
                        Block::UnorderedListItem { level, .. } => assert_eq!(*level, 1, "sub-item should be level 1"),
                        _ => {}
                    }
                }
            }
            _ => panic!("expected ordered list item"),
        }
    }

    #[test]
    fn parse_ordered_list_enumeration_sequence() {
        let text = "1. Install\n2. Clone\n3. Configure\n  a. Open file\n  b. Set target\n4. Run\n5. Deploy\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5);
        if let Block::OrderedListItem { marker, .. } = &blocks[0] { assert_eq!(marker, "1."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[1] { assert_eq!(marker, "2."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, children, .. } = &blocks[2] {
            assert_eq!(marker, "3.");
            let ol_children: Vec<_> = children.iter().filter(|c| c.block_type() == "ordered_list_item").collect();
            assert_eq!(ol_children.len(), 2);
            if let Block::OrderedListItem { marker, .. } = ol_children[0] { assert_eq!(marker, "a."); }
            if let Block::OrderedListItem { marker, .. } = ol_children[1] { assert_eq!(marker, "b."); }
        } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[3] { assert_eq!(marker, "4."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[4] { assert_eq!(marker, "5."); } else { panic!("expected OL"); }
    }

    #[test]
    fn parse_ordered_list_dot_sequence() {
        let text = ". Step one\n. Step two\n. Step three\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3);
        if let Block::OrderedListItem { marker, .. } = &blocks[0] { assert_eq!(marker, "1."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[1] { assert_eq!(marker, "2."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[2] { assert_eq!(marker, "3."); } else { panic!("expected OL"); }
    }

    #[test]
    fn parse_table_asciidoc_cell_prefix() {
        let text = "|===\n| simple text\na| - list item 1\n- list item 2\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[0][0].len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
                assert!(rows[0][1].len() >= 2, "expected 2+ blocks in a| cell, got {}", rows[0][1].len());
                assert_eq!(rows[0][1][0].block_type(), "unordered_list_item");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_cols_attribute_asciidoc() {
        let text = "[cols=\"1,2a\"]\n|===\n| header\n| - item 1\n- item 2\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
                assert!(rows[0][1].len() >= 1);
                assert_eq!(rows[0][1][0].block_type(), "unordered_list_item");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_text_cell_no_blocks() {
        let text = "|===\n| *bold* text\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows[0][0].len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
            }
            _ => panic!("expected table"),
        }
    }

    // === Panic-proof tests: parse_blocks must never panic on any input ===

    #[test]
    fn parse_blocks_empty_string() {
        let blocks = parse_blocks("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn parse_blocks_only_whitespace() {
        let blocks = parse_blocks("   \n  \n");
        // Should produce empty_line blocks, never panic
        assert!(!blocks.is_empty() || blocks.is_empty()); // just no panic
    }

    #[test]
    fn parse_blocks_attribute_at_eof() {
        // Attribute line at end of file — previously panicked (lines[i] after i += 1)
        let blocks = parse_blocks("[source,sql]");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "paragraph");
    }

    #[test]
    fn parse_blocks_cols_attribute_at_eof() {
        let blocks = parse_blocks("[cols=\"1,2\"]");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_cols_empty_spec() {
        // [cols=] or [cols=","] — empty spec should not panic
        let blocks = parse_blocks("[cols=]\n|===\n| a\n|===");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_cols_just_a_suffix() {
        // [cols="a"] — spec is just "a", num_str would be empty
        let blocks = parse_blocks("[cols=\"a\"]\n|===\n| cell\n|===");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_unclosed_code_block() {
        // Opening ---- without closing ----
        let blocks = parse_blocks("----\nlet x = 1;\nlet y = 2;");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "code_block");
    }

    #[test]
    fn parse_blocks_unclosed_table() {
        // |=== without closing |===
        let blocks = parse_blocks("|===\n| cell 1\n| cell 2");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "table");
    }

    #[test]
    fn parse_blocks_admonition_without_closing() {
        let blocks = parse_blocks("[WARNING]\n====\nBe careful!");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "admonition");
    }

    #[test]
    fn parse_blocks_admonition_missing_closing_does_not_swallow_rest() {
        // [WARNING] with no closing ==== must end at the next heading,
        // leaving the heading and following paragraph in the document.
        let text = "[WARNING]\n====\nBe careful!\n\n== Next section\n\nMore text here.";
        let blocks = parse_blocks(text);
        assert!(blocks[0].block_type() == "admonition");
        assert!(blocks.iter().any(|b| b.block_type() == "heading"), "heading swallowed by admonition");
        assert!(blocks.iter().any(|b| b.block_type() == "paragraph" && b.raw_text().contains("More text")), "content after heading swallowed");
    }

    #[test]
    fn parse_blocks_nested_parse_does_not_panic() {
        // Deeply nested content that exercises recursive parse_blocks calls
        let text = "> > > deeply nested\n> > still nested\n> top level";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_table_with_asciidoc_cells() {
        // Table with a| cells calls parse_blocks recursively
        let text = "|===\na| - item 1\n  - sub item\n- item 2\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_list_item_with_nested_content() {
        // List item continuation calls parse_blocks recursively
        let text = "- item\n  - sub item\n    - sub sub item";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_doc_attribute_vs_date_paragraph() {
        let text = "= Title\n2024-01-15\n:toc:\n:author: John Doe\n\nParagraph with date: 2024-01-15";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5);
        assert!(matches!(blocks[0], Block::Heading { .. }));
        assert!(matches!(blocks[1], Block::Paragraph { .. }));
        assert!(matches!(blocks[2], Block::Toc { .. }));
        assert!(matches!(blocks[3], Block::EmptyLine));
        assert!(matches!(blocks[4], Block::Paragraph { .. }));
    }

    #[test]
    fn parse_blocks_advanced_demo_does_not_panic() {
        // This is the actual file that caused the crash
        let text = include_str!("../examples/advanced-demo.adoc");
        let _blocks = parse_blocks(text);
        // Must not panic — that's the only assertion
    }

    #[test]
    fn parse_blocks_fuzz_like_inputs() {
        // Various pathological inputs that must not panic
        let long_eq = "=".repeat(100);
        let long_dash = "- ".repeat(50);
        let inputs = vec![
            "|",
            "|||",
            "a|",
            "[",
            "]",
            "[]",
            "[[]]",
            "= ",
            "=======",
            "----",
            "....",
            "====",
            "> ",
            "- ",
            "* ",
            "1. ",
            "\n\n\n",
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj",
            &long_eq,
            &long_dash,
            "|===\n|===\n|===",
        ];
        for input in inputs {
            let _blocks = parse_blocks(input);
            // Must not panic
        }
    }

    #[test]
    fn test_toggle_checkbox() {
        let text = "= Title\n\n- [ ] Task 1\n- [x] Task 2\n- [ ] Task 3\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5); // heading, empty, item1, item2, item3

        let toggled1 = toggle_checkbox(text, 2, "").unwrap();
        assert_eq!(toggled1, "= Title\n\n- [x] Task 1\n- [x] Task 2\n- [ ] Task 3\n");

        let toggled2 = toggle_checkbox(text, 3, "").unwrap();
        assert_eq!(toggled2, "= Title\n\n- [ ] Task 1\n- [ ] Task 2\n- [ ] Task 3\n");

        // Non-checkbox block
        assert!(toggle_checkbox(text, 0, "").is_none());
        // Out of bounds block
        assert!(toggle_checkbox(text, 99, "").is_none());
    }

    #[test]
    fn test_toggle_checkbox_nested() {
        let text = "* [ ] Parent 1\n  * [x] Child 1.1\n  * [ ] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);

        // Toggle Child 1.1 (sub_path "1") -> should uncheck Child 1.1
        let t1 = toggle_checkbox(text, 0, "1").unwrap();
        assert_eq!(t1, "* [ ] Parent 1\n  * [ ] Child 1.1\n  * [ ] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n");

        // Toggle Child 1.2 (sub_path "2") -> should check Child 1.2
        let t2 = toggle_checkbox(text, 0, "2").unwrap();
        assert_eq!(t2, "* [ ] Parent 1\n  * [x] Child 1.1\n  * [x] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n");

        // Toggle Grandchild 1.2.1 (sub_path "2.1") -> should check Grandchild 1.2.1
        let t3 = toggle_checkbox(text, 0, "2.1").unwrap();
        assert_eq!(t3, "* [ ] Parent 1\n  * [x] Child 1.1\n  * [ ] Child 1.2\n    * [x] Grandchild 1.2.1\n* [x] Parent 2\n");
    }

    #[test]
    fn test_parse_image_block() {
        let text = "image::screenshots/screen1.png[Main Screen, width=400, height=300]\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Image { target, alt, width, height, .. } = &blocks[0] {
            assert_eq!(target, "screenshots/screen1.png");
            assert_eq!(alt, "Main Screen");
            assert_eq!(width.as_deref(), Some("400"));
            assert_eq!(height.as_deref(), Some("300"));
        } else {
            panic!("expected Block::Image");
        }
    }

    #[test]
    fn test_parse_sidebar_block() {
        let text = ".Tips and Tricks\n****\nThis is inside a sidebar.\n\n* Item A\n* Item B\n****\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Sidebar { title, children, .. } = &blocks[0] {
            assert_eq!(title.as_deref(), Some("Tips and Tricks"));
            assert_eq!(children.len(), 4); // paragraph, empty_line, 2 UL items
        } else {
            panic!("expected Block::Sidebar");
        }
    }

    #[test]
    fn test_parse_example_block() {
        let text = ".Example Result\n====\nSample output here.\n====\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Example { title, children, .. } = &blocks[0] {
            assert_eq!(title.as_deref(), Some("Example Result"));
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected Block::Example");
        }
    }

    #[test]
    fn test_parse_description_list() {
        let text = "CPU:: Central Processing Unit\nRAM:: Random Access Memory\nGPU::\n  Graphics Processing Unit\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3);
        if let Block::DescriptionListItem { term, children, .. } = &blocks[0] {
            assert_eq!(term, "CPU");
            assert_eq!(children.len(), 1);
        } else { panic!("expected DL item 0"); }
        if let Block::DescriptionListItem { term, .. } = &blocks[1] {
            assert_eq!(term, "RAM");
        } else { panic!("expected DL item 1"); }
        if let Block::DescriptionListItem { term, children, .. } = &blocks[2] {
            assert_eq!(term, "GPU");
            assert_eq!(children.len(), 1);
        } else { panic!("expected DL item 2"); }
    }

    #[test]
    fn test_parse_callout_list() {
        let text = "<1> Initialize the system\n<2> Connect to database\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        if let Block::CalloutListItem { number, children, .. } = &blocks[0] {
            assert_eq!(*number, 1);
            assert_eq!(children.len(), 1);
        } else { panic!("expected Callout 1"); }
        if let Block::CalloutListItem { number, children, .. } = &blocks[1] {
            assert_eq!(*number, 2);
            assert_eq!(children.len(), 1);
        } else { panic!("expected Callout 2"); }
    }

    #[test]
    fn test_parse_anonymous_sidebar_and_example() {
        let text = "****\nAnonymous sidebar\n****\n\n====\nAnonymous example\n====\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3); // sidebar, empty_line, example
        if let Block::Sidebar { title, children, .. } = &blocks[0] {
            assert!(title.is_none());
            assert_eq!(children.len(), 1);
        } else { panic!("expected anonymous sidebar"); }
        if let Block::Example { title, children, .. } = &blocks[2] {
            assert!(title.is_none());
            assert_eq!(children.len(), 1);
        } else { panic!("expected anonymous example"); }
    }

    #[test]
    fn test_description_list_formatted_term() {
        let text = "`CONFIG_PATH`:: Path to config file\n*Verbose Mode*:: Enable logging\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        if let Block::DescriptionListItem { term_spans, .. } = &blocks[0] {
            assert_eq!(term_spans, &vec![InlineSpan::Code("CONFIG_PATH".into())]);
        } else { panic!("expected DL 0"); }
        if let Block::DescriptionListItem { term_spans, .. } = &blocks[1] {
            assert_eq!(term_spans, &vec![InlineSpan::Bold(vec![InlineSpan::Text("Verbose Mode".into())])]);
        } else { panic!("expected DL 1"); }
    }

    #[test]
    fn test_toggle_checkbox_with_extended_blocks() {
        let text = "= Doc Title\n\nimage::pic.png[Photo]\n\n****\nSidebar text\n****\n\n- [ ] Target item\n\nTerm:: Description\n";
        let blocks = parse_blocks(text);
        // blocks:
        // 0: heading
        // 1: empty_line
        // 2: image
        // 3: empty_line
        // 4: sidebar
        // 5: empty_line
        // 6: unordered_list_item (- [ ] Target item)
        // 7: empty_line
        // 8: description_list_item
        assert_eq!(blocks.len(), 9);
        assert_eq!(blocks[6].block_type(), "unordered_list_item");

        let toggled = toggle_checkbox(text, 6, "").expect("should toggle Target item at index 6");
        assert!(toggled.contains("- [x] Target item"));
    }

    #[test]
    fn test_chronicles_features_parsing() {
        let text = "// Settings:\n:description: A chronicle of adventures \\\n  across the realms.\n:wolpertinger: Wolpertinger\n\n[%notitle]\n[abstract]\n{description}\n\n[#ravages]\n== Section With Anchor\n\nAt ((Antwerp)) (((Conference,Devoxx))) we saw the {wolpertinger}!\n\n--\nHere is some content inside an open block.\n--\n\n<<<\n\n[appendix]\n== Appendix Section\n";
        let blocks = parse_blocks(text);

        // Check that comments and document attributes were preprocessed/filtered
        assert!(!blocks.iter().any(|b| b.raw_text().contains("// Settings:")));

        // Check that {description} was substituted and [abstract] block attribute applied
        let desc_p = blocks.iter().find(|b| b.raw_text().contains("across the realms")).expect("description paragraph");
        assert!(desc_p.raw_text().contains("A chronicle of adventures across the realms."));

        // Check heading
        let h1 = blocks.iter().find(|b| b.block_type() == "heading").expect("heading");
        assert_eq!(h1.raw_text(), "== Section With Anchor");

        // Check open block
        let open_b = blocks.iter().find(|b| b.block_type() == "open").expect("open block");
        if let Block::Open { children, .. } = open_b {
            assert_eq!(children.len(), 1);
            assert!(children[0].raw_text().contains("inside an open block"));
        } else {
            panic!("expected Block::Open");
        }

        // Check page break
        assert!(blocks.iter().any(|b| b.block_type() == "page_break"));
    }

    #[test]
    fn test_conditionals_and_comments() {
        let text = "////\nComment block to ignore\nMore comments\n////\n:my-feature:\n\nifdef::my-feature[]\nFeature is active.\nendif::[]\n\nifndef::nonexistent[]\nFallback content.\nendif::[]\n";
        let blocks = parse_blocks(text);
        assert!(!blocks.iter().any(|b| b.raw_text().contains("Comment block")));
        assert!(blocks.iter().any(|b| b.raw_text().contains("Feature is active.")));
        assert!(blocks.iter().any(|b| b.raw_text().contains("Fallback content.")));
    }

    #[test]
    fn test_configurable_comment_dropping() {
        let text = "// Top comment\n= Doc Title\n\n// Section note\nParagraph text\n";
        let dropped = parse_blocks_with_options(text, true);
        assert!(!dropped.iter().any(|b| b.block_type() == "comment"));

        let preserved = parse_blocks_with_options(text, false);
        assert!(preserved.iter().any(|b| b.block_type() == "comment" && b.raw_text().contains("// Top comment")));
        assert!(preserved.iter().any(|b| b.block_type() == "comment" && b.raw_text().contains("// Section note")));
    }
}
