/// Preprocesses raw HTML into clean, readable structured text with reduced markup.
///
/// Preserves headings, paragraphs, lists, tables, blockquotes, code blocks, links, and bold/italic formatting,
/// while stripping `<script>`, `<style>`, `<svg>`, `<noscript>`, `<nav>`, `<footer>`, `<header>`, `<aside>`,
/// comments, and extraneous attributes.
pub fn preprocess_html(input: &str) -> String {
    if !looks_like_html(input) {
        return input.trim().to_string();
    }

    // 1. Extract title if present in <title>...</title>
    let extracted_title = extract_title(input);

    // 2. Remove noise blocks (tag and entire inner contents)
    let cleaned_html = strip_noise_blocks(input);

    // 3. Convert HTML structure to clean markdown/text
    let mut converter = HtmlConverter::new();
    let converted = converter.convert(&cleaned_html);

    // 4. Post-process whitespace and collapse blank lines
    let mut result = post_process(&converted);

    // 5. If a title was extracted and document doesn't start with a heading, prepend it
    if let Some(title) = extracted_title {
        let trimmed_title = title.trim();
        if !trimmed_title.is_empty() {
            let starts_with_heading = result.starts_with('#')
                || result.starts_with('=')
                || result.lines().next().map(|l| l.trim().starts_with('#') || l.trim().starts_with('=')).unwrap_or(false);
            if !starts_with_heading {
                result = format!("# {}\n\n{}", trimmed_title, result);
            }
        }
    }

    result.trim().to_string()
}

/// Returns true if the string appears to be HTML content.
pub fn looks_like_html(s: &str) -> bool {
    let lower = s.trim_start().to_ascii_lowercase();
    if lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || lower.starts_with("<?xml")
        || lower.starts_with("<head")
        || lower.starts_with("<body")
    {
        return true;
    }

    // Check for common HTML opening/closing tag patterns
    let html_tag_patterns = [
        "<p>", "<p ", "</p>",
        "<div>", "<div ", "</div>",
        "<h1>", "<h1 ", "</h1>",
        "<h2>", "<h2 ", "</h2>",
        "<h3>", "<h3 ", "</h3>",
        "<ul>", "<ul ", "</ul>",
        "<ol>", "<ol ", "</ol>",
        "<li>", "<li ", "</li>",
        "<table>", "<table ", "</table>",
        "<article>", "<article ", "</article>",
        "<section>", "<section ", "</section>",
        "<main>", "<main ", "</main>",
        "<script", "<style", "<meta ", "<link ",
    ];

    html_tag_patterns.iter().any(|&pattern| lower.contains(pattern))
}

