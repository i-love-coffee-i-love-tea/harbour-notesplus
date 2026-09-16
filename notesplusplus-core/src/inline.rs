use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InlineSpan {
    Text(String),
    Bold(Vec<InlineSpan>),
    Italic(Vec<InlineSpan>),
    Code(String),
    Monospace(Vec<InlineSpan>),
    DoubleQuote(Vec<InlineSpan>),
    SingleQuote(Vec<InlineSpan>),
    Link { url: String, display: String },
    Xref { target: String, display: String },
    Strikethrough(Vec<InlineSpan>),
    Superscript(Vec<InlineSpan>),
    Subscript(Vec<InlineSpan>),
    Image { target: String, alt: String, width: Option<String> },
    Icon { name: String, options: Option<String> },
    Footnote { id: Option<String>, text: String },
    Callout(usize),
    Kbd(Vec<String>),
    Button(String),
    Menu(Vec<String>),
    Mark(Vec<InlineSpan>),
    Pass(String),
}

impl InlineSpan {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            InlineSpan::Text(s) => serde_json::json!({"type": "text", "value": s}),
            InlineSpan::Bold(spans) => serde_json::json!({"type": "bold", "spans": spans_json(spans)}),
            InlineSpan::Italic(spans) => serde_json::json!({"type": "italic", "spans": spans_json(spans)}),
            InlineSpan::Code(s) => serde_json::json!({"type": "code", "value": s}),
            InlineSpan::Monospace(spans) => serde_json::json!({"type": "code", "spans": spans_json(spans), "value": spans.iter().map(|s| s.plain_text()).collect::<String>()}),
            InlineSpan::DoubleQuote(spans) => serde_json::json!({"type": "quote", "spans": spans_json(spans)}),
            InlineSpan::SingleQuote(spans) => serde_json::json!({"type": "squote", "spans": spans_json(spans)}),
            InlineSpan::Link { url, display } => serde_json::json!({"type": "link", "url": url, "display": display}),
            InlineSpan::Xref { target, display } => serde_json::json!({"type": "xref", "target": target, "display": display}),
            InlineSpan::Strikethrough(spans) => serde_json::json!({"type": "strikethrough", "spans": spans_json(spans)}),
            InlineSpan::Superscript(spans) => serde_json::json!({"type": "superscript", "spans": spans_json(spans)}),
            InlineSpan::Subscript(spans) => serde_json::json!({"type": "subscript", "spans": spans_json(spans)}),
            InlineSpan::Image { target, alt, width } => serde_json::json!({
                "type": "image",
                "target": target,
                "alt": alt,
                "width": width
            }),
            InlineSpan::Icon { name, options } => serde_json::json!({
                "type": "icon",
                "name": name,
                "options": options,
            }),
            InlineSpan::Footnote { id, text } => serde_json::json!({
                "type": "footnote",
                "id": id,
                "text": text
            }),
            InlineSpan::Callout(num) => serde_json::json!({
                "type": "callout",
                "number": num
            }),
            InlineSpan::Kbd(keys) => serde_json::json!({
                "type": "kbd",
                "keys": keys
            }),
            InlineSpan::Button(text) => serde_json::json!({
                "type": "btn",
                "text": text
            }),
            InlineSpan::Menu(items) => serde_json::json!({
                "type": "menu",
                "items": items
            }),
            InlineSpan::Mark(spans) => serde_json::json!({
                "type": "mark",
                "spans": spans_json(spans)
            }),
            InlineSpan::Pass(val) => serde_json::json!({
                "type": "pass",
                "value": val
            }),
        }
    }

    pub fn plain_text(&self) -> String {
        match self {
            InlineSpan::Text(s) => s.clone(),
            InlineSpan::Bold(spans) | InlineSpan::Italic(spans) |
            InlineSpan::Strikethrough(spans) | InlineSpan::Superscript(spans) |
            InlineSpan::Subscript(spans) | InlineSpan::Monospace(spans) => spans.iter().map(|s| s.plain_text()).collect(),
            InlineSpan::DoubleQuote(spans) => format!("“{}”", spans.iter().map(|s| s.plain_text()).collect::<String>()),
            InlineSpan::SingleQuote(spans) => format!("‘{}’", spans.iter().map(|s| s.plain_text()).collect::<String>()),
            InlineSpan::Code(s) => s.clone(),
            InlineSpan::Link { display, .. } | InlineSpan::Xref { display, .. } => display.clone(),
            InlineSpan::Image { alt, target, .. } => if !alt.is_empty() { alt.clone() } else { target.clone() },
            InlineSpan::Icon { name, .. } => format!(":{}:", name),
            InlineSpan::Footnote { text, .. } => format!("[{}]", text),
            InlineSpan::Callout(num) => format!("<{}>", num),
            InlineSpan::Mark(spans) => spans.iter().map(|s| s.plain_text()).collect(),
            InlineSpan::Kbd(keys) => keys.join("+"),
            InlineSpan::Button(text) => format!("[{}]", text),
            InlineSpan::Menu(items) => items.join(" > "),
            InlineSpan::Pass(val) => val.clone(),
        }
    }
}

