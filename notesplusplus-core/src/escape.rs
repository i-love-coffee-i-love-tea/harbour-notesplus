/// Escapes all HTML-sensitive characters including quotes (for use in attributes).
pub fn escape_html(s: &str) -> String {
    escape_html_impl(s, true)
}

/// Escapes only &, <, > (for text content where quote escaping is unnecessary).
pub fn escape_html_text(s: &str) -> String {
    escape_html_impl(s, false)
}

fn escape_html_impl(s: &str, escape_quotes: bool) -> String {
    if !s.contains('&') && !s.contains('<') && !s.contains('>')
        && (!escape_quotes || (!s.contains('"') && !s.contains('\'')))
    {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if escape_quotes => out.push_str("&quot;"),
            '\'' if escape_quotes => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
