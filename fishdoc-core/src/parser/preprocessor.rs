use crate::parser::attributes::is_doc_attribute;
use crate::parser::delimiters::{is_code_delimiter, is_comment_delimiter, is_literal_delimiter};

pub fn preprocess_asciidoc(text: &str, drop_comments: bool) -> Vec<String> {
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
        if is_comment_delimiter(trimmed) {
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
        if is_code_delimiter(trimmed) || is_literal_delimiter(trimmed) {
            let delim = if is_code_delimiter(trimmed) { "----" } else { "...." };
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
        if is_code_delimiter(trimmed) || is_literal_delimiter(trimmed) {
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

pub fn parse_doc_attr_def(line: &str) -> (String, String, bool) {
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

pub fn substitute_attributes(line: &str, attrs: &std::collections::HashMap<String, String>) -> String {
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
