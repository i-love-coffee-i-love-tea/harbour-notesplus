use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InlineSpan {
    Text(String),
    Bold(Vec<InlineSpan>),
    Italic(Vec<InlineSpan>),
    Code(String),
    Link { url: String, display: String },
    Xref { target: String, display: String },
    Strikethrough(Vec<InlineSpan>),
    Superscript(Vec<InlineSpan>),
    Subscript(Vec<InlineSpan>),
}

impl InlineSpan {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            InlineSpan::Text(s) => serde_json::json!({"type": "text", "value": s}),
            InlineSpan::Bold(spans) => serde_json::json!({"type": "bold", "spans": spans_json(spans)}),
            InlineSpan::Italic(spans) => serde_json::json!({"type": "italic", "spans": spans_json(spans)}),
            InlineSpan::Code(s) => serde_json::json!({"type": "code", "value": s}),
            InlineSpan::Link { url, display } => serde_json::json!({"type": "link", "url": url, "display": display}),
            InlineSpan::Xref { target, display } => serde_json::json!({"type": "xref", "target": target, "display": display}),
            InlineSpan::Strikethrough(spans) => serde_json::json!({"type": "strikethrough", "spans": spans_json(spans)}),
            InlineSpan::Superscript(spans) => serde_json::json!({"type": "superscript", "spans": spans_json(spans)}),
            InlineSpan::Subscript(spans) => serde_json::json!({"type": "subscript", "spans": spans_json(spans)}),
        }
    }

    pub fn plain_text(&self) -> String {
        match self {
            InlineSpan::Text(s) => s.clone(),
            InlineSpan::Bold(spans) | InlineSpan::Italic(spans) |
            InlineSpan::Strikethrough(spans) | InlineSpan::Superscript(spans) |
            InlineSpan::Subscript(spans) => spans.iter().map(|s| s.plain_text()).collect(),
            InlineSpan::Code(s) => s.clone(),
            InlineSpan::Link { display, .. } | InlineSpan::Xref { display, .. } => display.clone(),
        }
    }
}

fn spans_json(spans: &[InlineSpan]) -> Vec<serde_json::Value> {
    spans.iter().map(|s| s.to_json()).collect()
}

/// Parse inline formatting from a text line.
pub fn parse_inline(text: &str) -> Vec<InlineSpan> {
    let mut spans = Vec::new();
    parse_inline_recursive(text, &mut spans, false);
    spans
}

fn try_match(rest: &str, in_nested: bool) -> Option<(InlineSpan, usize)> {
    // xref: xref:Target.adoc[Display]
    if rest.starts_with("xref:") {
        let re = Regex::new(r"^xref:([^\[]+)\[([^\]]*)\]").unwrap();
        if let Some(cap) = re.captures(rest) {
            let target = cap[1].to_string();
            let display = if cap[2].is_empty() { target.clone() } else { cap[2].to_string() };
            let consumed = cap[0].len();
            return Some((InlineSpan::Xref { target, display }, consumed));
        }
    }

    // link: https://...[Display] or link:url[Display]
    if rest.starts_with("http") || rest.starts_with("link:") {
        let re = Regex::new(r"^(?:link:)?(https?://[^\[]+)\[([^\]]*)\]").unwrap();
        if let Some(cap) = re.captures(rest) {
            let url = cap[1].to_string();
            let display = if cap[2].is_empty() { url.clone() } else { cap[2].to_string() };
            let consumed = cap[0].len();
            return Some((InlineSpan::Link { url, display }, consumed));
        }
    }

    // bold: *text*
    if !in_nested && rest.starts_with('*') {
        if let Some(inner) = find_closing(rest, '*', '*') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans, true);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Bold(spans), consumed));
        }
    }

    // italic: _text_
    if !in_nested && rest.starts_with('_') {
        if let Some(inner) = find_closing(rest, '_', '_') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans, true);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Italic(spans), consumed));
        }
    }

    // strikethrough: ~text~
    if rest.starts_with('~') {
        if let Some(inner) = find_closing(rest, '~', '~') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans, true);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Strikethrough(spans), consumed));
        }
    }

    // superscript: ^text^
    if rest.starts_with('^') {
        if let Some(inner) = find_closing(rest, '^', '^') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans, true);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Superscript(spans), consumed));
        }
    }

    // inline code: `code`
    if rest.starts_with('`') {
        if let Some(inner) = find_closing(rest, '`', '`') {
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Code(inner.to_string()), consumed));
        }
    }

    None
}

