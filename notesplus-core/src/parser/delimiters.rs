use crate::block::AdmonitionKind;

pub const ADMONITION_KINDS: &[&str] = &["NOTE", "TIP", "WARNING", "CAUTION", "IMPORTANT"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DelimiterKind {
    Code,       // ----
    Literal,    // ....
    Table,      // |===
    Sidebar,    // ****
    Example,    // ====
    Quote,      // ____
    Open,       // --
    Comment,    // ////
}

impl DelimiterKind {
    pub fn opener_str(&self) -> &'static str {
        match self {
            DelimiterKind::Code => "----",
            DelimiterKind::Literal => "....",
            DelimiterKind::Table => "|===",
            DelimiterKind::Sidebar => "****",
            DelimiterKind::Example => "====",
            DelimiterKind::Quote => "____",
            DelimiterKind::Open => "--",
            DelimiterKind::Comment => "////",
        }
    }

    pub fn from_line(line: &str) -> Option<Self> {
        let t = line.trim();
        if is_code_delimiter(t) { return Some(DelimiterKind::Code); }
        if is_literal_delimiter(t) { return Some(DelimiterKind::Literal); }
        if is_table_delimiter(t) { return Some(DelimiterKind::Table); }
        if is_sidebar_delimiter(t) { return Some(DelimiterKind::Sidebar); }
        if is_example_delimiter(t) { return Some(DelimiterKind::Example); }
        if is_quote_delimiter(t) { return Some(DelimiterKind::Quote); }
        if is_open_delimiter(t) { return Some(DelimiterKind::Open); }
        if is_comment_delimiter(t) { return Some(DelimiterKind::Comment); }
        None
    }
}

pub fn is_code_delimiter(t: &str) -> bool {
    let t = t.trim();
    if !t.starts_with("----") {
        return false;
    }
    if t.chars().all(|c| c == '-') {
        return true;
    }
    // Allow standard language identifier immediately following the delimiter (e.g. ----rust, ----python)
    t.len() > 4 && t.len() <= 20 && t[4..].chars().all(|c| c.is_ascii_alphabetic())
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

pub fn extract_admonition_kind(attr_line: &str) -> AdmonitionKind {
    let t = attr_line.trim().trim_start_matches('[').trim_end_matches(']');
    let name = t.split('%').next().unwrap_or("NOTE").split(',').next().unwrap_or("NOTE").trim();
    AdmonitionKind::from(name)
}

/// Returns the canonical delimiter opener if `line` is a block delimiter, or `None`.
/// Useful for tracking open/close state without a separate check per delimiter type.
pub fn as_delimiter_opener(line: &str) -> Option<&'static str> {
    DelimiterKind::from_line(line).map(|d| d.opener_str())
}
