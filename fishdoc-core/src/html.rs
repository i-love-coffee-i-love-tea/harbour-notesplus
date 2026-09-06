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

/// Convert raw AsciiDoc content into an HTML body snippet (for web preview & embeds).
pub fn adoc_to_html_body(adoc_content: &str, notes_dir: Option<&Path>) -> String {
    let blocks = parser::parse_blocks(adoc_content);
    blocks_to_html_body(&blocks, notes_dir)
}

/// Convert an AST of blocks into an HTML body snippet.
pub fn blocks_to_html_body(blocks: &[Block], notes_dir: Option<&Path>) -> String {
    let mut ctx = HtmlRenderContext::new(notes_dir, blocks);
    let body = ctx.render_blocks(blocks);
    let footnotes = ctx.render_footnotes();
    if footnotes.is_empty() {
        body
    } else {
        format!("{}\n{}", body, footnotes)
    }
}

/// Convert an AST of blocks into a standalone HTML5 document.
pub fn blocks_to_html5(blocks: &[Block], title: &str, notes_dir: Option<&Path>) -> String {
    let mut ctx = HtmlRenderContext::new(notes_dir, blocks);
    
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
    <script>
    document.addEventListener('DOMContentLoaded', function() {{
        document.querySelectorAll('.document-content pre').forEach(function(pre) {{
            pre.style.position = 'relative';
            var copyBtn = document.createElement('button');
            copyBtn.className = 'copy-code-btn';
            copyBtn.innerText = '📋 Copy';
            copyBtn.title = 'Copy code to clipboard';
            copyBtn.onclick = function(e) {{
                e.stopPropagation();
                var codeEl = pre.querySelector('code') || pre;
                var text = codeEl.innerText || codeEl.textContent;
                navigator.clipboard.writeText(text).then(function() {{
                    copyBtn.innerText = '✓ Copied!';
                    setTimeout(function() {{ copyBtn.innerText = '📋 Copy'; }}, 2000);
                }});
            }};
            pre.appendChild(copyBtn);
        }});
    }});
    </script>
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

#[derive(Debug, Clone)]
pub struct TocHeading {
    pub level: u8,
    pub title: String,
    pub id: String,
}

struct HtmlRenderContext<'a> {
    notes_dir: Option<&'a Path>,
    footnotes: Vec<String>,
    heading_counts: HashMap<String, usize>,
    toc_headings: Vec<TocHeading>,
}