/// Extracts the text inside `<title>...</title>`.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start_tag = "<title";
    let end_tag = "</title>";

    if let Some(start_idx) = lower.find(start_tag) {
        if let Some(tag_close) = html[start_idx..].find('>') {
            let content_start = start_idx + tag_close + 1;
            if let Some(end_idx) = lower[content_start..].find(end_tag) {
                let raw_title = &html[content_start..content_start + end_idx];
                let decoded = decode_html_entities(raw_title);
                let cleaned = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

/// Strips tags and contents for tags that contain non-content boilerplate.
fn strip_noise_blocks(html: &str) -> String {
    let mut result = html.to_string();

    // Remove HTML comments: <!-- ... -->
    while let Some(start) = result.find("<!--") {
        if let Some(end) = result[start..].find("-->") {
            result.replace_range(start..start + end + 3, " ");
        } else {
            result.truncate(start);
            break;
        }
    }

    // Strip noise tags: <tag ...> ... </tag>
    let noise_tags = [
        "script",
        "style",
        "svg",
        "noscript",
        "template",
        "iframe",
        "canvas",
        "audio",
        "video",
        "form",
        "dialog",
        "nav",
        "footer",
        "aside",
        "head",
    ];

    for tag in &noise_tags {
        result = strip_tag_and_contents(&result, tag);
    }

    result
}

/// Helper to remove all occurrences of `<tag ...> ... </tag>` case-insensitively.
fn strip_tag_and_contents(html: &str, tag_name: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();
    let len = bytes.len();

    let open_prefix = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);

    while cursor < len {
        let rest = &html[cursor..];
        let rest_lower = rest.to_ascii_lowercase();

        if let Some(pos) = rest_lower.find(&open_prefix) {
            let tag_start = cursor + pos;
            out.push_str(&html[cursor..tag_start]);

            // Check if it's a true tag match (next char is whitespace, '>', or '/')
            let after_name_idx = tag_start + open_prefix.len();
            if after_name_idx < len {
                let next_char = bytes[after_name_idx];
                if next_char == b'>' || next_char == b'/' || next_char.is_ascii_whitespace() {
                    // Find the end of opening tag
                    if let Some(open_close_idx) = html[tag_start..].find('>') {
                        let open_tag_end = tag_start + open_close_idx + 1;
                        // Check for self-closing tag e.g. <script ... />
                        if html[tag_start..open_tag_end].ends_with("/>") {
                            cursor = open_tag_end;
                            continue;
                        }

                        // Find matching closing tag </tag_name>
                        let search_from = open_tag_end;
                        let search_rest_lower = html[search_from..].to_ascii_lowercase();
                        if let Some(close_pos) = search_rest_lower.find(&close_tag) {
                            cursor = search_from + close_pos + close_tag.len();
                            continue;
                        } else {
                            // No closing tag found, skip the opening tag
                            cursor = open_tag_end;
                            continue;
                        }
                    }
                }
            }

            // Not a tag match, output the prefix and continue
            out.push_str(&html[tag_start..tag_start + 1]);
            cursor = tag_start + 1;
        } else {
            out.push_str(rest);
            break;
        }
    }

    out
}

struct ListState {
    is_ordered: bool,
    counter: usize,
}

struct TableCell {
    is_header: bool,
    text: String,
}

struct TableState {
    rows: Vec<Vec<TableCell>>,
    current_row: Vec<TableCell>,
    in_cell: bool,
    cell_is_header: bool,
    current_cell_text: String,
}

struct HtmlConverter {
    output: String,
    list_stack: Vec<ListState>,
    table_stack: Vec<TableState>,
    heading_level: Option<usize>,
    blockquote_depth: usize,
    in_link: Option<(String, String)>, // (href, accumulated_text)
    in_dt: bool,
    in_dd: bool,
}

impl HtmlConverter {
    fn new() -> Self {
        Self {
            output: String::with_capacity(4096),
            list_stack: Vec::new(),
            table_stack: Vec::new(),
            heading_level: None,
            blockquote_depth: 0,
            in_link: None,
            in_dt: false,
            in_dd: false,
        }
    }

    fn convert(&mut self, html: &str) -> String {
        let mut cursor = 0;
        let len = html.len();

        while cursor < len {
            if let Some(tag_start) = html[cursor..].find('<') {
                let text_chunk = &html[cursor..cursor + tag_start];
                self.handle_text(text_chunk);

                let abs_tag_start = cursor + tag_start;
                if let Some(tag_end) = html[abs_tag_start..].find('>') {
                    let raw_tag = &html[abs_tag_start + 1..abs_tag_start + tag_end];
                    let trimmed = raw_tag.trim();
                    let tag_name = trimmed.split_whitespace().next().unwrap_or("").to_ascii_lowercase();

                    if tag_name == "pre" && !trimmed.starts_with('/') {
                        let pre_tag_end = abs_tag_start + tag_end + 1;
                        if let Some(end_idx) = html[pre_tag_end..].to_ascii_lowercase().find("</pre>") {
                            let mut pre_body = &html[pre_tag_end..pre_tag_end + end_idx];

                            // Check language on <pre ...>
                            let mut lang = extract_attr(trimmed, "data-lang")
                                .or_else(|| extract_attr(trimmed, "class").and_then(|c| parse_code_language(&c)));

                            // If pre_body starts with <code ...>, extract lang and strip <code> and </code>
                            let trimmed_pre = pre_body.trim_start();
                            if trimmed_pre.to_ascii_lowercase().starts_with("<code") {
                                if let Some(code_tag_end) = trimmed_pre.find('>') {
                                    let code_raw_tag = &trimmed_pre[1..code_tag_end];
                                    if lang.is_none() {
                                        lang = extract_attr(code_raw_tag, "data-lang")
                                            .or_else(|| extract_attr(code_raw_tag, "class").and_then(|c| parse_code_language(&c)));
                                    }
                                    let after_code = &trimmed_pre[code_tag_end + 1..];
                                    if let Some(code_close_idx) = after_code.to_ascii_lowercase().rfind("</code>") {
                                        pre_body = &after_code[..code_close_idx];
                                    } else {
                                        pre_body = after_code;
                                    }
                                }
                            }

                            let decoded = decode_html_entities(pre_body);
                            self.ensure_newline(2);
                            if let Some(l) = lang {
                                self.output.push_str(&format!("```{}\n{}\n```\n\n", l, decoded.trim_matches('\n')));
                            } else {
                                self.output.push_str(&format!("```\n{}\n```\n\n", decoded.trim_matches('\n')));
                            }

                            cursor = pre_tag_end + end_idx + 6; // skip </pre>
                            continue;
                        }
                    }

                    self.handle_tag(raw_tag);
                    cursor = abs_tag_start + tag_end + 1;
                } else {
                    // Unclosed tag at end of string
                    break;
                }
            } else {
                let text_chunk = &html[cursor..];
                self.handle_text(text_chunk);
                break;
            }
        }

        // Close any dangling link
        if let Some((href, text)) = self.in_link.take() {
            self.emit_link(&href, &text);
        }

        self.output.clone()
    }

    fn handle_text(&mut self, raw: &str) {
        if raw.is_empty() {
            return;
        }

        let decoded = decode_html_entities(raw);

        if let Some((_, ref mut link_text)) = self.in_link {
            link_text.push_str(&decoded);
            return;
        }

        if let Some(table) = self.table_stack.last_mut() {
            if table.in_cell {
                table.current_cell_text.push_str(&decoded);
                return;
            }
        }

        let collapsed = collapse_whitespace(&decoded);
        self.output.push_str(&collapsed);
    }

    fn handle_tag(&mut self, raw_tag: &str) {
        let trimmed = raw_tag.trim();
        if trimmed.is_empty() {
            return;
        }

        let is_closing = trimmed.starts_with('/');
        let tag_body = if is_closing {
            trimmed[1..].trim()
        } else {
            trimmed.trim_end_matches('/')
        };

        let mut parts = tag_body.split_whitespace();
        let tag_name = parts.next().unwrap_or("").to_ascii_lowercase();

        if is_closing {
            self.handle_close_tag(&tag_name);
        } else {
            let is_self_closing = trimmed.ends_with('/') || matches!(tag_name.as_str(), "br" | "hr" | "img" | "input" | "meta" | "link");
            self.handle_open_tag(&tag_name, trimmed, is_self_closing);
        }
    }

    fn handle_open_tag(&mut self, tag: &str, raw_tag: &str, is_self_closing: bool) {
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag[1..].parse::<usize>().unwrap_or(1);
                self.ensure_newline(2);
                self.heading_level = Some(level);
                let prefix = "#".repeat(level);
                self.output.push_str(&prefix);
                self.output.push(' ');
            }
            "p" => {
                self.ensure_newline(2);
                if self.blockquote_depth > 0 {
                    self.output.push_str("> ");
                }
            }
            "br" => {
                self.output.push('\n');
                if self.blockquote_depth > 0 {
                    self.output.push_str("> ");
                }
            }
            "hr" => {
                self.ensure_newline(2);
                self.output.push_str("---\n\n");
            }
            "ul" => {
                self.ensure_newline(1);
                self.list_stack.push(ListState {
                    is_ordered: false,
                    counter: 0,
                });
            }
            "ol" => {
                self.ensure_newline(1);
                let start_val = extract_attr(raw_tag, "start")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(1);
                self.list_stack.push(ListState {
                    is_ordered: true,
                    counter: start_val,
                });
            }
            "li" => {
                self.ensure_newline(1);
                let depth = self.list_stack.len().saturating_sub(1);
                let indent = "  ".repeat(depth);
                self.output.push_str(&indent);

                if let Some(list) = self.list_stack.last_mut() {
                    if list.is_ordered {
                        let num = list.counter;
                        list.counter += 1;
                        self.output.push_str(&format!("{}. ", num));
                    } else {
                        self.output.push_str("* ");
                    }
                } else {
                    self.output.push_str("* ");
                }
            }
            "dl" => {
                self.ensure_newline(1);
            }
            "dt" => {
                self.ensure_newline(2);
                self.in_dt = true;
                self.output.push_str("**");
            }
            "dd" => {
                self.ensure_newline(1);
                self.in_dd = true;
                self.output.push_str("  ");
            }
            "blockquote" => {
                self.ensure_newline(2);
                self.blockquote_depth += 1;
                self.output.push_str("> ");
            }
            "code" | "kbd" => {
                self.output.push('`');
            }
            "strong" | "b" => {
                self.output.push_str("**");
            }
            "em" | "i" | "cite" => {
                self.output.push('*');
            }
            "s" | "del" | "strike" => {
                self.output.push_str("~~");
            }
            "u" => {
                self.output.push('_');
            }
            "mark" => {
                self.output.push_str("==");
            }
            "sup" => {
                self.output.push('^');
            }
            "sub" => {
                self.output.push('~');
            }
            "q" => {
                self.output.push('"');
            }
            "a" => {
                let href = extract_attr(raw_tag, "href").unwrap_or_default();
                self.in_link = Some((href, String::new()));
            }
            "img" => {
                let src = extract_attr(raw_tag, "src").unwrap_or_default();
                let alt = extract_attr(raw_tag, "alt").unwrap_or_else(|| "image".to_string());
                if !src.is_empty() {
                    self.output.push_str(&format!("![{}]({})", alt, src));
                }
            }
            "table" => {
                self.ensure_newline(2);
                self.table_stack.push(TableState {
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    in_cell: false,
                    cell_is_header: false,
                    current_cell_text: String::new(),
                });
            }
            "tr" => {
                if let Some(table) = self.table_stack.last_mut() {
                    table.current_row.clear();
                }
            }
            "th" => {
                if let Some(table) = self.table_stack.last_mut() {
                    table.in_cell = true;
                    table.cell_is_header = true;
                    table.current_cell_text.clear();
                }
            }
            "td" => {
                if let Some(table) = self.table_stack.last_mut() {
                    table.in_cell = true;
                    table.cell_is_header = false;
                    table.current_cell_text.clear();
                }
            }
            "div" | "section" | "article" | "main" | "header" => {
                self.ensure_newline(1);
            }
            _ => {}
        }

        if is_self_closing && tag != "br" && tag != "hr" && tag != "img" {
            self.handle_close_tag(tag);
        }
    }

    fn handle_close_tag(&mut self, tag: &str) {
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.heading_level = None;
                self.ensure_newline(2);
            }
            "p" => {
                self.ensure_newline(2);
            }
            "ul" | "ol" => {
                self.list_stack.pop();
                self.ensure_newline(2);
            }
            "li" => {
                self.ensure_newline(1);
            }
            "dt" => {
                if self.in_dt {
                    self.output.push_str("**\n");
                    self.in_dt = false;
                }
            }
            "dd" => {
                if self.in_dd {
                    self.ensure_newline(1);
                    self.in_dd = false;
                }
            }
            "dl" => {
                self.ensure_newline(2);
            }
            "blockquote" => {
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.ensure_newline(2);
            }
            "code" | "kbd" => {
                self.output.push('`');
            }
            "strong" | "b" => {
                self.output.push_str("**");
            }
            "em" | "i" | "cite" => {
                self.output.push('*');
            }
            "s" | "del" | "strike" => {
                self.output.push_str("~~");
            }
            "u" => {
                self.output.push('_');
            }
            "mark" => {
                self.output.push_str("==");
            }
            "sup" => {
                self.output.push('^');
            }
            "sub" => {
                self.output.push('~');
            }
            "q" => {
                self.output.push('"');
            }
            "a" => {
                if let Some((href, text)) = self.in_link.take() {
                    self.emit_link(&href, &text);
                }
            }
            "th" | "td" => {
                if let Some(table) = self.table_stack.last_mut() {
                    if table.in_cell {
                        let text = table.current_cell_text.split_whitespace().collect::<Vec<_>>().join(" ");
                        table.current_row.push(TableCell {
                            is_header: table.cell_is_header,
                            text,
                        });
                        table.in_cell = false;
                        table.current_cell_text.clear();
                    }
                }
            }
            "tr" => {
                if let Some(table) = self.table_stack.last_mut() {
                    if !table.current_row.is_empty() {
                        table.rows.push(std::mem::take(&mut table.current_row));
                    }
                }
            }
            "table" => {
                if let Some(table) = self.table_stack.pop() {
                    self.render_table(table);
                }
            }
            "div" | "section" | "article" | "main" => {
                self.ensure_newline(1);
            }
            _ => {}
        }
    }

    fn emit_link(&mut self, href: &str, text: &str) {
        let trimmed_href = href.trim();
        let trimmed_text = text.trim();

        if trimmed_href.is_empty() || trimmed_href.starts_with("javascript:") || trimmed_href == "#" {
            self.output.push_str(trimmed_text);
        } else if trimmed_text.is_empty() || trimmed_text == trimmed_href {
            self.output.push_str(trimmed_href);
        } else {
            self.output.push_str(&format!("[{}]({})", trimmed_text, trimmed_href));
        }
    }

    fn render_table(&mut self, table: TableState) {
        if table.rows.is_empty() {
            return;
        }

        self.ensure_newline(2);
        let max_cols = table.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if max_cols == 0 {
            return;
        }

        let mut has_rendered_header = false;

        for (row_idx, row) in table.rows.iter().enumerate() {
            self.output.push('|');
            for col_idx in 0..max_cols {
                let cell_text = row.get(col_idx).map(|c| c.text.as_str()).unwrap_or("");
                self.output.push(' ');
                self.output.push_str(cell_text);
                self.output.push_str(" |");
            }
            self.output.push('\n');

            let is_header_row = row.iter().any(|c| c.is_header);
            if (is_header_row || row_idx == 0) && !has_rendered_header {
                self.output.push('|');
                for _ in 0..max_cols {
                    self.output.push_str(" --- |");
                }
                self.output.push('\n');
                has_rendered_header = true;
            }
        }
        self.output.push('\n');
    }

    fn ensure_newline(&mut self, count: usize) {
        let trailing_newlines = self.output.chars().rev().take_while(|&c| c == '\n').count();
        if self.output.is_empty() {
            return;
        }
        for _ in trailing_newlines..count {
            self.output.push('\n');
        }
    }
}