fn spans_json(spans: &[InlineSpan]) -> Vec<serde_json::Value> {
    spans.iter().map(|s| s.to_json()).collect()
}

pub fn decode_html_entities(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut found_semi = false;
            while let Some(&next_c) = chars.peek() {
                if next_c == ';' {
                    chars.next();
                    found_semi = true;
                    break;
                } else if next_c.is_alphanumeric() || next_c == '#' {
                    entity.push(chars.next().unwrap());
                    if entity.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if found_semi {
                if let Some(stripped_hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                    if let Ok(code) = u32::from_str_radix(stripped_hex, 16) {
                        if let Some(decoded_char) = char::from_u32(code) {
                            result.push(decoded_char);
                            continue;
                        }
                    }
                } else if let Some(stripped_dec) = entity.strip_prefix('#') {
                    if let Ok(code) = stripped_dec.parse::<u32>() {
                        if let Some(decoded_char) = char::from_u32(code) {
                            result.push(decoded_char);
                            continue;
                        }
                    }
                } else {
                    match entity.as_str() {
                        "euro" => { result.push('€'); continue; }
                        "amp" => { result.push('&'); continue; }
                        "lt" => { result.push('<'); continue; }
                        "gt" => { result.push('>'); continue; }
                        "quot" => { result.push('"'); continue; }
                        "apos" => { result.push('\''); continue; }
                        "nbsp" => { result.push('\u{00A0}'); continue; }
                        "copy" => { result.push('©'); continue; }
                        "reg" => { result.push('®'); continue; }
                        "trade" => { result.push('™'); continue; }
                        "mdash" => { result.push('—'); continue; }
                        "ndash" => { result.push('–'); continue; }
                        "hellip" => { result.push('…'); continue; }
                        "bull" => { result.push('•'); continue; }
                        _ => {}
                    }
                }
                result.push('&');
                result.push_str(&entity);
                result.push(';');
            } else {
                result.push('&');
                result.push_str(&entity);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse inline formatting from a text line.
pub fn parse_inline(text: &str) -> Vec<InlineSpan> {
    let mut spans = Vec::new();
    if text.contains('&') {
        let decoded = decode_html_entities(text);
        parse_inline_recursive(&decoded, &mut spans);
    } else {
        parse_inline_recursive(text, &mut spans);
    }
    spans
}

fn try_match(rest: &str) -> Option<(InlineSpan, usize)> {
    if let Some(res) = match_index_or_anchor(rest) {
        return Some(res);
    }
    if let Some(res) = match_ui_macros(rest) {
        return Some(res);
    }
    if let Some(res) = match_passthrough(rest) {
        return Some(res);
    }
    if let Some(res) = match_media_and_links(rest) {
        return Some(res);
    }
    if let Some(res) = match_formatting_spans(rest) {
        return Some(res);
    }
    None
}

fn match_index_or_anchor(rest: &str) -> Option<(InlineSpan, usize)> {
    // Hidden index term: (((term1,term2,term3)))
    if rest.starts_with("(((") {
        if let Some(end) = rest.find(")))") {
            let consumed = end + 3;
            return Some((InlineSpan::Text(String::new()), consumed));
        }
    }

    // Visible inline index term: ((term))
    if rest.starts_with("((") {
        if let Some(end) = rest.find("))") {
            let inner = &rest[2..end];
            let consumed = end + 2;
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
            return Some((InlineSpan::Text(text), consumed));
        }
    }

    // Shorthand xref: <<target,display>> or <<target>>
    if rest.starts_with("<<") {
        if let Some(end) = rest.find(">>") {
            let inner = &rest[2..end];
            let consumed = end + 2;
            if let Some((target, disp)) = inner.split_once(',') {
                return Some((InlineSpan::Xref {
                    target: target.trim().to_string(),
                    display: disp.trim().to_string(),
                }, consumed));
            } else {
                return Some((InlineSpan::Xref {
                    target: inner.trim().to_string(),
                    display: inner.trim().to_string(),
                }, consumed));
            }
        }
    }

    // Inline anchor: [[id,label]] or [[id]]
    if rest.starts_with("[[") {
        if let Some(end) = rest.find("]]") {
            let inner = &rest[2..end];
            let consumed = end + 2;
            if let Some((_id, disp)) = inner.split_once(',') {
                let mut spans = Vec::new();
                parse_inline_recursive(disp.trim(), &mut spans);
                let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                return Some((InlineSpan::Text(text), consumed));
            } else {
                return Some((InlineSpan::Text(String::new()), consumed));
            }
        }
    }

    None
}

fn match_ui_macros(rest: &str) -> Option<(InlineSpan, usize)> {
    // kbd:[Ctrl,Alt,Backspace] or kbd:[Ctrl+T]
    if rest.starts_with("kbd:[") {
        if let Some(end) = rest.find(']') {
            let inner = &rest[5..end];
            let keys: Vec<String> = if inner.contains('+') {
                inner.split('+').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect()
            } else {
                inner.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect()
            };
            let consumed = end + 1;
            return Some((InlineSpan::Kbd(keys), consumed));
        }
    }

    // btn:[Download Zip]
    if rest.starts_with("btn:[") {
        if let Some(end) = rest.find(']') {
            let text = rest[5..end].trim().to_string();
            let consumed = end + 1;
            return Some((InlineSpan::Button(text), consumed));
        }
    }

    // menu:File[Quit] or menu:View[Zoom > Reset]
    if let Some(after) = rest.strip_prefix("menu:") {
        if let Some(open) = after.find('[') {
            if let Some(close) = after[open..].find(']') {
                let top = &after[..open];
                let sub = &after[open + 1..open + close];
                let mut items = Vec::new();
                if !top.is_empty() {
                    items.push(top.to_string());
                }
                if !sub.is_empty() {
                    for item in sub.split('>') {
                        let t = item.trim();
                        if !t.is_empty() {
                            items.push(t.to_string());
                        }
                    }
                }
                let consumed = 5 + open + close + 1;
                return Some((InlineSpan::Menu(items), consumed));
            }
        }
    }

    None
}

fn match_passthrough(rest: &str) -> Option<(InlineSpan, usize)> {
    // +++raw_html+++
    if let Some(after) = rest.strip_prefix("+++") {
        if let Some(end) = after.find("+++") {
            let html = after[..end].to_string();
            let consumed = 3 + end + 3;
            return Some((InlineSpan::Pass(html), consumed));
        }
    }

    // pass:[raw_html]
    if rest.starts_with("pass:[") {
        if let Some(end) = rest.find(']') {
            let html = rest[6..end].to_string();
            let consumed = end + 1;
            return Some((InlineSpan::Pass(html), consumed));
        }
    }

    None
}

fn match_media_and_links(rest: &str) -> Option<(InlineSpan, usize)> {
    // image: image:path.png[Alt text, 100] (not image:: which is block)
    if rest.starts_with("image:") && !rest.starts_with("image::") {
        if let Some(open) = rest.find('[') {
            if let Some(close) = rest[open..].find(']') {
                let target = rest[6..open].to_string();
                let attrs_str = &rest[open + 1..open + close];
                let (alt, width) = parse_image_attrs(attrs_str);
                let consumed = open + close + 1;
                return Some((InlineSpan::Image { target, alt, width }, consumed));
            }
        }
    }

    // icon: icon:name[attrs] or icon:name[]
    if let Some(rest_icon) = rest.strip_prefix("icon:") {
        if let Some(open) = rest_icon.find('[') {
            let name = &rest_icon[..open];
            if !name.is_empty() && !name.contains(char::is_whitespace) && !name.contains(':') {
                if let Some(close) = rest_icon[open..].find(']') {
                    let opts_str = &rest_icon[open + 1..open + close];
                    let options = if opts_str.is_empty() { None } else { Some(opts_str.to_string()) };
                    let consumed = 5 + open + close + 1;
                    return Some((InlineSpan::Icon { name: name.to_string(), options }, consumed));
                }
            }
        }
    }

    // stem: stem:[formula] or asciimath:[formula] or latexmath:[formula]
    if rest.starts_with("stem:") || rest.starts_with("asciimath:") || rest.starts_with("latexmath:") {
        let prefix_len = if rest.starts_with("stem:") { 5 } else { 10 };
        let after_prefix = &rest[prefix_len..];
        if let Some(open) = after_prefix.find('[') {
            if let Some(close) = after_prefix[open..].find(']') {
                let formula = after_prefix[open + 1..open + close].to_string();
                let consumed = prefix_len + open + close + 1;
                return Some((InlineSpan::Code(formula), consumed));
            }
        }
    }

    // footnote: footnote:[Text] or footnote:id[Text] or footnoteref:[id, Text]
    if rest.starts_with("footnote:") || rest.starts_with("footnoteref:") {
        let is_ref = rest.starts_with("footnoteref:");
        let prefix_len = if is_ref { 12 } else { 9 };
        let after_prefix = &rest[prefix_len..];
        if let Some(open) = after_prefix.find('[') {
            if let Some(close) = after_prefix[open..].find(']') {
                let id_part = &after_prefix[..open];
                let id = if id_part.is_empty() { None } else { Some(id_part.to_string()) };
                let text = after_prefix[open + 1..open + close].to_string();
                let consumed = prefix_len + open + close + 1;
                return Some((InlineSpan::Footnote { id, text }, consumed));
            }
        }
    }

    // callout: <1>, <2>, etc.
    if rest.starts_with('<') {
        if let Some(close) = rest.find('>') {
            let num_str = &rest[1..close];
            if let Ok(num) = num_str.parse::<usize>() {
                let consumed = close + 1;
                return Some((InlineSpan::Callout(num), consumed));
            }
        }
    }

    // xref: xref:Target.adoc[Display]
    if let Some(after_prefix) = rest.strip_prefix("xref:") {
        if let Some(open) = after_prefix.find('[') {
            if let Some(close) = after_prefix[open..].find(']') {
                let target = after_prefix[..open].to_string();
                let disp = &after_prefix[open + 1..open + close];
                let display = if disp.is_empty() { target.clone() } else { disp.to_string() };
                let consumed = 5 + open + close + 1;
                return Some((InlineSpan::Xref { target, display }, consumed));
            }
        }
    }

    // link: https://...[Display] or link:url[Display]
    if rest.starts_with("http://") || rest.starts_with("https://") || rest.starts_with("link:") {
        let is_link_prefix = rest.starts_with("link:");
        let url_start = if is_link_prefix { 5 } else { 0 };
        let after_prefix = &rest[url_start..];
        if let Some(open) = after_prefix.find('[') {
            if let Some(close) = after_prefix[open..].find(']') {
                let url = after_prefix[..open].to_string();
                let disp = &after_prefix[open + 1..open + close];
                let display = if disp.is_empty() { url.clone() } else { disp.to_string() };
                let consumed = url_start + open + close + 1;
                return Some((InlineSpan::Link { url, display }, consumed));
            }
        }
    }

    None
}

fn match_formatting_spans(rest: &str) -> Option<(InlineSpan, usize)> {
    // [.role]#text#
    if rest.starts_with("[.") {
        if let Some(cb) = rest.find("]#") {
            if let Some(end) = rest[cb + 2..].find('#') {
                let inner = &rest[cb + 2..cb + 2 + end];
                let mut spans = Vec::new();
                parse_inline_recursive(inner, &mut spans);
                let consumed = cb + 2 + end + 1;
                return Some((InlineSpan::Mark(spans), consumed));
            }
        }
    }

    // #highlighted text#
    if rest.starts_with('#') && rest.len() > 1 && !rest.starts_with("##") {
        if let Some(inner) = find_closing(rest, '#', '#') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Mark(spans), consumed));
        }
    }

    // Unconstrained bold: **text**
    if let Some(after) = rest.strip_prefix("**") {
        if let Some(end) = after.find("**") {
            let inner = &after[..end];
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = end + 4;
            return Some((InlineSpan::Bold(spans), consumed));
        }
    }

    // Unconstrained italic: __text__
    if let Some(after) = rest.strip_prefix("__") {
        if let Some(end) = after.find("__") {
            let inner = &after[..end];
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = end + 4;
            return Some((InlineSpan::Italic(spans), consumed));
        }
    }

    // Unconstrained code: ``text``
    if let Some(after) = rest.strip_prefix("``") {
        if let Some(end) = after.find("``") {
            let inner = &after[..end];
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = end + 4;
            let has_formatting = spans.iter().any(|s| !matches!(s, InlineSpan::Text(_)));
            if has_formatting {
                return Some((InlineSpan::Monospace(spans), consumed));
            } else {
                let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                return Some((InlineSpan::Code(text), consumed));
            }
        }
    }

    // Smart double quotes: "`text`"
    if let Some(after) = rest.strip_prefix("\"`") {
        if let Some(end) = after.find("`\"") {
            let inner = &after[..end];
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = end + 4;
            return Some((InlineSpan::DoubleQuote(spans), consumed));
        }
    }

    // Smart single quotes: '`text`'
    if let Some(after) = rest.strip_prefix("'`") {
        if let Some(end) = after.find("`'") {
            let inner = &after[..end];
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = end + 4;
            return Some((InlineSpan::SingleQuote(spans), consumed));
        }
    }

    // bold: *text*
    if rest.starts_with('*') {
        if let Some(inner) = find_closing(rest, '*', '*') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Bold(spans), consumed));
        }
    }

    // italic: _text_
    if rest.starts_with('_') {
        if let Some(inner) = find_closing(rest, '_', '_') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Italic(spans), consumed));
        }
    }

    // strikethrough: ~text~
    if rest.starts_with('~') {
        if let Some(inner) = find_closing(rest, '~', '~') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Strikethrough(spans), consumed));
        }
    }

    // superscript: ^text^
    if rest.starts_with('^') {
        if let Some(inner) = find_closing(rest, '^', '^') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            return Some((InlineSpan::Superscript(spans), consumed));
        }
    }

    // inline code: `code`
    if rest.starts_with('`') {
        if let Some(inner) = find_closing(rest, '`', '`') {
            let mut spans = Vec::new();
            parse_inline_recursive(inner, &mut spans);
            let consumed = inner.len() + 2;
            let has_formatting = spans.iter().any(|s| !matches!(s, InlineSpan::Text(_)));
            if has_formatting {
                return Some((InlineSpan::Monospace(spans), consumed));
            } else {
                let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                return Some((InlineSpan::Code(text), consumed));
            }
        }
    }

    None
}