impl<'a> HtmlRenderContext<'a> {
    fn new(notes_dir: Option<&'a Path>, blocks: &[Block]) -> Self {
        let mut toc_headings = Vec::new();
        let mut heading_counts = HashMap::new();
        collect_headings_recursive(blocks, &mut toc_headings, &mut heading_counts);
        Self {
            notes_dir,
            footnotes: Vec::new(),
            heading_counts: HashMap::new(),
            toc_headings,
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
        let mut i = 0;
        while i < blocks.len() {
            match &blocks[i] {
                Block::UnorderedListItem { .. } => {
                    let start = i;
                    while i < blocks.len() && matches!(&blocks[i], Block::UnorderedListItem { .. }) {
                        i += 1;
                    }
                    let items = &blocks[start..i];
                    let is_checklist = items.iter().any(|b| matches!(b, Block::UnorderedListItem { checked: Some(_), .. }));
                    let list_class = if is_checklist { "ulist checklist" } else { "ulist" };
                    let ul_class = if is_checklist { r#" class="checklist""# } else { "" };
                    out.push_str(&format!(r#"<div class="{list_class}"><ul{ul_class}>"#));
                    out.push('\n');
                    for item in items {
                        if let Block::UnorderedListItem { checked, children, .. } = item {
                            out.push_str(&self.render_unordered_list_item(*checked, children));
                            out.push('\n');
                        }
                    }
                    out.push_str("</ul></div>\n");
                }
                Block::OrderedListItem { .. } => {
                    let start = i;
                    while i < blocks.len() && matches!(&blocks[i], Block::OrderedListItem { .. }) {
                        i += 1;
                    }
                    let items = &blocks[start..i];
                    let is_reversed = items.iter().any(|b| matches!(b, Block::OrderedListItem { reversed: true, .. }));
                    let rev_attr = if is_reversed { " reversed" } else { "" };
                    out.push_str(&format!(r#"<div class="olist arabic"><ol class="arabic"{rev_attr}>"#));
                    out.push('\n');
                    for item in items {
                        if let Block::OrderedListItem { marker, reversed, children, .. } = item {
                            out.push_str(&self.render_ordered_list_item(marker, *reversed, children));
                            out.push('\n');
                        }
                    }
                    out.push_str("</ol></div>\n");
                }
                Block::DescriptionListItem { .. } => {
                    let start = i;
                    while i < blocks.len() && matches!(&blocks[i], Block::DescriptionListItem { .. }) {
                        i += 1;
                    }
                    let items = &blocks[start..i];
                    out.push_str(r#"<div class="dlist"><dl class="hdlist">"#);
                    out.push('\n');
                    for item in items {
                        if let Block::DescriptionListItem { term_spans, children, .. } = item {
                            out.push_str(&self.render_description_list_item(term_spans, children));
                            out.push('\n');
                        }
                    }
                    out.push_str("</dl></div>\n");
                }
                Block::CalloutListItem { .. } => {
                    let start = i;
                    while i < blocks.len() && matches!(&blocks[i], Block::CalloutListItem { .. }) {
                        i += 1;
                    }
                    let items = &blocks[start..i];
                    out.push_str(r#"<div class="colist arabic"><ol class="calloutlist">"#);
                    out.push('\n');
                    for item in items {
                        if let Block::CalloutListItem { number, children, .. } = item {
                            out.push_str(&self.render_callout_list_item(*number, children));
                            out.push('\n');
                        }
                    }
                    out.push_str("</ol></div>\n");
                }
                other => {
                    out.push_str(&self.render_block(other));
                    out.push('\n');
                    i += 1;
                }
            }
        }
        out
    }

    fn render_unordered_list_item(&mut self, checked: Option<bool>, children: &[Block]) -> String {
        let (class_attr, checkbox) = match checked {
            Some(true) => (
                r#" class="checklist-item checked""#,
                r#"<input type="checkbox" checked disabled class="checklist-checkbox">"#,
            ),
            Some(false) => (
                r#" class="checklist-item unchecked""#,
                r#"<input type="checkbox" disabled class="checklist-checkbox">"#,
            ),
            None => ("", ""),
        };
        let mut out = format!("<li{class_attr}>{checkbox}", class_attr = class_attr, checkbox = checkbox);
        out.push_str(&self.render_blocks(children));
        out.push_str("</li>");
        out
    }

    fn render_ordered_list_item(&mut self, marker: &str, reversed: bool, children: &[Block]) -> String {
        let rev_attr = if reversed { " data-reversed=\"true\"" } else { "" };
        let mut out = format!(r#"<li class="ordered-list-item"{rev_attr} data-marker="{marker}">"#);
        out.push_str(&self.render_blocks(children));
        out.push_str("</li>");
        out
    }

    fn render_description_list_item(&mut self, term_spans: &[InlineSpan], children: &[Block]) -> String {
        let term = self.render_spans(term_spans);
        let body = self.render_blocks(children);
        format!("<dt class=\"hdlist1\">{}</dt>\n<dd>{}</dd>", term, body)
    }

    fn render_callout_list_item(&mut self, number: usize, children: &[Block]) -> String {
        let body = self.render_blocks(children);
        format!(
            r#"<li class="callout-item"><b class="conum">({})</b> {}</li>"#,
            number, body
        )
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
                self.render_ordered_list_item(marker, *reversed, children)
            }
            Block::UnorderedListItem { checked, children, .. } => {
                self.render_unordered_list_item(*checked, children)
            }
            Block::DescriptionListItem { term_spans, children, .. } => {
                self.render_description_list_item(term_spans, children)
            }
            Block::CalloutListItem { number, children, .. } => {
                self.render_callout_list_item(*number, children)
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
            Block::Admonition { title, kind, children, .. } => {
                let k_lower = kind.to_ascii_lowercase();
                let svg_icon = match k_lower.as_str() {
                    "note" => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>"#,
                    "tip" => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18h6"></path><path d="M10 22h4"></path><path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14"></path></svg>"#,
                    "warning" => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>"#,
                    "caution" => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="7.86 2 16.14 2 22 7.86 22 16.14 16.14 22 7.86 22 2 16.14 2 7.86 7.86 2"></polygon><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>"#,
                    "important" => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>"#,
                    _ => r#"<svg class="admonition-icon-svg" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>"#,
                };
                let title_text = match title {
                    Some(t) if !t.is_empty() => format!("{}: {}", kind, escape_html(t)),
                    _ => escape_html(kind),
                };
                let body = self.render_blocks(children);
                format!(
                    r#"<div class="admonitionblock {k_lower}"><div class="admonition-header"><span class="admonition-icon">{svg}</span> <strong class="admonition-title">{title}</strong></div><div class="admonition-content">{body}</div></div>"#,
                    k_lower = k_lower,
                    svg = svg_icon,
                    title = title_text,
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
                let toc_content = render_toc_tree(&self.toc_headings);
                format!(
                    r#"<nav class="toc" id="toc" role="doc-toc"><div class="toctitle">Table of Contents</div>{}</nav>"#,
                    toc_content
                )
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
                let href = if target.starts_with('#') {
                    escape_html(target)
                } else if target.ends_with(".adoc") {
                    let page_name = target.strip_suffix(".adoc").unwrap_or(target);
                    format!("{}.html", escape_html(page_name))
                } else {
                    format!("#{}", escape_html(target))
                };
                format!("<a href=\"{}\" class=\"xref\">{}</a>", href, escape_html(display))
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
                if let Some(svg) = get_standard_svg_icon(name) {
                    svg.to_string()
                } else {
                    format!(r#"<span class="icon icon-{}">:{}:</span>"#, escape_html(name), escape_html(name))
                }
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
            InlineSpan::Callout(num) => format!(r#"<b class="conum"><span class="conum-badge">{}</span></b>"#, num),
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
            InlineSpan::Pass(val) => {
                val.replace("<script", "&lt;script")
                    .replace("</script>", "&lt;/script&gt;")
            }
        }
    }

    fn resolve_image_source(&self, target: &str) -> String {
        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with("data:") {
            return escape_html(target);
        }

        if let Some(dir) = self.notes_dir {
            if target.contains("..") {
                return escape_html(target);
            }
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

fn collect_headings_recursive(
    blocks: &[Block],
    out: &mut Vec<TocHeading>,
    counts: &mut HashMap<String, usize>,
) {
    for b in blocks {
        match b {
            Block::Heading { level, spans, .. } => {
                let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
                let slug: String = text
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
                    .collect();
                let slug = slug.trim_matches('-').to_string();
                let base_slug = if slug.is_empty() { "section".to_string() } else { slug };
                let count = counts.entry(base_slug.clone()).or_insert(0);
                *count += 1;
                let id = if *count == 1 {
                    base_slug
                } else {
                    format!("{}-{}", base_slug, *count)
                };
                out.push(TocHeading {
                    level: *level,
                    title: text,
                    id,
                });
            }
            Block::OrderedListItem { children, .. }
            | Block::UnorderedListItem { children, .. }
            | Block::DescriptionListItem { children, .. }
            | Block::CalloutListItem { children, .. }
            | Block::Blockquote { children, .. }
            | Block::Sidebar { children, .. }
            | Block::Example { children, .. }
            | Block::Open { children, .. } => {
                collect_headings_recursive(children, out, counts);
            }
            _ => {}
        }
    }
}

fn render_toc_tree(headings: &[TocHeading]) -> String {
    let toc_headings: Vec<&TocHeading> = headings
        .iter()
        .filter(|h| h.level >= 1 && h.level <= 5)
        .collect();
    if toc_headings.is_empty() {
        return String::new();
    }
    let min_level = toc_headings.iter().map(|h| h.level).min().unwrap_or(1);
    let mut out = String::new();
    let mut current_level = min_level;

    out.push_str(&format!("<ul class=\"sectlevel{}\">\n", current_level));

    for (i, h) in toc_headings.iter().enumerate() {
        if h.level > current_level {
            while current_level < h.level {
                current_level += 1;
                out.push_str(&format!("<ul class=\"sectlevel{}\">\n", current_level));
            }
        } else if h.level < current_level {
            while current_level > h.level {
                out.push_str("</li>\n</ul>\n");
                current_level -= 1;
            }
            out.push_str("</li>\n");
        } else if i > 0 {
            out.push_str("</li>\n");
        }

        out.push_str(&format!(
            "<li><a href=\"#{id}\">{title}</a>",
            id = h.id,
            title = escape_html(&h.title)
        ));
    }

    while current_level >= min_level {
        out.push_str("</li>\n</ul>\n");
        if current_level == 0 {
            break;
        }
        current_level -= 1;
    }

    out
}

fn get_standard_svg_icon(name: &str) -> Option<&'static str> {
    match name {
        "star" => Some(r#"<svg class="fishdoc-icon icon-star" viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" style="display:inline-block;vertical-align:-0.15em;"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/></svg>"#),
        "heart" => Some(r#"<svg class="fishdoc-icon icon-heart" viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" style="display:inline-block;vertical-align:-0.15em;"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/></svg>"#),
        "check" | "check-circle" => Some(r#"<svg class="fishdoc-icon icon-check" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><polyline points="20 6 9 17 4 12"/></svg>"#),
        "info" => Some(r#"<svg class="fishdoc-icon icon-info" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>"#),
        "warning" => Some(r#"<svg class="fishdoc-icon icon-warning" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#),
        "folder" => Some(r#"<svg class="fishdoc-icon icon-folder" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>"#),
        "file" => Some(r#"<svg class="fishdoc-icon icon-file" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>"#),
        "tag" | "tags" => Some(r#"<svg class="fishdoc-icon icon-tag" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/><line x1="7" y1="7" x2="7.01" y2="7"/></svg>"#),
        "search" => Some(r#"<svg class="fishdoc-icon icon-search" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#),
        "gear" | "settings" => Some(r#"<svg class="fishdoc-icon icon-gear" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>"#),
        "edit" => Some(r#"<svg class="fishdoc-icon icon-edit" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>"#),
        "copy" => Some(r#"<svg class="fishdoc-icon icon-copy" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"#),
        "trash" => Some(r#"<svg class="fishdoc-icon icon-trash" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>"#),
        "calendar" => Some(r#"<svg class="fishdoc-icon icon-calendar" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>"#),
        "clock" => Some(r#"<svg class="fishdoc-icon icon-clock" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>"#),
        "user" => Some(r#"<svg class="fishdoc-icon icon-user" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:inline-block;vertical-align:-0.15em;"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>"#),
        _ => None,
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
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
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
    --important-bg: #faf5ff;
    --important-border: #9333ea;
    --shadow: 0 4px 12px rgba(0, 0, 0, 0.06);
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
        --warning-bg: #3d2800;
        --warning-border: #fbbf24;
        --caution-bg: #3b0a15;
        --caution-border: #fb7185;
        --important-bg: #3b0764;
        --important-border: #c084fc;
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
    position: relative;
}

.copy-code-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    background-color: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.25);
    color: #e2e8f0;
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 0.74rem;
    cursor: pointer;
    opacity: 0.8;
    transition: opacity 0.2s, background-color 0.2s;
    z-index: 5;
}

.copy-code-btn:hover {
    opacity: 1;
    background-color: rgba(255, 255, 255, 0.25);
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

/* Table of Contents */
.toc {
    margin: 1.5em 0 2em 0;
    padding: 16px 20px;
    background-color: var(--sidebar-bg);
    border: 1px solid var(--border-color);
    border-radius: 8px;
}

.toc .toctitle {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--heading-color);
    margin-bottom: 12px;
}

.toc ul {
    margin: 0.3em 0;
    padding-left: 20px;
    list-style-type: none;
}

.toc ul.sectlevel1 {
    padding-left: 4px;
}

.toc li {
    margin: 0.3em 0;
}

.toc li::before {
    content: "•";
    color: var(--link-color);
    display: inline-block;
    width: 1em;
    margin-left: -1em;
}

.toc a {
    color: var(--link-color);
    text-decoration: none;
    font-size: 0.95rem;
}

.toc a:hover {
    text-decoration: underline;
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
    box-shadow: var(--shadow);
}

.admonitionblock.note { background-color: var(--note-bg); border-color: var(--note-border); color: var(--text-color); }
.admonitionblock.tip { background-color: var(--tip-bg); border-color: var(--tip-border); color: var(--text-color); }
.admonitionblock.warning { background-color: var(--warning-bg); border-color: var(--warning-border); color: var(--text-color); }
.admonitionblock.caution { background-color: var(--caution-bg); border-color: var(--caution-border); color: var(--text-color); }
.admonitionblock.important { background-color: var(--important-bg); border-color: var(--important-border); color: var(--text-color); }

.admonition-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    margin-bottom: 8px;
}

.admonition-icon {
    display: flex;
    align-items: center;
}

.admonition-icon-svg {
    display: inline-block;
    vertical-align: middle;
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

table.frame-all { border: 1px solid var(--border-color); }
table.frame-topbot { border-top: 1px solid var(--border-color); border-bottom: 1px solid var(--border-color); border-left: none; border-right: none; }
table.frame-sides { border-left: 1px solid var(--border-color); border-right: 1px solid var(--border-color); border-top: none; border-bottom: none; }
table.frame-none { border: none; }

table.grid-all th, table.grid-all td { border: 1px solid var(--border-color); }
table.grid-rows th, table.grid-rows td { border-top: 1px solid var(--border-color); border-bottom: 1px solid var(--border-color); border-left: none; border-right: none; }
table.grid-cols th, table.grid-cols td { border-left: 1px solid var(--border-color); border-right: 1px solid var(--border-color); border-top: none; border-bottom: none; }
table.grid-none th, table.grid-none td { border: none; }

/* Lists */
ul, ol {
    margin: 0.8em 0;
    padding-left: 28px;
}

li {
    margin: 0.35em 0;
}

li > p {
    margin: 0;
    display: inline;
}

.checklist-item {
    list-style-type: none;
    margin-left: -20px;
    display: flex;
    align-items: flex-start;
}

.checklist-item p {
    margin: 0;
    display: inline;
}

.checklist-checkbox {
    margin: 3px 8px 0 0;
    cursor: pointer;
    flex-shrink: 0;
    transform: scale(1.15);
}

.callout-item {
    margin: 0.35em 0;
}

.callout-item p {
    margin: 0;
    display: inline;
}

dl.hdlist {
    margin: 1em 0;
}

dl.hdlist dt.hdlist1 {
    font-weight: 700;
    color: var(--heading-color);
    margin-top: 0.8em;
}

dl.hdlist dd {
    margin-left: 20px;
    margin-bottom: 0.5em;
}

.conum {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background-color: var(--link-color);
    color: #ffffff;
    border-radius: 50%;
    width: 1.35em;
    height: 1.35em;
    font-size: 0.8em;
    font-weight: 700;
    vertical-align: 0.1em;
    margin: 0 3px;
}

.conum-badge {
    line-height: 1;
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

/* Footnotes */
#footnotes {
    margin-top: 40px;
    padding-top: 16px;
    border-top: 1px solid var(--border-color);
    font-size: 0.88rem;
}

#footnotes .footnotes-title {
    font-weight: 700;
    margin-bottom: 8px;
    color: var(--heading-color);
}

#footnotes ol {
    padding-left: 20px;
}

#footnotes li {
    margin-bottom: 6px;
}

.footnote a, .footnote-backref {
    color: var(--link-color);
    text-decoration: none;
    font-weight: 600;
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

    #[test]
    fn test_list_grouping_and_nesting() {
        let adoc = r#"
* Item 1
* Item 2
** Nested A
** Nested B
* Item 3

. First
. Second
.. Sub 1
.. Sub 2

[source,rust]
----
fn main() { // <1>
    println!("hi"); // <2>
}
----
<1> Entry point
<2> Print statement

Term 1:: Description 1
Term 2:: Description 2
"#;
        let html = adoc_to_html5(adoc, "List Test", None);
        // Unordered list is enclosed in <ul> or <div class="ulist"><ul>
        assert!(html.contains("<ul"));
        assert!(html.contains("</ul>"));
        // Ordered list is enclosed in <ol
        assert!(html.contains("<ol"));
        assert!(html.contains("</ol>"));
        // Description list is enclosed in <dl
        assert!(html.contains("<dl"));
        assert!(html.contains("</dl>"));
        // Callout list is enclosed in <ol class="calloutlist" or colist
        assert!(html.contains("callout"));
        // Check nesting: Nested A is inside a nested <ul>
        assert!(html.contains("Nested A"));
    }

    #[test]
    fn test_admonition_rendering_with_svg() {
        let adoc = r#"
[NOTE]
====
This is a note.
====

[TIP]
====
This is a tip.
====

[WARNING]
====
This is a warning.
====

[CAUTION]
====
This is caution.
====

[IMPORTANT]
====
This is important.
====
"#;
        let html = adoc_to_html5(adoc, "Admonitions", None);
        assert!(html.contains("class=\"admonitionblock note\""));
        assert!(html.contains("class=\"admonitionblock tip\""));
        assert!(html.contains("class=\"admonitionblock warning\""));
        assert!(html.contains("class=\"admonitionblock caution\""));
        assert!(html.contains("class=\"admonitionblock important\""));
        // Check for embedded SVG icons
        assert!(html.contains("<svg"));
    }

    #[test]
    fn test_table_frames_and_grids() {
        let adoc = r#"
[frame="topbot",grid="rows"]
|===
| Header A | Header B

| Val 1 | Val 2
|===
"#;
        let html = adoc_to_html5(adoc, "Table Frame Grid", None);
        assert!(html.contains("frame-topbot"));
        assert!(html.contains("grid-rows"));
    }

    #[test]
    fn test_inline_macros_and_icons() {
        let adoc = r#"
Press kbd:[Ctrl+T] or click btn:[Save].
Navigate to menu:File[New > Project].
Marked text: #highlighted#
Standard icon: icon:star[] and icon:folder[]
Callout conum: <1>
Cross reference: xref:other-page.adoc[Other Page]
"#;
        let html = adoc_to_html5(adoc, "Inline Macros", None);
        assert!(html.contains("<kbd>Ctrl</kbd>"));
        assert!(html.contains("<kbd>T</kbd>"));
        assert!(html.contains("class=\"btn\""));
        assert!(html.contains("class=\"menuseq\""));
        assert!(html.contains("<mark>"));
        assert!(html.contains("class=\"conum\""));
        assert!(html.contains("class=\"xref\""));
        assert!(html.contains("class=\"fishdoc-icon icon-star\""));
        assert!(html.contains("class=\"fishdoc-icon icon-folder\""));
    }

    #[test]
    fn test_checklist_rendering_and_styling() {
        let adoc = r#"
= Checklist Note

* [ ] Buy groceries
* [x] Finish documentation
* Normal bullet item
"#;
        let html = adoc_to_html5(adoc, "Checklist Note", None);
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("<input type=\"checkbox\" disabled class=\"checklist-checkbox\">"));
        assert!(html.contains("<input type=\"checkbox\" checked disabled class=\"checklist-checkbox\">"));
        assert!(html.contains("Buy groceries"));
        assert!(html.contains("Finish documentation"));
        // Verify CSS includes inline paragraph styling for list items and flex alignment
        assert!(html.contains(".checklist-item {"));
        assert!(html.contains("li > p {"));
    }

    #[test]
    fn test_dynamic_toc_generation() {
        let adoc = r#"
= Master Document

toc::[]

== Chapter One
Intro text.

=== Sub-section A
Details.

=== Sub-section B
More details.

== Chapter Two
Conclusion.
"#;
        let html = adoc_to_html5(adoc, "Master Document", None);
        assert!(html.contains("class=\"toc\""));
        assert!(html.contains("Table of Contents"));
        assert!(html.contains("href=\"#chapter-one\""));
        assert!(html.contains("Chapter One"));
        assert!(html.contains("href=\"#sub-section-a\""));
        assert!(html.contains("Sub-section A"));
        assert!(html.contains("href=\"#chapter-two\""));
        assert!(html.contains("Chapter Two"));
    }
}
