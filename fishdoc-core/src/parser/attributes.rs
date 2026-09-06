pub fn is_attribute_line(line: &str) -> bool {
    let t = line.trim();
    (t.starts_with('[') && t.ends_with(']')) || (t.starts_with("[[") && t.ends_with("]]"))
}

pub fn is_doc_attribute(line: &str) -> bool {
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

pub fn parse_source_lang(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with('[') || !t.ends_with(']') {
        return None;
    }
    let inner = t[1..t.len() - 1].trim();
    if let Some(rest) = inner.strip_prefix("source,") {
        Some(rest.trim().to_string())
    } else if let Some(rest) = inner.strip_prefix(',') {
        // e.g. [,ruby] or [,xml]
        let lang = rest.trim();
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
    } else {
        None
    }
}

pub fn parse_cols_value(val: &str) -> (Vec<f64>, Vec<bool>) {
    let val = val.trim_matches('"').trim_matches('\'').trim();
    let mut widths = Vec::new();
    let mut asciidoc = Vec::new();
    for spec in val.split(',') {
        let spec = spec.trim();
        if spec.is_empty() {
            continue;
        }
        let is_ad = spec.ends_with('a') || spec.ends_with('A') || spec.ends_with('d') || spec.ends_with('D');
        let num_str = if is_ad { &spec[..spec.len() - 1] } else { spec };
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

pub fn parse_table_attributes(
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
        }
    }
}

pub fn parse_quote_or_verse_attr(attr: &str) -> (bool, bool, Option<String>, Option<String>) {
    let inner = attr.trim().trim_start_matches('[').trim_end_matches(']').trim();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in inner.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            ',' if !in_quotes => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    parts.push(current.trim().to_string());

    let mut is_quote = false;
    let mut is_verse = false;
    let mut attribution = None;
    let mut citation = None;

    if !parts.is_empty() {
        let first = parts[0].to_lowercase();
        if first == "quote" {
            is_quote = true;
        } else if first == "verse" {
            is_verse = true;
        }
    }

    if parts.len() > 1 && !parts[1].is_empty() {
        attribution = Some(parts[1].trim_matches('"').trim_matches('\'').to_string());
    }
    if parts.len() > 2 && !parts[2].is_empty() {
        citation = Some(parts[2].trim_matches('"').trim_matches('\'').to_string());
    }

    (is_quote, is_verse, attribution, citation)
}

pub fn parse_ordered_list_start(attr_lines: &[&str]) -> Option<usize> {
    parse_ordered_list_attributes(attr_lines).0
}

pub fn parse_ordered_list_attributes(attr_lines: &[&str]) -> (Option<usize>, bool, Option<u8>) {
    let mut start_num = None;
    let mut is_reversed = false;
    let mut numbering_style = None;

    for a in attr_lines {
        let inner = a.trim().trim_start_matches('[').trim_end_matches(']').trim();
        for token in inner.split(',') {
            let t = token.trim();
            if t.is_empty() {
                continue;
            }
            let t_clean = t.trim_matches('"').trim_matches('\'').trim();
            if t == "%reversed"
                || t == "reversed"
                || t_clean == "%reversed"
                || t_clean == "reversed"
                || t.starts_with("options=\"reversed\"")
                || t.starts_with("opts=\"reversed\"")
                || t.starts_with("options=reversed")
                || t.starts_with("opts=reversed")
                || t.starts_with("options=\"%reversed\"")
                || t.starts_with("opts=\"%reversed\"")
                || t == "reversed=\"reversed\""
            {
                is_reversed = true;
            } else if let Some((k, v)) = t.split_once('=') {
                let k_trim = k.trim();
                let v_trim = v.trim().trim_matches('"').trim_matches('\'').trim();
                if k_trim == "start" {
                    start_num = v_trim.parse::<usize>().ok();
                } else if (k_trim == "options" || k_trim == "opts")
                    && (v_trim.split('+').any(|opt| opt.trim() == "reversed" || opt.trim() == "%reversed")
                        || v_trim.split(',').any(|opt| opt.trim() == "reversed" || opt.trim() == "%reversed"))
                    {
                        is_reversed = true;
                    }
            } else {
                match t_clean {
                    "arabic" | "1" => numbering_style = Some(0),
                    "loweralpha" | "a" => numbering_style = Some(1),
                    "lowerroman" | "i" => numbering_style = Some(2),
                    "upperalpha" | "A" => numbering_style = Some(3),
                    "upperroman" | "I" => numbering_style = Some(4),
                    _ => {}
                }
            }
        }
    }

    (start_num, is_reversed, numbering_style)
}
