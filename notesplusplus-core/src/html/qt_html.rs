use std::fmt::Write;

use crate::block::Block;
use crate::inline::InlineSpan;
use crate::escape::escape_html_text as escape_html;

/// Theme colors injected from QML (Sailfish Theme.* values).
#[derive(Debug, Clone)]
pub struct QtThemeColors {
    pub highlight_color: String,
    pub primary_color: String,
    pub highlight_background_color: String,
}

impl Default for QtThemeColors {
    fn default() -> Self {
        Self {
            highlight_color: "#0088cc".into(),
            primary_color: "#ffffff".into(),
            highlight_background_color: "rgba(0,136,204,0.25)".into(),
        }
    }
}

impl QtThemeColors {
    pub fn from_map(map: &std::collections::HashMap<String, String>) -> Self {
        Self {
            highlight_color: map.get("highlightColor").cloned().unwrap_or_else(|| "#0088cc".into()),
            primary_color: map.get("primaryColor").cloned().unwrap_or_else(|| "#ffffff".into()),
            highlight_background_color: map.get("highlightBackgroundColor").cloned()
                .unwrap_or_else(|| "rgba(0,136,204,0.25)".into()),
        }
    }
}

/// Options controlling per-page rendering behavior.
#[derive(Debug, Clone, Default)]
pub struct QtRenderOptions {
    pub search_terms: Vec<String>,
    pub notes_dir: Option<String>,
    pub allow_external_images: bool,
}

struct QtHtmlCtx<'a> {
    theme: &'a QtThemeColors,
    options: &'a QtRenderOptions,
    current_path: Vec<usize>,
}

impl<'a> QtHtmlCtx<'a> {
    fn new(theme: &'a QtThemeColors, options: &'a QtRenderOptions) -> Self {
        Self { theme, options, current_path: Vec::new() }
    }

