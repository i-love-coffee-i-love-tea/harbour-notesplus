use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::inline::InlineSpan;
use crate::parser;

/// Convert raw AsciiDoc content into a standalone HTML5 document.
pub fn adoc_to_html5(adoc_content: &str, title: &str, notes_dir: Option<&Path>) -> String {
    let blocks = parser::parse_blocks(adoc_content);
    blocks_to_html5(&blocks, title, notes_dir)
}

/// Convert an AST of blocks into a standalone HTML5 document.
pub fn blocks_to_html5(blocks: &[Block], title: &str, notes_dir: Option<&Path>) -> String {
    let mut ctx = HtmlRenderContext::new(notes_dir);
    
    // First pass: extract document title if not provided and collect headings for TOC
    let doc_title = if title.is_empty() {
        extract_title(blocks).unwrap_or_else(|| "Notes++ Document".to_string())
    } else {
        title.to_string()
    };

    let body_html = ctx.render_blocks(blocks);
    let footnotes_html = ctx.render_footnotes();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Notes++ (FishDoc) HTML5 Exporter">
    <title>{title}</title>
    <style>
{css}
    </style>
</head>
<body class="fishdoc-body">
    <div class="fishdoc-container">
        <header class="document-header">
            <h1 class="document-title">{title}</h1>
        </header>
        <main class="document-content">
{body}
{footnotes}
        </main>
        <footer class="document-footer">
            <p>Exported by <strong>Notes++</strong> on {date}</p>
        </footer>
    </div>
</body>
</html>"#,
        title = escape_html(&doc_title),
        css = DOCUMENT_CSS,
        body = body_html,
        footnotes = footnotes_html,
        date = chrono::Local::now().format("%Y-%m-%d %H:%M"),
    )
}

/// Export a specific note page from `notes_dir` to an output HTML file.
pub fn export_page_to_html5(
    notes_dir: &Path,
    filename: &str,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let adoc_path = notes_dir.join(filename);
    let content = std::fs::read_to_string(&adoc_path)
        .map_err(|e| format!("Failed to read {}: {}", adoc_path.display(), e))?;

    let title = filename.strip_suffix(".adoc").unwrap_or(filename);
    let html = adoc_to_html5(&content, title, Some(notes_dir));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output dir {}: {}", parent.display(), e))?;
    }

    std::fs::write(output_path, html.as_bytes())
        .map_err(|e| format!("Failed to write HTML to {}: {}", output_path.display(), e))?;

    Ok(output_path.to_path_buf())
}

struct HtmlRenderContext<'a> {
    notes_dir: Option<&'a Path>,
    footnotes: Vec<String>,
    heading_counts: HashMap<String, usize>,
}

impl<'a> HtmlRenderContext<'a> {
    fn new(notes_dir: Option<&'a Path>) -> Self {
        Self {
            notes_dir,
            footnotes: Vec::new(),
            heading_counts: HashMap::new(),
        }
    }

    fn generate_heading_id(&mut self, text: &str) -> String {
        let slug: String = text
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        let slug = slug.trim_matches('-').to_string();
        let base_slug = if slug.is_empty() { "section".to_string() } else { slug };

        let count = self.heading_counts.entry(base_slug.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            base_slug
        } else {
            format!("{}-{}", base_slug, *count)
        }
    }

    fn render_blocks(&mut self, blocks: &[Block]) -> String {
        let mut out = String::new();
        for block in blocks {
            out.push_str(&self.render_block(block));
            out.push('\n');
        }
        out
    }