fn push_text(out: &mut Vec<InlineSpan>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(InlineSpan::Text(last)) = out.last_mut() {
        last.push_str(text);
    } else {
        out.push(InlineSpan::Text(text.to_string()));
    }
}

fn parse_inline_recursive(text: &str, out: &mut Vec<InlineSpan>) {
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

        // Backslash escape: \* \_ \` \^ \~ \xref \link \http → literal next char/sequence
        if rest.starts_with('\\') && rest.len() > 1 {
            let next_char = rest[1..].chars().next().unwrap();
            if next_char == '*' || next_char == '_' || next_char == '`' || next_char == '^' || next_char == '~' || next_char == '\\' {
                let mut char_buf = [0u8; 4];
                push_text(out, next_char.encode_utf8(&mut char_buf));
                pos_idx += 2; // skip \ and the escaped char
                continue;
            }
        }

        if let Some((span, consumed)) = try_match(rest) {
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
                if c == '*' || c == '_' || c == '`' || c == '^' || c == '~' || c == '#' || c == '"' || c == '\'' {
                    break;
                }
                // Only break on \ if it's an escape (followed by a special char)
                if c == '\\' && end_idx + 1 < char_count {
                    let next = text[char_positions[end_idx + 1]..].chars().next().unwrap();
                    if next == '*' || next == '_' || next == '`' || next == '^' || next == '~' || next == '#' || next == '\\' {
                        break;
                    }
                }
                let ahead = &text[char_positions[end_idx]..];
                if ahead.starts_with("xref:") || ahead.starts_with("link:") || ahead.starts_with("http")
                    || ahead.starts_with("image:") || ahead.starts_with("icon:") || ahead.starts_with("footnote:")
                    || ahead.starts_with("footnoteref:") || ahead.starts_with('<')
                    || ahead.starts_with("((") || ahead.starts_with("<<") || ahead.starts_with("[[")
                    || ahead.starts_with("kbd:[") || ahead.starts_with("btn:[") || ahead.starts_with("menu:")
                    || ahead.starts_with("stem:") || ahead.starts_with("asciimath:") || ahead.starts_with("latexmath:")
                    || ahead.starts_with("+++") || ahead.starts_with("pass:[") || ahead.starts_with("[.") {
                    break;
                }
                end_idx += 1;
            }
            let end_byte = if end_idx < char_count { char_positions[end_idx] } else { text.len() };
            push_text(out, &text[byte_pos..end_byte]);
            pos_idx = end_idx;
        }
    }
}