/// Parses programming language from code class like `language-rust` or `lang-python`.
fn parse_code_language(class_attr: &str) -> Option<String> {
    for word in class_attr.split_whitespace() {
        if let Some(lang) = word.strip_prefix("language-").or_else(|| word.strip_prefix("lang-")).or_else(|| word.strip_prefix("highlight-")) {
            if !lang.is_empty() {
                return Some(lang.to_string());
            }
        }
    }
    None
}

/// Extracts attribute value from a raw tag string (e.g. `href="https://..."`).
fn extract_attr(raw_tag: &str, attr_name: &str) -> Option<String> {
    let lower_tag = raw_tag.to_ascii_lowercase();
    let search_attr = format!("{}=", attr_name.to_ascii_lowercase());

    if let Some(mut idx) = lower_tag.find(&search_attr) {
        // Verify attribute boundary (preceded by space)
        if idx > 0 && !raw_tag.as_bytes()[idx - 1].is_ascii_whitespace() {
            return None;
        }
        idx += search_attr.len();
        let rest = raw_tag[idx..].trim_start();
        if rest.starts_with('"') {
            if let Some(end) = rest[1..].find('"') {
                return Some(decode_html_entities(&rest[1..1 + end]));
            }
        } else if rest.starts_with('\'') {
            if let Some(end) = rest[1..].find('\'') {
                return Some(decode_html_entities(&rest[1..1 + end]));
            }
        } else {
            let val = rest.split_whitespace().next().unwrap_or("").trim_end_matches('>');
            if !val.is_empty() {
                return Some(decode_html_entities(val));
            }
        }
    }
    None
}