fn parse_inline_recursive(text: &str, out: &mut Vec<InlineSpan>, in_nested: bool) {
    if text.is_empty() {
        return;
    }

    // Use byte offsets with char_indices — char indices != byte indices for UTF-8.
    let char_positions: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let char_count = char_positions.len();
    let mut pos_idx = 0; // index into char_positions

    while pos_idx < char_count {
        let byte_pos = char_positions[pos_idx];
        let rest = &text[byte_pos..];

        if let Some((span, consumed)) = try_match(rest, in_nested) {
            out.push(span);
            // Advance pos_idx by the number of chars consumed
            let end_byte = byte_pos + consumed;
            while pos_idx < char_count && char_positions[pos_idx] < end_byte {
                pos_idx += 1;
            }
        } else {
            // Accumulate plain text until next potential marker
            let mut end_idx = pos_idx + 1;
            while end_idx < char_count {
                let c = text[char_positions[end_idx]..].chars().next().unwrap();
                if c == '*' || c == '_' || c == '`' || c == '^' || c == '~' {
                    break;
                }
                let ahead = &text[char_positions[end_idx]..];
                if ahead.starts_with("xref:") || ahead.starts_with("link:") || ahead.starts_with("http") {
                    break;
                }
                end_idx += 1;
            }
            let end_byte = if end_idx < char_count { char_positions[end_idx] } else { text.len() };
            out.push(InlineSpan::Text(text[byte_pos..end_byte].to_string()));
            pos_idx = end_idx;
        }
    }
}

fn find_closing(text: &str, open: char, close: char) -> Option<&str> {
    if !text.starts_with(open) {
        return None;
    }
    let rest = &text[open.len_utf8()..];
    // Find the closing marker, but not immediately (empty content)
    let bytes = rest.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == close as u8 {
            if i == 0 {
                return None; // empty content like ** or __
            }
            return Some(&rest[..i]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let spans = parse_inline("hello world");
        assert_eq!(spans, vec![InlineSpan::Text("hello world".into())]);
    }

    #[test]
    fn bold_text() {
        let spans = parse_inline("some *bold* text");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0], InlineSpan::Text("some ".into()));
        assert_eq!(spans[1], InlineSpan::Bold(vec![InlineSpan::Text("bold".into())]));
        assert_eq!(spans[2], InlineSpan::Text(" text".into()));
    }

    #[test]
    fn italic_text() {
        let spans = parse_inline("some _italic_ text");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Italic(vec![InlineSpan::Text("italic".into())]));
    }

    #[test]
    fn inline_code() {
        let spans = parse_inline("use `foo` here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Code("foo".into()));
    }

    #[test]
    fn xref_link() {
        let spans = parse_inline("see xref:Other.adoc[Other Page] for details");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Xref {
            target: "Other.adoc".into(),
            display: "Other Page".into(),
        });
    }

    #[test]
    fn xref_no_display() {
        let spans = parse_inline("xref:Page.adoc[]");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0], InlineSpan::Xref {
            target: "Page.adoc".into(),
            display: "Page.adoc".into(),
        });
    }

    #[test]
    fn external_link() {
        let spans = parse_inline("visit https://example.com[Example] now");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Link {
            url: "https://example.com".into(),
            display: "Example".into(),
        });
    }

    #[test]
    fn mixed_content() {
        let spans = parse_inline("a *b* `c` _d_");
        assert_eq!(spans.len(), 6);
    }

    #[test]
    fn empty_string() {
        let spans = parse_inline("");
        assert!(spans.is_empty());
    }

    #[test]
    fn plain_text_extraction() {
        let span = InlineSpan::Bold(vec![
            InlineSpan::Text("hello".into()),
            InlineSpan::Code("world".into()),
        ]);
        assert_eq!(span.plain_text(), "helloworld");
    }

    #[test]
    fn utf8_em_dash() {
        // The actual crash case from technical-doc.adoc
        let spans = parse_inline("`fishdoc-core` — Pure Rust core library");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], InlineSpan::Code("fishdoc-core".into()));
        assert_eq!(spans[1], InlineSpan::Text(" — Pure Rust core library".into()));
    }

    #[test]
    fn utf8_accented() {
        let spans = parse_inline("café *bold* résumé");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0], InlineSpan::Text("café ".into()));
        assert_eq!(spans[1], InlineSpan::Bold(vec![InlineSpan::Text("bold".into())]));
        assert_eq!(spans[2], InlineSpan::Text(" résumé".into()));
    }
}