    fn toggle_path(&self) -> String {
        self.current_path.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// Public wrapper for escape_html, used by bridge for footnotes rendering.
pub fn escape_html_for_footnote(text: &str) -> String {
    escape_html(text)
}

// ── Image path resolution (matches JS resolveImagePath) ──────────────

pub fn resolve_qt_image_path(target: &str, notes_dir: Option<&str>, allow_external: bool) -> String {
    if target.is_empty() { return String::new(); }
    if target.starts_with("http://") || target.starts_with("https://") {
        return if allow_external { target.to_string() } else { String::new() };
    }
    if target.starts_with("file://") || target.starts_with('/') {
        return target.to_string();
    }
    if let Some(dir) = notes_dir {
        if !dir.is_empty() {
            return format!("file://{}/{}", dir, target);
        }
    }
    format!("file:///usr/share/harbour-notesplusplus/examples/{}", target)
}

// ── Icon rendering (matches JS renderIcon) ───────────────────────────

pub fn render_icon(icon_name: &str, highlight_color: &str) -> String {
    if icon_name.is_empty() { return String::new(); }
    let name = icon_name.to_lowercase();
    let color = if highlight_color.is_empty() { "#ffffff" } else { highlight_color };
    match name.as_str() {
        "heart" => format!("<span style='color:{};'>&#10084;</span>", color),
        "star" => format!("<span style='color:{};'>&#9733;</span>", color),
        "check" | "check-circle" => format!("<span style='color:{};font-weight:bold;'>&#10004;</span>", color),
        "info" | "info-circle" => format!("<span style='color:{};font-weight:bold;'>&#8505;</span>", color),
        "warning" | "exclamation" | "alert" => format!("<span style='color:{};font-weight:bold;'>&#9888;</span>", color),
        "folder" => format!("<span style='color:{};'>&#128193;</span>", color),
        "file" | "document" => format!("<span style='color:{};'>&#128196;</span>", color),
        "tag" | "tags" => format!("<span style='color:{};'>&#127991;</span>", color),
        "search" => format!("<span style='color:{};'>&#128269;</span>", color),
        "gear" | "cog" | "settings" => format!("<span style='color:{};'>&#9881;</span>", color),
        _ => format!("<span style='color:{};'>:{}</span>", color, escape_html(icon_name)),
    }
}

// ── Inline span rendering (matches JS spanToHtml) ────────────────────

fn render_span(span: &InlineSpan, ctx: &QtHtmlCtx) -> String {
    match span {
        InlineSpan::Text(s) => escape_html(s),
        InlineSpan::Bold(inner) => format!("<b>{}</b>", render_spans(inner, ctx)),
        InlineSpan::Italic(inner) => format!("<i>{}</i>", render_spans(inner, ctx)),
        InlineSpan::Code(s) => {
            format!("<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:0;font-family:monospace;word-break:break-all;'>{}</code>", escape_html(s))
        }
        InlineSpan::Monospace(inner) => {
            format!("<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:0;font-family:monospace;word-break:break-all;'>{}</code>", render_spans(inner, ctx))
        }
        InlineSpan::DoubleQuote(inner) => format!("&ldquo;{}&rdquo;", render_spans(inner, ctx)),
        InlineSpan::SingleQuote(inner) => format!("&lsquo;{}&rsquo;", render_spans(inner, ctx)),
        InlineSpan::Link { url, display } => {
            let disp = if display.is_empty() { url.as_str() } else { display.as_str() };
            // Use __LINK_COLOR__ placeholder — QML substitutes Theme.highlightColor
            format!("<a href='{}' style='color:__LINK_COLOR__'>{}</a>", escape_html(url), escape_html(disp))
        }
        InlineSpan::Xref { target, display } => {
            let disp = if display.is_empty() { target.as_str() } else { display.as_str() };
            format!("<a href='xref:{}' style='color:__LINK_COLOR__'>{}</a>", escape_html(target), escape_html(disp))
        }
        InlineSpan::Strikethrough(inner) => format!("<s>{}</s>", render_spans(inner, ctx)),
        InlineSpan::Superscript(inner) => format!("<sup>{}</sup>", render_spans(inner, ctx)),
        InlineSpan::Subscript(inner) => format!("<sub>{}</sub>", render_spans(inner, ctx)),
        InlineSpan::Image { target, alt, width } => {
            let src = resolve_qt_image_path(target, ctx.options.notes_dir.as_deref(), ctx.options.allow_external_images);
            let w_attr = match width {
                Some(w) => format!(" width='{}'", escape_html(w)),
                None => " style='max-width:100%;'".to_string(),
            };
            format!("<img src='{}'{} alt='{}' />", escape_html(&src), w_attr, escape_html(alt))
        }
        InlineSpan::Icon { name, .. } => render_icon(name, &ctx.theme.highlight_color),
        InlineSpan::Footnote { id, text } => {
            let content = if text.is_empty() { id.as_deref().unwrap_or("") } else { text.as_str() };
            format!("<sup style='color:{}'>[{}]</sup>", ctx.theme.highlight_color, escape_html(content))
        }
        InlineSpan::Callout(num) => {
            format!("<b style='color:{};background:{};padding:1px 4px;border-radius:8px;'>&lt;{}&gt;</b>",
                ctx.theme.highlight_color, ctx.theme.highlight_background_color, num)
        }
        InlineSpan::Mark(inner) => {
            format!("<mark style='background:{};color:{};padding:1px 3px;border-radius:2px;'>{}</mark>",
                ctx.theme.highlight_background_color, ctx.theme.primary_color, render_spans(inner, ctx))
        }
        InlineSpan::Kbd(keys) => {
            let joined = keys.join("+");
            format!("<kbd style='background:#2a2a2e;color:#f2f2f7;padding:1px 5px;border:1px solid #444;border-radius:4px;font-family:monospace;'>{}</kbd>", escape_html(&joined))
        }
        InlineSpan::Button(text) => {
            format!("<span style='background:{};color:{};padding:1px 6px;border:1px solid {};border-radius:4px;font-weight:bold;'>[{}]</span>",
                ctx.theme.highlight_background_color, ctx.theme.highlight_color, ctx.theme.highlight_color, escape_html(text))
        }
        InlineSpan::Menu(items) => {
            let joined = items.iter().map(|i| escape_html(i)).collect::<Vec<_>>().join(" \u{25B8} ");
            format!("<b style='color:{};'>{}</b>", ctx.theme.highlight_color, joined)
        }
        InlineSpan::Pass(val) => val.clone(),
    }
}

/// Render inline spans to Qt RichText HTML. Matches JS `spansToHtml()`.
fn render_spans(spans: &[InlineSpan], ctx: &QtHtmlCtx) -> String {
    if spans.is_empty() { return String::new(); }
    // Fast path: single text span (matches JS optimization)
    if spans.len() == 1 {
        if let InlineSpan::Text(s) = &spans[0] {
            return escape_html(s);
        }
    }
    let mut out = String::new();
    for span in spans {
        out.push_str(&render_span(span, ctx));
    }
    out
}

// ── Block rendering ──────────────────────────

/// Render a single block to Qt RichText HTML. Returns the HTML string.
pub fn render_qt_block(block: &Block, block_index: usize, theme: &QtThemeColors, options: &QtRenderOptions) -> String {
    let mut ctx = QtHtmlCtx::new(theme, options);
    ctx.current_path.push(block_index);
    render_block_inner(block, &mut ctx)
}

/// Render a list of blocks (for children of compound blocks).
pub fn render_qt_blocks(blocks: &[Block], path_prefix: &str, theme: &QtThemeColors, options: &QtRenderOptions) -> String {
    let mut ctx = QtHtmlCtx::new(theme, options);
    let items: Vec<(&Block, String)> = blocks.iter()
        .enumerate()
        .map(|(i, b)| {
            let path = if path_prefix.is_empty() { i.to_string() } else { format!("{}.{}", path_prefix, i) };
            (b, path)
        })
        .collect();
    render_blocks_from_items(&items, 0, &mut ctx)
}

/// Render inline spans to Qt RichText HTML (public wrapper).
pub fn render_qt_spans(spans: &[InlineSpan], theme: &QtThemeColors, options: &QtRenderOptions) -> String {
    let ctx = QtHtmlCtx::new(theme, options);
    render_spans(spans, &ctx)
}

fn render_block_inner(block: &Block, ctx: &mut QtHtmlCtx) -> String {
    let highlight = &ctx.theme.highlight_color;
    let primary = &ctx.theme.primary_color;
    let highlight_bg = &ctx.theme.highlight_background_color;

    match block {
        Block::Paragraph { spans, .. } => {
            format!("<p style='margin:4px 0;'>{}</p>", render_spans(spans, ctx))
        }
        Block::Heading { spans, .. } => {
            // Use __LINK_COLOR__ placeholder — QML substitutes Theme.highlightColor
            format!("<h3 style='color:__LINK_COLOR__;margin:6px 0 2px 0;'>{}</h3>", render_spans(spans, ctx))
        }
        Block::CodeBlock { lines, language, .. } if language.as_deref() == Some("svgbob") => {
            let source = lines.join("\n");
            let svg = crate::diagram::render_svgbob(&source);
            let b64 = crate::html::base64_encode(svg.as_bytes());
            format!("<p><img src='data:image/svg+xml;base64,{}' /></p>", b64)
        }
        Block::CodeBlock { lines, .. } | Block::LiteralBlock { lines, .. } => {
            let code_lines = lines.iter().map(|l| escape_html(l)).collect::<Vec<_>>().join("<br/>");
            format!("<pre style='background:#18181c;color:#f2f2f7;padding:6px;border-radius:0;font-family:monospace;margin:4px 0;word-break:break-all;'>{}</pre>", code_lines)
        }
        Block::DescriptionListItem { term, term_spans, children, .. } => {
            let term_txt = if !term_spans.is_empty() {
                render_spans(term_spans, ctx)
            } else {
                escape_html(term)
            };
            let desc_html = if !children.is_empty() {
                if let Some(first) = children.first() {
                    let first_html = match first {
                        Block::Paragraph { spans, .. } => render_spans(spans, ctx),
                        _ => render_block_inner(first, ctx),
                    };
                    if children.len() > 1 {
                        let rest: Vec<&Block> = children[1..].iter().collect();
                        let rest_html = render_blocks_slice(&rest, ctx);
                        format!("{}<br/>{}", first_html, rest_html)
                    } else {
                        first_html
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            format!("<p style='margin:4px 0;'><b>{}:</b> {}</p>", term_txt, desc_html)
        }
        Block::Blockquote { children, .. } => {
            let quote_body = render_blocks_slice(&children.iter().collect::<Vec<_>>(), ctx);
            format!("<blockquote style='margin:4px 0;padding-left:8px;border-left:2px solid {};color:{};'>{}</blockquote>", highlight, primary, quote_body)
        }
        Block::Verse { lines, spans, .. } => {
            let quote_body = if !spans.is_empty() {
                spans.iter().enumerate().map(|(i, line_spans)| {
                    let line_html = render_spans(line_spans, ctx);
                    if i < spans.len() - 1 { format!("{}<br/>", line_html) } else { line_html }
                }).collect::<String>()
            } else {
                lines.iter().map(|l| escape_html(l)).collect::<Vec<_>>().join("<br/>")
            };
            format!("<blockquote style='margin:4px 0;padding-left:8px;border-left:2px solid {};color:{};'>{}</blockquote>", highlight, primary, quote_body)
        }
        Block::Admonition { kind, children, .. } => {
            let inner = render_blocks_slice(&children.iter().collect::<Vec<_>>(), ctx);
            let (border_color, bg_color) = match kind {
                crate::block::AdmonitionKind::Warning => ("#d9534f", "rgba(217,83,79,0.12)"),
                crate::block::AdmonitionKind::Tip => ("#f0ad4e", "rgba(240,173,78,0.12)"),
                _ => ("__LINK_COLOR__", highlight_bg.as_str()),
            };
            format!("<div style='margin:4px 0;padding:6px;border-left:3px solid {};background:{};'><b style='color:{};'>{}:</b> {}</div>",
                border_color, bg_color, border_color, escape_html(kind.as_str()), inner)
        }
        Block::Sidebar { title, children, .. } | Block::Example { title, children, .. } | Block::Open { title, children, .. } => {
            let title_html = match title {
                Some(t) if !t.is_empty() => format!("<b style='color:{};'>{}</b><br/>", highlight, escape_html(t)),
                _ => String::new(),
            };
            let inner = render_blocks_slice(&children.iter().collect::<Vec<_>>(), ctx);
            format!("<div style='margin:4px 0;padding:6px;border:1px solid {};border-radius:0;'>{}{}</div>", highlight, title_html, inner)
        }
        Block::Image { target, alt, width, .. } => {
            let src = resolve_qt_image_path(target, ctx.options.notes_dir.as_deref(), ctx.options.allow_external_images);
            let w_attr = match width {
                Some(w) => format!(" width='{}'", escape_html(w)),
                None => " style='max-width:100%;'".to_string(),
            };
            format!("<img src='{}'{} alt='{}' />", escape_html(&src), w_attr, escape_html(alt))
        }
        Block::UnorderedListItem { .. } | Block::OrderedListItem { .. } | Block::CalloutListItem { .. } => {
            // Single list item — render via the list path
            let items: Vec<(&Block, String)> = vec![(block, ctx.toggle_path())];
            render_blocks_from_items(&items, 0, ctx)
        }
        Block::Table { rows, col_widths, .. } => {
            render_table(rows, col_widths, ctx)
        }
        Block::EmptyLine => "<div style='height:8px;'></div>".to_string(),
        Block::HorizontalRule { .. } => "<hr/>".to_string(),
        Block::PageBreak { .. } => "<div style='height:8px;'></div>".to_string(),
        Block::Comment { text, .. } => {
            format!("<p style='margin:4px 0;font-style:italic;font-family:monospace;color:rgba(255,255,255,0.6);'>// {}</p>", escape_html(text))
        }
        Block::Toc { .. } => String::new(), // Toc rendered natively in QML
    }
}

fn render_table(rows: &[Vec<crate::block::TableCell>], col_widths: &[f64], ctx: &mut QtHtmlCtx) -> String {
    let mut html = String::new();
    // Use HTML border attribute (Qt RichText supports this, not CSS border)
    html.push_str("<table border='1' cellspacing='0' cellpadding='4' width='100%' style='margin:4px 0;'>");
    for (row_idx, row) in rows.iter().enumerate() {
        html.push_str("<tr>");
        for (cell_idx, cell) in row.iter().enumerate() {
            let tag = if row_idx == 0 { "th" } else { "td" };
            let cell_html = render_blocks_slice(&cell.blocks.iter().collect::<Vec<_>>(), ctx);
            let colspan_attr = if cell.colspan > 1 { format!(" colspan='{}'", cell.colspan) } else { String::new() };
            let align_attr = match cell.align.as_deref() {
                Some("center") => " align='center'",
                Some("right") => " align='right'",
                _ => "",
            };
            let bg_attr = if row_idx == 0 {
                " bgcolor='rgba(0,136,204,0.1)'"
            } else if row_idx % 2 == 1 {
                " bgcolor='rgba(255,255,255,0.03)'"
            } else {
                ""
            };
            let bold_open = if row_idx == 0 || (row_idx == 1 && rows.first().map_or(false, |r| r.is_empty())) {
                "<b>"
            } else {
                ""
            };
            let bold_close = if row_idx == 0 || (row_idx == 1 && rows.first().map_or(false, |r| r.is_empty())) {
                "</b>"
            } else {
                ""
            };
            // Column width
            let width_attr = if !col_widths.is_empty() && cell_idx < col_widths.len() {
                let total: f64 = col_widths.iter().sum();
                if total > 0.0 {
                    let pct = (col_widths[cell_idx] / total * 100.0) as u32;
                    format!(" width='{}%'", pct)
                } else { String::new() }
            } else { String::new() };
            write!(html, "<{}{}{}{}{}>{}{}{}</{}>",
                tag, colspan_attr, width_attr, align_attr, bg_attr, bold_open, cell_html, bold_close, tag).unwrap();
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    html
}

fn render_blocks_slice(blocks: &[&Block], ctx: &mut QtHtmlCtx) -> String {
    let items: Vec<(&Block, String)> = blocks.iter()
        .enumerate()
        .map(|(i, b)| (*b, i.to_string()))
        .collect();
    render_blocks_from_items(&items, 0, ctx)
}

fn render_blocks_from_items(items: &[(&Block, String)], depth: usize, ctx: &mut QtHtmlCtx) -> String {
    if items.is_empty() { return String::new(); }

    let mut html = String::new();
    let mut list_buf: Vec<(&Block, &str)> = Vec::new();
    let mut list_type = "";

    for (b, path) in items {
        let t = b.block_type();
        if t == "unordered_list_item" || t == "ordered_list_item" || t == "callout_list_item" {
            let cur_type = if t == "unordered_list_item" { "unordered" }
                else if t == "ordered_list_item" { "ordered" }
                else { "callout" };
            if !list_buf.is_empty() && cur_type != list_type {
                flush_list(&mut list_buf, &mut list_type, &mut html, depth, ctx);
            }
            list_type = cur_type;
            list_buf.push((b, path.as_str()));
        } else {
            flush_list(&mut list_buf, &mut list_type, &mut html, depth, ctx);
            html.push_str(&render_block_inner(b, ctx));
        }
    }
    flush_list(&mut list_buf, &mut list_type, &mut html, depth, ctx);
    html
}

fn flush_list(buf: &mut Vec<(&Block, &str)>, lt: &mut &str, html: &mut String, depth: usize, ctx: &mut QtHtmlCtx) {
    if buf.is_empty() { return; }
    let primary = ctx.theme.primary_color.clone();
    let highlight = ctx.theme.highlight_color.clone();

    for (i, (b, path)) in buf.iter().enumerate() {
        let item_text = get_item_text(b, ctx);
        let is_checked = match b {
            Block::UnorderedListItem { checked, .. } => *checked,
            _ => None,
        };
        let check_mark = match is_checked {
            Some(true) => Some('\u{2611}'),
            Some(false) => Some('\u{2610}'),
            None => None,
        };

        let (marker_html, marker_color_owned, marker_width, is_bold) = if let Some(cm) = check_mark {
            let marker = format!("<a href='toggle:{}' style='color:{};text-decoration:none'>{}</a>", path, primary, cm);
            (marker, primary.clone(), "24px", false)
        } else if *lt == "callout" {
            let num = match b {
                Block::CalloutListItem { number, .. } => *number,
                _ => i + 1,
            };
            let marker = format!("&lt;{}&gt;", num);
            (marker, highlight.clone(), "28px", true)
        } else if *lt == "ordered" {
            let m = match b {
                Block::OrderedListItem { marker, .. } if !marker.is_empty() => marker.clone(),
                _ => format!("{}.", i + 1),
            };
            let marker = escape_html(&m);
            (marker, primary.clone(), "28px", true)
        } else {
            let bullet_chars = ["&bull;", "&#9702;", "&#9642;", "&#8212;", "&bull;"];
            let lvl = match b {
                Block::UnorderedListItem { level, .. } => *level as usize,
                _ => depth,
            };
            let marker = bullet_chars[lvl % bullet_chars.len()].to_string();
            (marker, primary.clone(), "18px", true)
        };

        // Wrap checkbox item text in toggle link
        let final_text = if check_mark.is_some() && !item_text.contains("<a ") {
            format!("<a href='toggle:{}' style='color:{};text-decoration:none'>{}</a>", path, primary, item_text)
        } else {
            item_text
        };

        // Child blocks
        let child_html = match b {
            Block::UnorderedListItem { children, .. }
            | Block::OrderedListItem { children, .. }
            | Block::CalloutListItem { children, .. } => {
                if children.len() > 1 {
                    let child_items: Vec<(&Block, String)> = children[1..].iter()
                        .enumerate()
                        .map(|(k, child)| {
                            let child_path = if path.is_empty() { format!("{}", k) } else { format!("{}.{}", path, k) };
                            (child, child_path)
                        })
                        .collect();
                    render_blocks_from_items(&child_items, depth + 1, ctx)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };

        let indent_pad = {
            let lvl = match b {
                Block::UnorderedListItem { level, .. } => *level as usize,
                _ => 0,
            };
            if lvl > 0 { lvl * 20 } else if depth > 0 { depth * 20 } else { 0 }
        };

        let marker_color = marker_color_owned;
        html.push_str("<table width='100%' style='width:100%;border-collapse:collapse;margin:2px 0;'><tr>");
        if indent_pad > 0 {
            write!(html, "<td width='{}px' style='width:{}px;padding:0;'></td>", indent_pad, indent_pad).unwrap();
        }
        let bold_str = if is_bold { "bold" } else { "normal" };
        write!(html, "<td width='{}' style='width:{};vertical-align:top;padding-right:6px;font-weight:{};color:{};'>{}</td>",
            marker_width, marker_width, bold_str, marker_color, marker_html).unwrap();
        write!(html, "<td width='99%' style='vertical-align:top;word-wrap:break-word;'>{}{}</td>", final_text, child_html).unwrap();
        html.push_str("</tr></table>");
    }
    buf.clear();
    *lt = "";
}

fn get_item_text(b: &Block, ctx: &QtHtmlCtx) -> String {
    match b {
        Block::UnorderedListItem { children, raw, .. }
        | Block::OrderedListItem { children, raw, .. } => {
            if let Some(first) = children.first() {
                match first {
                    Block::Paragraph { spans, .. } => return render_spans(spans, ctx),
                    _ => {}
                }
            }
            escape_html(raw)
        }
        Block::CalloutListItem { children, raw, .. } => {
            if let Some(first) = children.first() {
                match first {
                    Block::Paragraph { spans, .. } => return render_spans(spans, ctx),
                    _ => {}
                }
            }
            escape_html(raw)
        }
        _ => String::new(),
    }
}

// ── Search highlighting (matches JS highlightSearchTerms) ────────────

pub fn highlight_search_terms(html: &str, term: &str) -> String {
    if term.is_empty() || html.is_empty() { return html.to_string(); }
    let terms: Vec<String> = term.to_lowercase().split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    if terms.is_empty() { return html.to_string(); }

    let mut result = html.to_string();
    for t in &terms {
        if t.is_empty() { continue; }
        // Split on HTML tags to avoid highlighting inside tag attributes
        // Matches JS: result.split(/(<[^>]+>)/)
        let parts = split_tags(&result);
        let mut rebuilt = String::with_capacity(result.len() + 64);
        for part in &parts {
            if part.starts_with('<') {
                rebuilt.push_str(part);
            } else {
                // Case-insensitive replacement in text portions
                let lower_part = part.to_lowercase();
                let mut last_end = 0;
                let t_len = t.len();
                let mut search_start = 0;
                while let Some(pos) = lower_part[search_start..].find(t.as_str()) {
                    let abs_pos = search_start + pos;
                    rebuilt.push_str(&part[last_end..abs_pos]);
                    rebuilt.push_str("<mark style=\"background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;\">");
                    rebuilt.push_str(&part[abs_pos..abs_pos + t_len]);
                    rebuilt.push_str("</mark>");
                    last_end = abs_pos + t_len;
                    search_start = last_end;
                }
                rebuilt.push_str(&part[last_end..]);
            }
        }
        result = rebuilt;
    }
    result
}

/// Split HTML string into alternating text and tag parts.
/// Matches JS: str.split(/(<[^>]+>)/)
fn split_tags(html: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut last = 0;
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'<' {
            // Find the closing >
            if let Some(close) = html[i..].find('>') {
                // Add text before this tag
                if i > last {
                    parts.push(&html[last..i]);
                }
                // Add the tag itself
                let tag_end = i + close + 1;
                parts.push(&html[i..tag_end]);
                last = tag_end;
                i = tag_end;
                continue;
            }
        }
        i += 1;
    }
    // Add remaining text
    if last < len {
        parts.push(&html[last..]);
    }
    parts
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::parse_inline;
    use crate::parser;

    fn default_theme() -> QtThemeColors { QtThemeColors::default() }
    fn default_opts() -> QtRenderOptions { QtRenderOptions::default() }

    #[test]
    fn escape_html_passthrough() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn escape_html_special_chars() {
        assert_eq!(escape_html("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn escape_html_no_quotes() {
        // JS version does NOT escape quotes — verify we match
        assert_eq!(escape_html("it's \"quoted\""), "it's \"quoted\"");
    }

    #[test]
    fn paragraph_simple() {
        let blocks = parser::parse_blocks("Hello world");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert_eq!(html, "<p style='margin:4px 0;'>Hello world</p>");
    }

    #[test]
    fn paragraph_with_bold() {
        let blocks = parser::parse_blocks("Hello *world*");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert_eq!(html, "<p style='margin:4px 0;'>Hello <b>world</b></p>");
    }

    #[test]
    fn paragraph_with_italic() {
        let blocks = parser::parse_blocks("Hello _world_");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert_eq!(html, "<p style='margin:4px 0;'>Hello <i>world</i></p>");
    }

    #[test]
    fn paragraph_with_inline_code() {
        let blocks = parser::parse_blocks("use `foo` here");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:0;font-family:monospace;word-break:break-all;'>foo</code>"));
    }

    #[test]
    fn heading_level1() {
        let blocks = parser::parse_blocks("= Title");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert_eq!(html, "<h3 style='color:__LINK_COLOR__;margin:6px 0 2px 0;'>Title</h3>");
    }

    #[test]
    fn heading_level3() {
        let blocks = parser::parse_blocks("=== Sub");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        // All headings render as <h3> in the JS path
        assert_eq!(html, "<h3 style='color:__LINK_COLOR__;margin:6px 0 2px 0;'>Sub</h3>");
    }

    #[test]
    fn code_block() {
        let blocks = parser::parse_blocks("----\nfn main() {\n    println!(\"hi\");\n}\n----");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.starts_with("<pre style='background:#18181c;color:#f2f2f7;padding:6px;border-radius:0;font-family:monospace;margin:4px 0;word-break:break-all;'>"));
        assert!(html.contains("fn main()"));
        assert!(html.contains("<br/>"));
    }

    #[test]
    fn empty_line() {
        let blocks = parser::parse_blocks("Hello\n\nWorld");
        // Second block should be EmptyLine
        let html = render_qt_block(&blocks[1], 1, &default_theme(), &default_opts());
        assert_eq!(html, "<div style='height:8px;'></div>");
    }

    #[test]
    fn unordered_list_bullet_chars() {
        let blocks = parser::parse_blocks("* Item A\n* Item B");
        let html0 = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        let html1 = render_qt_block(&blocks[1], 1, &default_theme(), &default_opts());
        // Level 0 bullet = &bull;
        assert!(html0.contains("&bull;"));
        assert!(html1.contains("&bull;"));
        assert!(html0.contains("Item A"));
        assert!(html1.contains("Item B"));
    }

    #[test]
    fn checkbox_checked() {
        let blocks = parser::parse_blocks("* [x] Done");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("\u{2611}")); // ballot box with check
        assert!(html.contains("href='toggle:0'"));
    }

    #[test]
    fn checkbox_unchecked() {
        let blocks = parser::parse_blocks("* [ ] Todo");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("\u{2610}")); // empty ballot box
        assert!(html.contains("href='toggle:0'"));
    }

    #[test]
    fn nested_list_toggle_path() {
        let blocks = parser::parse_blocks("* [ ] Parent\n  * [x] Child");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("toggle:0")); // parent path
        assert!(html.contains("toggle:0.0")); // nested child path
    }

    #[test]
    fn ordered_list_marker() {
        let blocks = parser::parse_blocks(". First\n. Second");
        let html0 = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html0.contains("1."));
    }

    #[test]
    fn description_list() {
        let blocks = parser::parse_blocks("Term:: Definition text");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("<b>"));
        assert!(html.contains("Term"));
        assert!(html.contains("Definition text"));
    }

    #[test]
    fn admonition() {
        let blocks = parser::parse_blocks("[NOTE]\n====\nBe careful.\n====");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("NOTE:"));
        assert!(html.contains("Be careful."));
        assert!(html.contains("border-left:3px solid"));
    }

    #[test]
    fn sidebar_with_title() {
        let blocks = parser::parse_blocks(".My Sidebar\n****\nSome content.\n****");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("My Sidebar"));
        assert!(html.contains("Some content."));
        assert!(html.contains("border:1px solid"));
    }

    #[test]
    fn blockquote() {
        let blocks = parser::parse_blocks("[quote]\n____\nFamous words.\n____");
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("Famous words."));
        assert!(html.contains("<blockquote"));
        assert!(html.contains("border-left:2px solid"));
    }

    #[test]
    fn link_span() {
        let spans = parse_inline("visit https://example.com[Example] here");
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("<a href='https://example.com'"));
        assert!(html.contains("style='color:__LINK_COLOR__'"));
        assert!(html.contains("Example"));
    }