fn parse_image_attrs(attr_str: &str) -> (String, Option<String>) {
    let attr_str = attr_str.trim();
    if attr_str.is_empty() {
        return (String::new(), None);
    }
    let parts: Vec<&str> = attr_str.split(',').map(|s| s.trim()).collect();
    let mut alt = parts[0].to_string();
    let mut width = None;
    if alt.starts_with("alt=") {
        alt = alt.strip_prefix("alt=").unwrap().trim_matches('"').trim_matches('\'').to_string();
    }
    for part in &parts[1..] {
        if part.starts_with("width=") {
            width = Some(part.strip_prefix("width=").unwrap().trim_matches('"').trim_matches('\'').to_string());
        } else if part.chars().all(|c| c.is_ascii_digit()) && width.is_none() {
            width = Some(part.to_string());
        }
    }
    (alt, width)
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
        let spans = parse_inline("`notesplusplus-core` — Pure Rust core library");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], InlineSpan::Code("notesplusplus-core".into()));
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

    #[test]
    fn backslash_escape_underscore() {
        let spans = parse_inline("\\_not italic_");
        assert_eq!(spans, vec![
            InlineSpan::Text("_not italic_".into()),
        ]);
    }

    #[test]
    fn backslash_escape_asterisk() {
        let spans = parse_inline("\\*not bold\\*");
        assert_eq!(spans, vec![
            InlineSpan::Text("*not bold*".into()),
        ]);
    }

    #[test]
    fn bold_italic_nested() {
        let spans = parse_inline("*_bold italic_*");
        assert_eq!(spans, vec![
            InlineSpan::Bold(vec![
                InlineSpan::Italic(vec![
                    InlineSpan::Text("bold italic".into()),
                ])
            ])
        ]);

        let spans2 = parse_inline("_*italic bold*_");
        assert_eq!(spans2, vec![
            InlineSpan::Italic(vec![
                InlineSpan::Bold(vec![
                    InlineSpan::Text("italic bold".into()),
                ])
            ])
        ]);
    }

    #[test]
    fn smart_quotes_with_formatted_monospace() {
        let spans = parse_inline("'```e = mc^2^ *_the scent of science_* _smells like a genius_```'");
        assert_eq!(spans, vec![
            InlineSpan::SingleQuote(vec![
                InlineSpan::Monospace(vec![
                    InlineSpan::Text("e = mc".into()),
                    InlineSpan::Superscript(vec![InlineSpan::Text("2".into())]),
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Bold(vec![
                        InlineSpan::Italic(vec![
                            InlineSpan::Text("the scent of science".into()),
                        ])
                    ]),
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Italic(vec![
                        InlineSpan::Text("smells like a genius".into()),
                    ]),
                ])
            ])
        ]);
    }

    #[test]
    fn backslash_in_normal_text() {
        let spans = parse_inline("path\\to\\file");
        assert_eq!(spans, vec![InlineSpan::Text("path\\to\\file".into())]);
    }

    #[test]
    fn inline_image() {
        let spans = parse_inline("Look at image:logo.png[App Logo, width=64] here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Image {
            target: "logo.png".into(),
            alt: "App Logo".into(),
            width: Some("64".into()),
        });
    }

    #[test]
    fn inline_footnote() {
        let spans = parse_inline("Statement footnote:[See official docs for details] continues.");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Footnote {
            id: None,
            text: "See official docs for details".into(),
        });
    }

    #[test]
    fn inline_footnote_with_id() {
        let spans = parse_inline("Statement footnote:fn1[Custom footnote text] continues.");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Footnote {
            id: Some("fn1".into()),
            text: "Custom footnote text".into(),
        });
    }

    #[test]
    fn inline_footnoteref() {
        let spans = parse_inline("Referencing footnoteref:[fn1, Reused text] again.");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], InlineSpan::Footnote {
            id: None,
            text: "fn1, Reused text".into(),
        });
    }

    #[test]
    fn inline_image_bare_and_positional_width() {
        let s1 = parse_inline("image:banner.png[]");
        assert_eq!(s1, vec![InlineSpan::Image {
            target: "banner.png".into(),
            alt: "".into(),
            width: None,
        }]);

        let s2 = parse_inline("image:chart.png[Performance, 300]");
        assert_eq!(s2, vec![InlineSpan::Image {
            target: "chart.png".into(),
            alt: "Performance".into(),
            width: Some("300".into()),
        }]);
    }

    #[test]
    fn inline_callout() {
        let spans = parse_inline("let x = 42; <1>");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[1], InlineSpan::Callout(1));
    }

    #[test]
    fn inline_plain_text_new_spans() {
        let img = InlineSpan::Image { target: "a.png".into(), alt: "Photo".into(), width: None };
        assert_eq!(img.plain_text(), "Photo");

        let fn_span = InlineSpan::Footnote { id: None, text: "note".into() };
        assert_eq!(fn_span.plain_text(), "[note]");

        let callout = InlineSpan::Callout(3);
        assert_eq!(callout.plain_text(), "<3>");

        let icon = InlineSpan::Icon { name: "heart".into(), options: Some("role=love".into()) };
        assert_eq!(icon.plain_text(), ":heart:");
    }

    #[test]
    fn inline_icon() {
        let spans = parse_inline("Brought to you with icon:heart[set=fas,role=love] and icon:star[] today");
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[0], InlineSpan::Text("Brought to you with ".into()));
        assert_eq!(spans[1], InlineSpan::Icon {
            name: "heart".into(),
            options: Some("set=fas,role=love".into()),
        });
        assert_eq!(spans[2], InlineSpan::Text(" and ".into()));
        assert_eq!(spans[3], InlineSpan::Icon {
            name: "star".into(),
            options: None,
        });
        assert_eq!(spans[4], InlineSpan::Text(" today".into()));
    }

    #[test]
    fn inline_indexterms() {
        let spans = parse_inline("At ((Devoxx)) (((Conference,Devoxx))) we met ((Antwerp)).");
        let plain = spans.iter().map(|s| s.plain_text()).collect::<String>();
        assert_eq!(plain, "At Devoxx  we met Antwerp.");
    }

    #[test]
    fn inline_shorthand_xref() {
        let s1 = parse_inline("See <<ravages>> for details");
        assert_eq!(s1[1], InlineSpan::Xref { target: "ravages".into(), display: "ravages".into() });

        let s2 = parse_inline("At <<bier-central,Bier Central>> today");
        assert_eq!(s2[1], InlineSpan::Xref { target: "bier-central".into(), display: "Bier Central".into() });
    }

    #[test]
    fn inline_anchors() {
        let spans = parse_inline("Come on, [[bier-central,Bier Central]]_Bier Central_, of course!");
        let plain = spans.iter().map(|s| s.plain_text()).collect::<String>();
        assert!(plain.contains("Bier Central"));
    }

    #[test]
    fn inline_ui_macros() {
        let s = parse_inline("Quick, hit kbd:[Ctrl,Alt,Backspace] or select menu:File[Quit] and click btn:[Download Zip]!");
        assert_eq!(s[1], InlineSpan::Kbd(vec!["Ctrl".into(), "Alt".into(), "Backspace".into()]));
        assert_eq!(s[3], InlineSpan::Menu(vec!["File".into(), "Quit".into()]));
        assert_eq!(s[5], InlineSpan::Button("Download Zip".into()));
    }

    #[test]
    fn inline_pass_html() {
        let spans = parse_inline("Roses are +++<span style=\"color: #FF0000\">red</span>+++.");
        assert_eq!(spans[1], InlineSpan::Pass("<span style=\"color: #FF0000\">red</span>".into()));
    }

    #[test]
    fn inline_html_entities() {
        let spans = parse_inline("Price: 100&#x20ac; &amp; 50&euro; for &copy; 2026");
        let plain = spans.iter().map(|s| s.plain_text()).collect::<String>();
        assert_eq!(plain, "Price: 100€ & 50€ for © 2026");
    }

    #[test]
    fn inline_math_stem() {
        let spans = parse_inline("Formula: stem:[sqrt(4) = 2] or asciimath:[x^2 + y^2 = r^2]");
        assert_eq!(spans[1], InlineSpan::Code("sqrt(4) = 2".into()));
        assert_eq!(spans[3], InlineSpan::Code("x^2 + y^2 = r^2".into()));
    }
}