/// Collapses runs of multiple spaces/tabs into a single space, preserving newlines.
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;

    for ch in s.chars() {
        if ch == ' ' || ch == '\t' || ch == '\u{a0}' {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else if ch == '\n' || ch == '\r' {
            out.push('\n');
            last_was_space = false;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    out
}

/// Post-processes converted markdown text to normalize line endings and clean excess blank lines.
fn post_process(text: &str) -> String {
    let mut lines = Vec::new();
    let mut blank_count = 0;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim_end();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 && !lines.is_empty() {
                lines.push("");
            }
        } else {
            blank_count = 0;
            lines.push(trimmed);
        }
    }

    lines.join("\n")
}

/// Decodes common HTML named and numeric entities into UTF-8 characters.
pub fn decode_html_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            let mut entity_buf = String::with_capacity(12);
            let mut matched = false;

            while let Some(&next_ch) = chars.peek() {
                if next_ch == ';' {
                    chars.next();
                    matched = true;
                    break;
                } else if next_ch.is_alphanumeric() || next_ch == '#' {
                    entity_buf.push(next_ch);
                    chars.next();
                    if entity_buf.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if matched {
                if let Some(decoded_char) = decode_single_entity(&entity_buf) {
                    out.push(decoded_char);
                } else {
                    out.push('&');
                    out.push_str(&entity_buf);
                    out.push(';');
                }
            } else {
                out.push('&');
                out.push_str(&entity_buf);
            }
        } else {
            out.push(ch);
        }
    }

    out
}