    #[test]
    fn xref_span() {
        let spans = parse_inline("see <<other-page>>");
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("href='xref:"));
    }

    #[test]
    fn icon_heart() {
        let html = render_icon("heart", "#ff0000");
        assert_eq!(html, "<span style='color:#ff0000;'>&#10084;</span>");
    }

    #[test]
    fn icon_unknown() {
        let html = render_icon("unknown", "#fff");
        assert_eq!(html, "<span style='color:#fff;'>:unknown</span>");
    }

    #[test]
    fn image_resolve_relative() {
        let path = resolve_qt_image_path("photo.png", Some("/home/user/notes"), true);
        assert_eq!(path, "file:///home/user/notes/photo.png");
    }

    #[test]
    fn image_resolve_absolute() {
        let path = resolve_qt_image_path("/tmp/photo.png", None, true);
        assert_eq!(path, "/tmp/photo.png");
    }

    #[test]
    fn image_resolve_http() {
        let path = resolve_qt_image_path("https://example.com/img.png", None, true);
        assert_eq!(path, "https://example.com/img.png");
    }

    #[test]
    fn image_resolve_http_blocked() {
        let path = resolve_qt_image_path("https://example.com/img.png", None, false);
        assert_eq!(path, "");
    }

    #[test]
    fn search_highlight_basic() {
        let html = "<p>Hello world</p>";
        let result = highlight_search_terms(html, "world");
        assert!(result.contains("<mark style=\"background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;\">world</mark>"));
    }

    #[test]
    fn search_highlight_skips_tags() {
        let html = "<p style='margin:4px 0;'>Hello</p>";
        let result = highlight_search_terms(html, "style");
        assert!(!result.contains("<mark"));
    }

    #[test]
    fn search_highlight_empty_term() {
        let html = "<p>Hello</p>";
        let result = highlight_search_terms(html, "");
        assert_eq!(result, html);
    }

    #[test]
    fn search_highlight_case_insensitive() {
        let html = "<p>Hello WORLD</p>";
        let result = highlight_search_terms(html, "world");
        assert!(result.contains("<mark"));
    }

    #[test]
    fn span_strikethrough() {
        let spans = parse_inline("this is ~~deleted~~ text");
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("<s>deleted</s>"));
    }

    #[test]
    fn span_superscript() {
        let spans = parse_inline("x^2^");
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("<sup>2</sup>"));
    }

    #[test]
    fn span_double_quote() {
        let spans = vec![InlineSpan::DoubleQuote(vec![InlineSpan::Text("quoted".into())])];
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("&ldquo;"));
        assert!(html.contains("&rdquo;"));
        assert!(html.contains("quoted"));
    }

    #[test]
    fn span_bold_nested_italic() {
        let spans = parse_inline("*bold _and italic_*");
        let html = render_qt_spans(&spans, &default_theme(), &default_opts());
        assert!(html.contains("<b>"));
        assert!(html.contains("<i>"));
    }

    #[test]
    fn custom_theme_colors() {
        let theme = QtThemeColors {
            highlight_color: "#ff0000".into(),
            primary_color: "#000000".into(),
            highlight_background_color: "rgba(255,0,0,0.25)".into(),
        };
        let blocks = parser::parse_blocks("= Title");
        let html = render_qt_block(&blocks[0], 0, &theme, &default_opts());
        assert!(html.contains("color:__LINK_COLOR__"));
    }

    #[test]
    fn table_basic() {
        let adoc = "|===\n| A | B\n| C | D\n|===";
        let blocks = parser::parse_blocks(adoc);
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert!(html.contains("<table"));
        assert!(html.contains("<td") || html.contains("<th"));
        assert!(html.contains("A"));
        assert!(html.contains("B"));
    }

    #[test]
    fn render_blocks_slice_empty() {
        let html = render_blocks_slice(&[], &mut QtHtmlCtx::new(&default_theme(), &default_opts()));
        assert_eq!(html, "");
    }

    #[test]
    fn full_example_chronicles() {
        let adoc = include_str!("../../examples/chronicles.adoc");
        let blocks = parser::parse_blocks(adoc);
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        // Should produce valid-looking HTML without panics
        assert!(html.contains("<h3"));
    }
}