    fn render_block(&mut self, block: &Block) -> String {
        match block {
            Block::Heading { level, spans, .. } => {
                let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                let heading_id = self.generate_heading_id(&text);
                let tag_level = (*level).clamp(1, 6);
                let content = self.render_spans(spans);
                format!(
                    "<h{lvl} id=\"{id}\" class=\"sect-heading sect{lvl}\"><a class=\"anchor\" href=\"#{id}\">#</a>{content}</h{lvl}>",
                    lvl = tag_level,
                    id = heading_id,
                    content = content
                )
            }
            Block::Paragraph { spans, .. } => {
                let content = self.render_spans(spans);
                format!("<p>{}</p>", content)
            }
            Block::OrderedListItem { marker, reversed, children, .. } => {
                let mut out = format!(r#"<li class="ordered-list-item" data-marker="{marker}">"#);
                out.push_str(&self.render_blocks(children));
                out.push_str("</li>");
                out
            }
            Block::UnorderedListItem { checked, children, .. } => {
                let (class_attr, checkbox) = match checked {
                    Some(true) => (
                        r#" class="checklist-item checked""#,
                        r#"<input type="checkbox" checked disabled class="checklist-checkbox"> "#,
                    ),
                    Some(false) => (
                        r#" class="checklist-item unchecked""#,
                        r#"<input type="checkbox" disabled class="checklist-checkbox"> "#,
                    ),
                    None => ("", ""),
                };
                let mut out = format!("<li{class_attr}>{checkbox}", class_attr = class_attr, checkbox = checkbox);
                out.push_str(&self.render_blocks(children));
                out.push_str("</li>");
                out
            }
            Block::DescriptionListItem { term_spans, children, .. } => {
                let term = self.render_spans(term_spans);
                let body = self.render_blocks(children);
                format!("<dt class=\"hdlist1\">{}</dt>\n<dd>{}</dd>", term, body)
            }
            Block::CalloutListItem { number, children, .. } => {
                let body = self.render_blocks(children);
                format!(
                    r#"<li class="callout-item"><b class="conum">({})</b> {}</li>"#,
                    number, body
                )
            }
            Block::CodeBlock { title, language, lines, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let lang_class = language
                    .as_ref()
                    .map(|l| format!(" language-{}", escape_html(l)))
                    .unwrap_or_default();
                let code_content = lines
                    .iter()
                    .map(|l| escape_html(l))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    r#"<div class="listingblock">{title}<pre class="highlight"><code class="code-block{lang}">{code}</code></pre></div>"#,
                    title = title_html,
                    lang = lang_class,
                    code = code_content
                )
            }
            Block::LiteralBlock { title, lines, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let code_content = lines
                    .iter()
                    .map(|l| escape_html(l))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    r#"<div class="literalblock">{title}<pre class="literal">{code}</pre></div>"#,
                    title = title_html,
                    code = code_content
                )
            }
            Block::Blockquote { title, attribution, citation, children, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let body = self.render_blocks(children);
                let attr_html = match (attribution, citation) {
                    (Some(a), Some(c)) => format!(
                        r#"<div class="attribution">&#8212; {} <cite>{}</cite></div>"#,
                        escape_html(a),
                        escape_html(c)
                    ),
                    (Some(a), None) => format!(
                        r#"<div class="attribution">&#8212; {}</div>"#,
                        escape_html(a)
                    ),
                    (None, Some(c)) => format!(
                        r#"<div class="attribution"><cite>{}</cite></div>"#,
                        escape_html(c)
                    ),
                    (None, None) => String::new(),
                };
                format!(
                    r#"<div class="quoteblock">{title}<blockquote>{body}</blockquote>{attr}</div>"#,
                    title = title_html,
                    body = body,
                    attr = attr_html
                )
            }
            Block::Verse { title, attribution, citation, lines, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let content = lines
                    .iter()
                    .map(|l| escape_html(l))
                    .collect::<Vec<_>>()
                    .join("\n");
                let attr_html = match (attribution, citation) {
                    (Some(a), Some(c)) => format!(
                        r#"<div class="attribution">&#8212; {} <cite>{}</cite></div>"#,
                        escape_html(a),
                        escape_html(c)
                    ),
                    (Some(a), None) => format!(
                        r#"<div class="attribution">&#8212; {}</div>"#,
                        escape_html(a)
                    ),
                    (None, Some(c)) => format!(
                        r#"<div class="attribution"><cite>{}</cite></div>"#,
                        escape_html(c)
                    ),
                    (None, None) => String::new(),
                };
                format!(
                    r#"<div class="verseblock">{title}<pre class="verse">{content}</pre>{attr}</div>"#,
                    title = title_html,
                    content = content,
                    attr = attr_html
                )
            }
            Block::Sidebar { title, children, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="sidebar-title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let body = self.render_blocks(children);
                format!(
                    r#"<aside class="sidebarblock">{title}<div class="sidebar-content">{body}</div></aside>"#,
                    title = title_html,
                    body = body
                )
            }
            Block::Example { title, children, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="example-title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let body = self.render_blocks(children);
                format!(
                    r#"<div class="exampleblock">{title}<div class="example-content">{body}</div></div>"#,
                    title = title_html,
                    body = body
                )
            }
            Block::Table { title, rows, col_widths, frame, grid, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="table-title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let frame_class = frame.as_deref().unwrap_or("all");
                let grid_class = grid.as_deref().unwrap_or("all");

                let mut table_out = format!(
                    r#"<div class="tableblock">{title}<table class="table frame-{frame} grid-{grid}">"#,
                    title = title_html,
                    frame = frame_class,
                    grid = grid_class
                );

                if !col_widths.is_empty() {
                    table_out.push_str("<colgroup>");
                    for width in col_widths {
                        table_out.push_str(&format!(r#"<col style="width: {:.1}%;">"#, width));
                    }
                    table_out.push_str("</colgroup>");
                }

                table_out.push_str("<tbody>");
                for (row_idx, row) in rows.iter().enumerate() {
                    let is_header = row_idx == 0 && rows.len() > 1;
                    table_out.push_str("<tr>");
                    for cell in row {
                        let tag = if is_header { "th" } else { "td" };
                        let cell_html = self.render_blocks(cell);
                        table_out.push_str(&format!("<{tag}>{cell}</{tag}>", tag = tag, cell = cell_html));
                    }
                    table_out.push_str("</tr>");
                }
                table_out.push_str("</tbody></table></div>");
                table_out
            }
            Block::Image { title, target, alt, width, height, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let img_src = self.resolve_image_source(target);
                let mut style = String::new();
                if let Some(w) = width {
                    style.push_str(&format!("max-width:{};", escape_html(w)));
                }
                if let Some(h) = height {
                    style.push_str(&format!("height:{};", escape_html(h)));
                }
                let style_attr = if !style.is_empty() {
                    format!(r#" style="{}""#, style)
                } else {
                    String::new()
                };

                format!(
                    r#"<div class="imageblock">{title}<img src="{src}" alt="{alt}"{style} loading="lazy"></div>"#,
                    title = title_html,
                    src = img_src,
                    alt = escape_html(alt),
                    style = style_attr
                )
            }
            Block::HorizontalRule { .. } => "<hr class=\"horizontal-rule\">".to_string(),
            Block::Admonition { kind, children, .. } => {
                let k_lower = kind.to_ascii_lowercase();
                let icon_symbol = match k_lower.as_str() {
                    "note" => "&#9432;",
                    "tip" => "&#128161;",
                    "warning" => "&#9888;",
                    "caution" => "&#9888;",
                    "important" => "&#10071;",
                    _ => "&#9432;",
                };
                let body = self.render_blocks(children);
                format!(
                    r#"<div class="admonitionblock {k_lower}"><div class="admonition-header"><span class="admonition-icon">{icon}</span> <strong class="admonition-title">{kind}</strong></div><div class="admonition-content">{body}</div></div>"#,
                    k_lower = k_lower,
                    icon = icon_symbol,
                    kind = escape_html(kind),
                    body = body
                )
            }
            Block::Open { title, children, .. } => {
                let title_html = title
                    .as_ref()
                    .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
                    .unwrap_or_default();
                let body = self.render_blocks(children);
                format!(
                    r#"<div class="openblock">{title}<div class="openblock-content">{body}</div></div>"#,
                    title = title_html,
                    body = body
                )
            }
            Block::PageBreak { .. } => r#"<div class="page-break"></div>"#.to_string(),
            Block::Comment { .. } | Block::EmptyLine => String::new(),
            Block::Toc { .. } => {
                r#"<div class="toc"><div class="toctitle">Table of Contents</div></div>"#.to_string()
            }
        }
    }

    fn render_spans(&mut self, spans: &[InlineSpan]) -> String {
        let mut out = String::new();
        for span in spans {
            out.push_str(&self.render_span(span));
        }
        out
    }

    fn render_span(&mut self, span: &InlineSpan) -> String {
        match span {
            InlineSpan::Text(s) => escape_html(s),
            InlineSpan::Bold(inner) => format!("<strong>{}</strong>", self.render_spans(inner)),
            InlineSpan::Italic(inner) => format!("<em>{}</em>", self.render_spans(inner)),
            InlineSpan::Code(s) => format!("<code>{}</code>", escape_html(s)),
            InlineSpan::Monospace(inner) => format!("<code>{}</code>", self.render_spans(inner)),
            InlineSpan::DoubleQuote(inner) => format!("&ldquo;{}&rdquo;", self.render_spans(inner)),
            InlineSpan::SingleQuote(inner) => format!("&lsquo;{}&rsquo;", self.render_spans(inner)),
            InlineSpan::Link { url, display } => format!(
                r#"<a href="{}" target="_blank" rel="noopener noreferrer">{}</a>"#,
                escape_html(url),
                escape_html(display)
            ),
            InlineSpan::Xref { target, display } => {
                format!("<a href=\"#{}\" class=\"xref\">{}</a>", escape_html(target), escape_html(display))
            }
            InlineSpan::Strikethrough(inner) => format!("<s>{}</s>", self.render_spans(inner)),
            InlineSpan::Superscript(inner) => format!("<sup>{}</sup>", self.render_spans(inner)),
            InlineSpan::Subscript(inner) => format!("<sub>{}</sub>", self.render_spans(inner)),
            InlineSpan::Image { target, alt, width } => {
                let src = self.resolve_image_source(target);
                let w_attr = width.as_ref().map(|w| format!(r#" width="{}""#, escape_html(w))).unwrap_or_default();
                format!(r#"<img src="{}" alt="{}"{} class="inline-image">"#, src, escape_html(alt), w_attr)
            }
            InlineSpan::Icon { name, .. } => {
                format!(r#"<span class="icon icon-{}">:{}:</span>"#, escape_html(name), escape_html(name))
            }
            InlineSpan::Footnote { id, text } => {
                self.footnotes.push(text.clone());
                let idx = self.footnotes.len();
                let fn_id = id.clone().unwrap_or_else(|| format!("fn-{}", idx));
                format!(
                    "<sup class=\"footnote\" id=\"fnref-{idx}\"><a href=\"#{fn_id}\">[{idx}]</a></sup>",
                    idx = idx,
                    fn_id = fn_id
                )
            }
            InlineSpan::Callout(num) => format!(r#"<b class="conum">({})</b>"#, num),
            InlineSpan::Kbd(keys) => {
                let keys_html = keys
                    .iter()
                    .map(|k| format!("<kbd>{}</kbd>", escape_html(k)))
                    .collect::<Vec<_>>()
                    .join("+");
                format!(r#"<span class="keyseq">{}</span>"#, keys_html)
            }
            InlineSpan::Button(text) => format!(r#"<span class="btn">[{}]</span>"#, escape_html(text)),
            InlineSpan::Menu(items) => {
                let items_html = items
                    .iter()
                    .map(|i| format!(r#"<span class="menu-item">{}</span>"#, escape_html(i)))
                    .collect::<Vec<_>>()
                    .join(" &#9656; ");
                format!(r#"<span class="menuseq">{}</span>"#, items_html)
            }
            InlineSpan::Mark(inner) => format!("<mark>{}</mark>", self.render_spans(inner)),
            InlineSpan::Pass(val) => val.clone(),
        }
    }

    fn resolve_image_source(&self, target: &str) -> String {
        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with("data:") {
            return escape_html(target);
        }

        if let Some(dir) = self.notes_dir {
            let path = dir.join(target);
            if path.is_file() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                    let mime = match ext.as_str() {
                        "jpg" | "jpeg" => "image/jpeg",
                        "svg" => "image/svg+xml",
                        "png" => "image/png",
                        "gif" => "image/gif",
                        "webp" => "image/webp",
                        _ => "image/png",
                    };
                    let b64 = base64_encode(&bytes);
                    return format!("data:{};base64,{}", mime, b64);
                }
            }
        }

        escape_html(target)
    }

    fn render_footnotes(&self) -> String {
        if self.footnotes.is_empty() {
            return String::new();
        }
        let mut out = String::from(r#"<div id="footnotes"><hr><div class="footnotes-title">Footnotes</div><ol>"#);
        for (i, fn_text) in self.footnotes.iter().enumerate() {
            let idx = i + 1;
            out.push_str(&format!(
                "<li id=\"fn-{idx}\"><p>{} <a href=\"#fnref-{idx}\">&#8617;</a></p></li>",
                escape_html(fn_text),
                idx = idx
            ));
        }
        out.push_str("</ol></div>");
        out
    }
}

fn extract_title(blocks: &[Block]) -> Option<String> {
    for b in blocks {
        if let Block::Heading { level: 1, spans, .. } = b {
            return Some(spans.iter().map(|s| s.plain_text()).collect::<String>());
        }
    }
    None
}

/// Simple RFC 4648 Base64 Encoder without extra external dependencies
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(TABLE[(b0 >> 2) as usize] as char);
        result.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

const DOCUMENT_CSS: &str = r#"
:root {
    --bg-color: #ffffff;
    --text-color: #24292f;
    --heading-color: #1f2328;
    --border-color: #d0d7de;
    --code-bg: #18181c;
    --code-text: #f2f2f7;
    --inline-code-bg: #f3f4f6;
    --inline-code-text: #e11d48;
    --link-color: #0969da;
    --sidebar-bg: #f6f8fa;
    --example-bg: #f8fafc;
    --table-header-bg: #f6f8fa;
    --table-alt-bg: #f9fafb;
    --note-bg: #e0f2fe;
    --note-border: #0284c7;
    --tip-bg: #f0fdf4;
    --tip-border: #16a34a;
    --warning-bg: #fffbeb;
    --warning-border: #d97706;
    --caution-bg: #fff1f2;
    --caution-border: #e11d48;
    --shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-color: #0f1117;
        --text-color: #e6edf3;
        --heading-color: #ffffff;
        --border-color: #30363d;
        --code-bg: #161b22;
        --code-text: #f0f6fc;
        --inline-code-bg: #21262d;
        --inline-code-text: #ff7b72;
        --link-color: #58a6ff;
        --sidebar-bg: #161b22;
        --example-bg: #161b22;
        --table-header-bg: #21262d;
        --table-alt-bg: #161b22;
        --note-bg: #0c2d48;
        --note-border: #38bdf8;
        --tip-bg: #064e3b;
        --tip-border: #34d399;
        --warning-bg: #451a03;
        --warning-border: #fbbf24;
        --caution-bg: #4c0519;
        --caution-border: #fb7185;
        --shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
    }
}

* {
    box-sizing: border-box;
}

body.fishdoc-body {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    font-size: 16px;
    line-height: 1.65;
    background-color: var(--bg-color);
    color: var(--text-color);
    -webkit-font-smoothing: antialiased;
}

.fishdoc-container {
    max-width: 900px;
    margin: 0 auto;
    padding: 32px 24px 64px 24px;
}

.document-header {
    border-bottom: 2px solid var(--border-color);
    padding-bottom: 16px;
    margin-bottom: 32px;
}

.document-title {
    font-size: 2.25rem;
    font-weight: 700;
    color: var(--heading-color);
    margin: 0;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin-top: 1.8em;
    margin-bottom: 0.6em;
    font-weight: 600;
    line-height: 1.25;
    position: relative;
}

h1 { font-size: 1.85rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.3em; }
h2 { font-size: 1.5rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.2em; }
h3 { font-size: 1.25rem; }
h4 { font-size: 1.1rem; }
h5 { font-size: 1rem; }
h6 { font-size: 0.9rem; }

.anchor {
    position: absolute;
    left: -1.2em;
    color: var(--border-color);
    text-decoration: none;
    font-weight: normal;
    opacity: 0;
    transition: opacity 0.2s;
}

h1:hover .anchor, h2:hover .anchor, h3:hover .anchor, h4:hover .anchor {
    opacity: 1;
}

p {
    margin: 0 0 1em 0;
}

a {
    color: var(--link-color);
    text-decoration: none;
}

a:hover {
    text-decoration: underline;
}

code {
    font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace;
    font-size: 0.9em;
    background-color: var(--inline-code-bg);
    color: var(--inline-code-text);
    padding: 0.2em 0.4em;
    border-radius: 4px;
}

pre {
    font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace;
    font-size: 0.9em;
    margin: 1em 0;
    padding: 16px;
    overflow-x: auto;
    border-radius: 8px;
}

.listingblock, .literalblock {
    margin: 1.5em 0;
}

.listingblock pre, .literalblock pre {
    background-color: var(--code-bg);
    color: var(--code-text);
    border: 1px solid var(--border-color);
}

.listingblock .title, .literalblock .title, .table-title, .example-title, .sidebar-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--heading-color);
    margin-bottom: 6px;
}

blockquote {
    margin: 1.5em 0;
    padding: 12px 20px;
    border-left: 4px solid var(--link-color);
    background-color: var(--sidebar-bg);
    border-radius: 0 8px 8px 0;
}

.quoteblock .attribution, .verseblock .attribution {
    margin-top: 8px;
    font-size: 0.88rem;
    color: var(--text-color);
    opacity: 0.85;
    text-align: right;
}

.verseblock pre.verse {
    font-family: inherit;
    font-size: 1rem;
    line-height: 1.6;
    background: var(--sidebar-bg);
    border-left: 4px solid var(--link-color);
    color: var(--text-color);
    padding: 12px 20px;
    border-radius: 0 8px 8px 0;
}

.sidebarblock {
    margin: 1.5em 0;
    padding: 16px 20px;
    background-color: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 8px;
}

.exampleblock {
    margin: 1.5em 0;
    padding: 16px 20px;
    background-color: var(--example-bg);
    border: 1px solid var(--border-color);
    border-left: 4px solid var(--border-color);
    border-radius: 0 8px 8px 0;
}

/* Admonitions */
.admonitionblock {
    margin: 1.5em 0;
    padding: 14px 18px;
    border-radius: 8px;
    border-left: 5px solid;
}

.admonitionblock.note { background-color: var(--note-bg); border-color: var(--note-border); }
.admonitionblock.tip { background-color: var(--tip-bg); border-color: var(--tip-border); }
.admonitionblock.warning { background-color: var(--warning-bg); border-color: var(--warning-border); }
.admonitionblock.caution, .admonitionblock.important { background-color: var(--caution-bg); border-color: var(--caution-border); }

.admonition-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    margin-bottom: 8px;
}

.admonition-icon {
    font-size: 1.2rem;
}

/* Tables */
table.table {
    width: 100%;
    border-collapse: collapse;
    margin: 1.5em 0;
    font-size: 0.95rem;
}

table.table th, table.table td {
    padding: 10px 14px;
    border: 1px solid var(--border-color);
    text-align: left;
    vertical-align: top;
}

table.table th {
    background-color: var(--table-header-bg);
    font-weight: 600;
}

table.table tr:nth-child(even) td {
    background-color: var(--table-alt-bg);
}

/* Lists */
ul, ol {
    margin: 0.8em 0;
    padding-left: 28px;
}

li {
    margin: 0.35em 0;
}

.checklist-item {
    list-style-type: none;
    margin-left: -20px;
}

.checklist-checkbox {
    margin-right: 8px;
    transform: scale(1.15);
}

.conum {
    display: inline-block;
    color: var(--link-color);
    font-weight: 700;
    margin-right: 4px;
}

.keyseq kbd {
    display: inline-block;
    padding: 2px 6px;
    font-size: 0.85em;
    font-family: ui-monospace, monospace;
    background: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    box-shadow: 0 1px 0 var(--border-color);
}

.btn {
    display: inline-block;
    padding: 2px 8px;
    font-size: 0.85em;
    background: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-weight: 500;
}

.menuseq {
    font-weight: 500;
}

img {
    max-width: 100%;
    height: auto;
    border-radius: 6px;
}

.imageblock {
    margin: 1.5em 0;
    text-align: center;
}

.document-footer {
    margin-top: 48px;
    padding-top: 20px;
    border-top: 1px solid var(--border-color);
    font-size: 0.85rem;
    color: var(--text-color);
    opacity: 0.7;
    text-align: center;
}

@media print {
    body.fishdoc-body { background: #fff; color: #000; font-size: 12pt; }
    .fishdoc-container { max-width: 100%; padding: 0; }
    .anchor { display: none; }
    .page-break { page-break-after: always; }
    pre, blockquote { page-break-inside: avoid; }
    table { page-break-inside: avoid; }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adoc_to_html5_simple() {
        let adoc = r#"
= My Test Document
Author Name

This is a paragraph with *bold*, _italic_, and `code` text.

== Section One

- [ ] Task 1
- [x] Task 2
- Normal item

[NOTE]
====
This is a helpful note!
====

[source,rust]
----
fn main() {
    println!("Hello World!");
}
----
"#;
        let html = adoc_to_html5(adoc, "My Test Document", None);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>My Test Document</title>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("class=\"admonitionblock note\""));
        assert!(html.contains("class=\"code-block language-rust\""));
    }

    #[test]
    fn test_html_table_rendering() {
        let adoc = r#"
|===
| Header 1 | Header 2

| Cell 1
| Cell 2
|===
"#;
        let html = adoc_to_html5(adoc, "Table Test", None);
        assert!(html.contains("<table class=\"table"));
        assert!(html.contains("<th>"));
        assert!(html.contains("Header 1"));
        assert!(html.contains("Cell 1"));
    }
}