fn decode_single_entity(entity: &str) -> Option<char> {
    if let Some(stripped_hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
        u32::from_str_radix(stripped_hex, 16).ok().and_then(char::from_u32)
    } else if let Some(stripped_dec) = entity.strip_prefix('#') {
        stripped_dec.parse::<u32>().ok().and_then(char::from_u32)
    } else {
        match entity {
            "quot" => Some('"'),
            "amp" => Some('&'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            "ensp" => Some(' '),
            "emsp" => Some(' '),
            "thinsp" => Some(' '),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            "lsquo" => Some('‘'),
            "rsquo" => Some('’'),
            "sbquo" => Some('‚'),
            "ldquo" => Some('“'),
            "rdquo" => Some('”'),
            "bdquo" => Some('„'),
            "bull" => Some('•'),
            "copy" => Some('©'),
            "reg" => Some('®'),
            "trade" => Some('™'),
            "pound" => Some('£'),
            "euro" => Some('€'),
            "yen" => Some('¥'),
            "cent" => Some('¢'),
            "sect" => Some('§'),
            "deg" => Some('°'),
            "plusmn" => Some('±'),
            "times" => Some('×'),
            "divide" => Some('÷'),
            "laquo" => Some('«'),
            "raquo" => Some('»'),
            "para" => Some('¶'),
            "middot" => Some('·'),
            "larr" => Some('←'),
            "rarr" => Some('→'),
            "uarr" => Some('↑'),
            "darr" => Some('↓'),
            "harr" => Some('↔'),
            "check" | "checkmark" => Some('✓'),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("Hello &amp; world &lt;3 &gt; &quot;yes&quot;"), "Hello & world <3 > \"yes\"");
        assert_eq!(decode_html_entities("Copyright &copy; 2026 &#8212; &#x2764;"), "Copyright © 2026 — ❤");
    }

    #[test]
    fn test_strip_noise_blocks() {
        let html = r#"
            <html>
            <head><title>Test Page</title><script>console.log("bad");</script><style>body { color: red; }</style></head>
            <body>
                <nav><a href="/">Home</a></nav>
                <p>Real content</p>
                <aside>Sidebar ad</aside>
                <footer>Footer links</footer>
            </body>
            </html>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("Real content"));
        assert!(!preprocessed.contains("console.log"));
        assert!(!preprocessed.contains("body { color: red; }"));
        assert!(!preprocessed.contains("Sidebar ad"));
        assert!(!preprocessed.contains("Footer links"));
        assert!(preprocessed.contains("# Test Page"));
    }

    #[test]
    fn test_preprocess_headings_and_paragraphs() {
        let html = r#"
            <article>
                <h1>Article Title</h1>
                <p>First paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
                <h2>Section 1</h2>
                <p>Second paragraph with <a href="https://example.com">a link</a>.</p>
            </article>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("# Article Title"));
        assert!(preprocessed.contains("First paragraph with **bold** and *italic* text."));
        assert!(preprocessed.contains("## Section 1"));
        assert!(preprocessed.contains("[a link](https://example.com)"));
    }

    #[test]
    fn test_preprocess_lists() {
        let html = r#"
            <div>
                <ul>
                    <li>First bullet</li>
                    <li>Second bullet</li>
                </ul>
                <ol>
                    <li>First item</li>
                    <li>Second item</li>
                </ol>
            </div>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("* First bullet"));
        assert!(preprocessed.contains("* Second bullet"));
        assert!(preprocessed.contains("1. First item"));
        assert!(preprocessed.contains("2. Second item"));
    }

    #[test]
    fn test_preprocess_code_blocks() {
        let html = r#"
            <pre><code class="language-rust">fn main() {
    println!("Hello, World!");
}</code></pre>
            <p>Use <code>cargo run</code> to execute.</p>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("```rust"));
        assert!(preprocessed.contains("println!(\"Hello, World!\");"));
        assert!(preprocessed.contains("`cargo run`"));
    }

    #[test]
    fn test_preprocess_table() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Item</th><th>Price</th></tr>
                </thead>
                <tbody>
                    <tr><td>Apple</td><td>$1.00</td></tr>
                    <tr><td>Banana</td><td>$0.50</td></tr>
                </tbody>
            </table>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("| Item | Price |"));
        assert!(preprocessed.contains("| --- | --- |"));
        assert!(preprocessed.contains("| Apple | $1.00 |"));
        assert!(preprocessed.contains("| Banana | $0.50 |"));
    }

    #[test]
    fn test_non_html_passthrough() {
        let text = "This is just plain text\nwith multiple lines.";
        assert_eq!(preprocess_html(text), text);
    }

    #[test]
    fn test_preprocess_nested_lists_and_formatting() {
        let html = r#"
            <main>
                <h2>Complex Note</h2>
                <ul>
                    <li>Item 1
                        <ul>
                            <li>Sub-item 1.1</li>
                            <li>Sub-item 1.2</li>
                        </ul>
                    </li>
                    <li>Item 2</li>
                </ul>
                <blockquote>
                    <p>First quote line.</p>
                    <p>Second quote line.</p>
                </blockquote>
                <p>Formatted text: <del>deleted</del>, <mark>highlighted</mark>, <sup>super</sup>, <sub>sub</sub>, <kbd>Ctrl+C</kbd>.</p>
            </main>
        "#;
        let preprocessed = preprocess_html(html);
        assert!(preprocessed.contains("## Complex Note"));
        assert!(preprocessed.contains("* Item 1"));
        assert!(preprocessed.contains("  * Sub-item 1.1"));
        assert!(preprocessed.contains("  * Sub-item 1.2"));
        assert!(preprocessed.contains("* Item 2"));
        assert!(preprocessed.contains("> First quote line."));
        assert!(preprocessed.contains("> Second quote line."));
        assert!(preprocessed.contains("~~deleted~~"));
        assert!(preprocessed.contains("==highlighted=="));
        assert!(preprocessed.contains("^super^"));
        assert!(preprocessed.contains("~sub~"));
        assert!(preprocessed.contains("`Ctrl+C`"));
    }

    #[test]
    fn test_large_html_without_premature_truncation() {
        let mut paragraphs = String::new();
        for i in 0..100 {
            paragraphs.push_str(&format!("<p>This is test paragraph number {} containing plenty of valuable content for the document.</p>\n", i));
        }
        let html = format!("<html><body><h1>Large Document</h1>{}</body></html>", paragraphs);
        let preprocessed = preprocess_html(&html);
        assert!(preprocessed.contains("# Large Document"));
        assert!(preprocessed.contains("paragraph number 0"));
        assert!(preprocessed.contains("paragraph number 99"));
        assert!(!preprocessed.contains("truncated"));
    }
}
