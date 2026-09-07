pub const ADMONITION_KINDS: &[&str] = &["NOTE", "TIP", "WARNING", "CAUTION", "IMPORTANT"];

pub fn is_code_delimiter(t: &str) -> bool {
    let t = t.trim();
    if !t.starts_with("----") {
        return false;
    }
    if t.chars().all(|c| c == '-') {
        return true;
    }
    t.len() > 4 && t[4..].chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

pub fn is_literal_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 4 && t.chars().all(|c| c == '.')
}

pub fn is_example_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 4 && t.chars().all(|c| c == '=')
}

pub fn is_sidebar_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 4 && t.chars().all(|c| c == '*')
}

pub fn is_quote_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 4 && t.chars().all(|c| c == '_')
}

pub fn is_comment_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 4 && t.chars().all(|c| c == '/')
}

pub fn is_table_delimiter(t: &str) -> bool {
    let t = t.trim();
    t.starts_with("|===") && t.chars().skip(1).all(|c| c == '=')
}

pub fn is_open_delimiter(t: &str) -> bool {
    t.trim() == "--"
}

pub fn is_delimiter(line: &str) -> bool {
    let t = line.trim();
    is_code_delimiter(t)
        || is_literal_delimiter(t)
        || is_example_delimiter(t)
        || is_sidebar_delimiter(t)
        || is_quote_delimiter(t)
        || is_comment_delimiter(t)
        || is_table_delimiter(t)
        || is_open_delimiter(t)
}

pub fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    if is_delimiter(trimmed) || is_comment_delimiter(trimmed) || trimmed == "<<<" {
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

pub fn is_admonition_kind(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('[') || !t.ends_with(']') {
        return false;
    }
    let inner = &t[1..t.len() - 1];
    let kind_name = inner.split('%').next().unwrap_or("").split(',').next().unwrap_or("").trim();
    ADMONITION_KINDS.contains(&kind_name)
}

pub fn extract_admonition_kind(attr_line: &str) -> String {
    let t = attr_line.trim().trim_start_matches('[').trim_end_matches(']');
    let name = t.split('%').next().unwrap_or("NOTE").split(',').next().unwrap_or("NOTE").trim();
    if ADMONITION_KINDS.contains(&name) {
        name.to_string()
    } else {
        "NOTE".to_string()
    }
}
