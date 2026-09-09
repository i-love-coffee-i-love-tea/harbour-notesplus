use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::inline::InlineSpan;
use crate::parser;

pub mod assets;
pub mod icons;
pub mod preprocess;

pub use assets::{DOCUMENT_CSS, DOCUMENT_JS};
pub use icons::{get_admonition_svg_icon, get_standard_svg_icon};
pub use preprocess::preprocess_html;

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
    <meta name="generator" content="Notes++ HTML5 Exporter">
    <title>{title}</title>
    <style>
{css}
    </style>
</head>
<body class="notes-body">
    <div class="notes-container">
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
{js}
    </script>
</body>
</html>"#,
        title = escape_html(&doc_title),
        css = DOCUMENT_CSS,
        js = DOCUMENT_JS,
        body = body_html,
        footnotes = footnotes_html,
        date = chrono::Local::now().format("%Y-%m-%d %H:%M"),
    )
}

/// Export a specific note page from `notes_dir` to an output HTML file.
/// `notes_dir` is used for reading .adoc files; `assets_dir` is used for image resolution.
pub fn export_page_to_html5(
    notes_dir: &Path,
    assets_dir: &Path,
    filename: &str,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let adoc_path = notes_dir.join(filename);
    let content = std::fs::read_to_string(&adoc_path)
        .map_err(|e| format!("Failed to read {}: {}", adoc_path.display(), e))?;

    let title = filename.strip_suffix(".adoc").unwrap_or(filename);
    let html = adoc_to_html5(&content, title, Some(assets_dir));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output dir {}: {}", parent.display(), e))?;
    }

    std::fs::write(output_path, html.as_bytes())
        .map_err(|e| format!("Failed to write HTML to {}: {}", output_path.display(), e))?;

    Ok(output_path.to_path_buf())
}

/// Export all .adoc files in `notes_dir` to HTML5 in `output_dir`.
/// `notes_dir` is used for reading .adoc files; `assets_dir` is used for image resolution.
pub fn export_all_pages_to_html5(
    notes_dir: &Path,
    assets_dir: &Path,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let mut exported = Vec::new();
    let entries = std::fs::read_dir(notes_dir)
        .map_err(|e| format!("Failed to read notes directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let out_filename = format!("{}.html", name.strip_suffix(".adoc").unwrap_or(name));
                let out_path = output_dir.join(out_filename);
                match export_page_to_html5(notes_dir, assets_dir, name, &out_path) {
                    Ok(p) => exported.push(p),
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Ok(exported)
}

#[derive(Debug, Clone)]
pub struct TocHeading {
    pub level: u8,
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone)]
struct FootnoteItem {
    id: String,
    text: String,
}

struct ListItemNode<'a> {
    block: &'a Block,
    sub_items: Vec<ListItemNode<'a>>,
}

fn build_list_tree<'a>(items: &'a [Block]) -> Vec<ListItemNode<'a>> {
    let mut root_nodes: Vec<ListItemNode<'a>> = Vec::new();
    let mut stack: Vec<(u8, ListItemNode<'a>)> = Vec::new();

    for item in items {
        let level = match item {
            Block::UnorderedListItem { level, .. } => *level,
            Block::OrderedListItem { level, .. } => *level,
            _ => 0,
        };

        let current_node = ListItemNode {
            block: item,
            sub_items: Vec::new(),
        };

        while let Some((top_level, _)) = stack.last() {
            if *top_level >= level {
                let (_, popped) = stack.pop().unwrap();
                if let Some((_, parent)) = stack.last_mut() {
                    parent.sub_items.push(popped);
                } else {
                    root_nodes.push(popped);
                }
            } else {
                break;
            }
        }

        stack.push((level, current_node));
    }

    while let Some((_, popped)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.sub_items.push(popped);
        } else {
            root_nodes.push(popped);
        }
    }

    root_nodes
}

struct HtmlRenderContext<'a> {
    notes_dir: Option<&'a Path>,
    footnotes: Vec<FootnoteItem>,
    footnote_id_to_idx: HashMap<String, usize>,
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
            footnote_id_to_idx: HashMap::new(),
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
                    let tree = build_list_tree(items);
                    out.push_str(&self.render_unordered_tree(&tree));
                    out.push('\n');
                }
                Block::OrderedListItem { .. } => {
                    let start = i;
                    while i < blocks.len() && matches!(&blocks[i], Block::OrderedListItem { .. }) {
                        i += 1;
                    }
                    let items = &blocks[start..i];
                    let tree = build_list_tree(items);
                    out.push_str(&self.render_ordered_tree(&tree));
                    out.push('\n');
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

    fn render_unordered_tree(&mut self, nodes: &[ListItemNode]) -> String {
        if nodes.is_empty() {
            return String::new();
        }
        let is_checklist = nodes.iter().any(|n| {
            matches!(n.block, Block::UnorderedListItem { checked: Some(_), .. })
        });
        let list_class = if is_checklist { "ulist checklist" } else { "ulist" };
        let ul_class = if is_checklist { r#" class="checklist""# } else { "" };
        let mut out = format!(r#"<div class="{list_class}"><ul{ul_class}>"#);
        out.push('\n');
        for node in nodes {
            if let Block::UnorderedListItem { checked, children, .. } = node.block {
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
                out.push_str(&format!("<li{class_attr}>{checkbox}"));
                out.push_str(self.render_blocks(children).trim());
                if !node.sub_items.is_empty() {
                    out.push('\n');
                    out.push_str(&self.render_unordered_tree(&node.sub_items));
                }
                out.push_str("</li>\n");
            }
        }
        out.push_str("</ul></div>");
        out
    }

    fn render_ordered_tree(&mut self, nodes: &[ListItemNode]) -> String {
        if nodes.is_empty() {
            return String::new();
        }
        let is_reversed = nodes.iter().any(|n| {
            matches!(n.block, Block::OrderedListItem { reversed: true, .. })
        });
        let rev_attr = if is_reversed { " reversed" } else { "" };
        let mut out = format!(r#"<div class="olist arabic"><ol class="arabic"{rev_attr}>"#);
        out.push('\n');
        for node in nodes {
            if let Block::OrderedListItem { marker, reversed, children, .. } = node.block {
                let rev_item_attr = if *reversed { " data-reversed=\"true\"" } else { "" };
                out.push_str(&format!(r#"<li class="ordered-list-item"{rev_item_attr} data-marker="{marker}">"#));
                out.push_str(self.render_blocks(children).trim());
                if !node.sub_items.is_empty() {
                    out.push('\n');
                    out.push_str(&self.render_ordered_tree(&node.sub_items));
                }
                out.push_str("</li>\n");
            }
        }
        out.push_str("</ol></div>");
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
            r#"<li class="callout-item"><b class="conum"><span class="conum-badge">{}</span></b> {}</li>"#,
            number, body.trim()
        )
    }

    fn render_block(&mut self, block: &Block) -> String {
        match block {
            Block::Heading { level, spans, .. } => self.render_heading_block(*level, spans),
            Block::Paragraph { spans, .. } => format!("<p>{}</p>", self.render_spans(spans)),
            Block::OrderedListItem { .. } => {
                let tree = build_list_tree(std::slice::from_ref(block));
                self.render_ordered_tree(&tree)
            }
            Block::UnorderedListItem { .. } => {
                let tree = build_list_tree(std::slice::from_ref(block));
                self.render_unordered_tree(&tree)
            }
            Block::DescriptionListItem { term_spans, children, .. } => {
                self.render_description_list_item(term_spans, children)
            }
            Block::CalloutListItem { number, children, .. } => {
                self.render_callout_list_item(*number, children)
            }
            Block::CodeBlock { title, language, lines, .. } => {
                self.render_code_block(title.as_deref(), language.as_deref(), lines)
            }
            Block::LiteralBlock { title, lines, .. } => {
                self.render_literal_block(title.as_deref(), lines)
            }
            Block::Blockquote { title, attribution, citation, children, .. } => {
                self.render_blockquote(title.as_deref(), attribution.as_deref(), citation.as_deref(), children)
            }
            Block::Verse { title, attribution, citation, lines, .. } => {
                self.render_verse_block(title.as_deref(), attribution.as_deref(), citation.as_deref(), lines)
            }
            Block::Sidebar { title, children, .. } => {
                self.render_sidebar_block(title.as_deref(), children)
            }
            Block::Example { title, children, .. } => {
                self.render_example_block(title.as_deref(), children)
            }
            Block::Table { title, rows, col_widths, frame, grid, .. } => {
                self.render_table_block(title.as_deref(), rows, col_widths, frame.as_deref(), grid.as_deref())
            }
            Block::Image { title, target, alt, width, height, .. } => {
                self.render_image_block(title.as_deref(), target, alt, width.as_deref(), height.as_deref())
            }
            Block::HorizontalRule { .. } => "<hr class=\"horizontal-rule\">".to_string(),
            Block::Admonition { title, kind, children, .. } => {
                self.render_admonition_block(title.as_deref(), kind, children)
            }
            Block::Open { title, children, .. } => {
                self.render_open_block(title.as_deref(), children)
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

    fn render_heading_block(&mut self, level: u8, spans: &[InlineSpan]) -> String {
        let text = spans.iter().map(|s| s.plain_text()).collect::<String>();
        let heading_id = self.generate_heading_id(&text);
        let tag_level = level.clamp(1, 6);
        let content = self.render_spans(spans);
        format!(
            "<h{lvl} id=\"{id}\" class=\"sect-heading sect{lvl}\">{content}</h{lvl}>",
            lvl = tag_level,
            id = heading_id,
            content = content
        )
    }

    fn render_code_block(&self, title: Option<&str>, language: Option<&str>, lines: &[String]) -> String {
        let title_html = title
            .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
            .unwrap_or_default();
        let lang_class = language
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

    fn render_literal_block(&self, title: Option<&str>, lines: &[String]) -> String {
        let title_html = title
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

    fn render_blockquote(&mut self, title: Option<&str>, attribution: Option<&str>, citation: Option<&str>, children: &[Block]) -> String {
        let title_html = title
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

    fn render_verse_block(&self, title: Option<&str>, attribution: Option<&str>, citation: Option<&str>, lines: &[String]) -> String {
        let title_html = title
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

    fn render_sidebar_block(&mut self, title: Option<&str>, children: &[Block]) -> String {
        let title_html = title
            .map(|t| format!(r#"<div class="sidebar-title">{}</div>"#, escape_html(t)))
            .unwrap_or_default();
        let body = self.render_blocks(children);
        format!(
            r#"<aside class="sidebarblock">{title}<div class="sidebar-content">{body}</div></aside>"#,
            title = title_html,
            body = body
        )
    }

    fn render_example_block(&mut self, title: Option<&str>, children: &[Block]) -> String {
        let title_html = title
            .map(|t| format!(r#"<div class="example-title">{}</div>"#, escape_html(t)))
            .unwrap_or_default();
        let body = self.render_blocks(children);
        format!(
            r#"<div class="exampleblock">{title}<div class="example-content">{body}</div></div>"#,
            title = title_html,
            body = body
        )
    }

    fn render_table_block(&mut self, title: Option<&str>, rows: &[Vec<crate::block::TableCell>], col_widths: &[f64], frame: Option<&str>, grid: Option<&str>) -> String {
        let title_html = title
            .map(|t| format!(r#"<div class="table-title">{}</div>"#, escape_html(t)))
            .unwrap_or_default();
        let frame_class = frame.unwrap_or("all");
        let grid_class = grid.unwrap_or("all");

        let mut table_out = format!(
            r#"<div class="tableblock">{title}<table class="table frame-{frame} grid-{grid}">"#,
            title = title_html,
            frame = frame_class,
            grid = grid_class
        );

        if !col_widths.is_empty() {
            let total_width: f64 = col_widths.iter().sum();
            table_out.push_str("<colgroup>");
            for width in col_widths {
                let pct = if total_width > 0.0 {
                    (width / total_width) * 100.0
                } else {
                    100.0 / col_widths.len() as f64
                };
                table_out.push_str(&format!(r#"<col style="width: {:.4}%;">"#, pct));
            }
            table_out.push_str("</colgroup>");
        }

        table_out.push_str("<tbody>");
        for (row_idx, row) in rows.iter().enumerate() {
            let is_header = row_idx == 0 && rows.len() > 1;
            table_out.push_str("<tr>");
            for cell in row {
                let tag = if is_header { "th" } else { "td" };
                let mut attrs = String::new();
                if cell.colspan > 1 {
                    attrs.push_str(&format!(r#" colspan="{}""#, cell.colspan));
                }
                let mut styles = Vec::new();
                if let Some(ref a) = cell.align {
                    styles.push(format!("text-align: {};", a));
                }
                if let Some(ref v) = cell.valign {
                    styles.push(format!("vertical-align: {};", v));
                }
                if !styles.is_empty() {
                    attrs.push_str(&format!(r#" style="{}""#, styles.join(" ")));
                }
                let cell_html = self.render_blocks(&cell.blocks);
                table_out.push_str(&format!("<{tag}{attrs}>{cell}</{tag}>", tag = tag, attrs = attrs, cell = cell_html.trim()));
            }
            table_out.push_str("</tr>");
        }
        table_out.push_str("</tbody></table></div>");
        table_out
    }

    fn render_image_block(&self, title: Option<&str>, target: &str, alt: &str, width: Option<&str>, height: Option<&str>) -> String {
        let title_html = title
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

    fn render_admonition_block(&mut self, title: Option<&str>, kind: &str, children: &[Block]) -> String {
        let k_lower = kind.to_ascii_lowercase();
        let svg_icon = get_admonition_svg_icon(&k_lower);
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

    fn render_open_block(&mut self, title: Option<&str>, children: &[Block]) -> String {
        let title_html = title
            .map(|t| format!(r#"<div class="title">{}</div>"#, escape_html(t)))
            .unwrap_or_default();
        let body = self.render_blocks(children);
        format!(
            r#"<div class="openblock">{title}<div class="openblock-content">{body}</div></div>"#,
            title = title_html,
            body = body
        )
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
                let existing_idx = id.as_ref().and_then(|id_str| self.footnote_id_to_idx.get(id_str).copied());
                if let Some(idx) = existing_idx {
                    let fn_id = self.footnotes[idx - 1].id.clone();
                    format!(
                        "<sup class=\"footnote\"><a href=\"#{}\">[{}]</a></sup>",
                        fn_id, idx
                    )
                } else if text.is_empty() && id.is_some() {
                    let idx = self.footnotes.len() + 1;
                    let fn_id = id.clone().unwrap_or_else(|| format!("fn-{}", idx));
                    self.footnote_id_to_idx.insert(fn_id.clone(), idx);
                    self.footnotes.push(FootnoteItem {
                        id: fn_id.clone(),
                        text: text.clone(),
                    });
                    format!(
                        "<sup class=\"footnote\" id=\"fnref-{}\"><a href=\"#{}\">[{}]</a></sup>",
                        idx, fn_id, idx
                    )
                } else {
                    let idx = self.footnotes.len() + 1;
                    let fn_id = id.clone().unwrap_or_else(|| format!("fn-{}", idx));
                    if let Some(id_str) = id.as_ref() {
                        self.footnote_id_to_idx.insert(id_str.clone(), idx);
                    }
                    self.footnote_id_to_idx.insert(fn_id.clone(), idx);
                    self.footnotes.push(FootnoteItem {
                        id: fn_id.clone(),
                        text: text.clone(),
                    });
                    format!(
                        "<sup class=\"footnote\" id=\"fnref-{}\"><a href=\"#{}\">[{}]</a></sup>",
                        idx, fn_id, idx
                    )
                }
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
            InlineSpan::Pass(val) => escape_html(val),
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
        let active_footnotes: Vec<(usize, &FootnoteItem)> = self
            .footnotes
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.text.is_empty())
            .map(|(i, item)| (i + 1, item))
            .collect();

        if active_footnotes.is_empty() {
            return String::new();
        }
        let mut out = String::from(r#"<div id="footnotes"><hr><div class="footnotes-title">Footnotes</div><ol>"#);
        for (idx, item) in active_footnotes {
            out.push_str(&format!(
                "<li id=\"{}\"><p>{} <a href=\"#fnref-{idx}\">&#8617;</a></p></li>",
                item.id,
                escape_html(&item.text),
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
        assert!(html.contains("<h2 id=\"section-one\" class=\"sect-heading sect2\">Section One</h2>"));
        assert!(!html.contains("<a class=\"anchor\""));
    }

    #[test]
    fn test_heading_rendering_without_anchor_hash() {
        let adoc = "== My Section\n\n=== My Subsection\n";
        let html = adoc_to_html_body(adoc, None);
        assert_eq!(
            html.trim(),
            "<h2 id=\"my-section\" class=\"sect-heading sect2\">My Section</h2>\n\n<h3 id=\"my-subsection\" class=\"sect-heading sect3\">My Subsection</h3>"
        );
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
        // Check admonition colors and sharp corners
        assert!(html.contains("--tip-border: #f0ad4e;"));
        assert!(html.contains("--warning-border: #d9534f;"));
        assert!(html.contains(".admonitionblock {\n    margin: 1.5em 0;\n    padding: 14px 18px;\n    border-radius: 0;"));
        assert!(html.contains(".toc {\n    margin: 1.5em 0 2em 0;\n    padding: 16px 20px;\n    background-color: var(--sidebar-bg);\n    border: 1px solid var(--border-color);\n    border-radius: 0;"));
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
        assert!(html.contains(".conum {"));
        assert!(html.contains("background-color: var(--note-bg);"));
        assert!(html.contains("color: var(--note-border);"));
        assert!(html.contains("class=\"xref\""));
        assert!(html.contains("class=\"notes-icon icon-star\""));
        assert!(html.contains("class=\"notes-icon icon-folder\""));
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
        // Verify CSS includes inline paragraph styling for list items and proper checklist styling
        assert!(html.contains(".checklist-item {"));
        assert!(html.contains("li > p {"));
    }

    #[test]
    fn test_nested_checklist_rendering() {
        let adoc = r#"
* [ ] Parent task
  * [x] Child task 1
  * [ ] Child task 2
"#;
        let html = adoc_to_html5(adoc, "Nested Checklist", None);
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("Parent task"));
        assert!(html.contains("Child task 1"));
        assert!(html.contains("Child task 2"));
        // Ensure child list is nested inside parent list item correctly
        assert!(html.contains("<li class=\"checklist-item unchecked\"><input type=\"checkbox\" disabled class=\"checklist-checkbox\"><p>Parent task</p>\n<div class=\"ulist checklist\"><ul class=\"checklist\">"));
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

    #[test]
    fn escape_html_escapes_angle_brackets() {
        assert_eq!(escape_html("<script>alert(1)</script>"),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    #[test]
    fn escape_html_escapes_quotes() {
        assert_eq!(escape_html(r#"a"b'c"#), "a&quot;b&#39;c");
    }

    #[test]
    fn escape_html_escapes_ampersand() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
    }

    #[test]
    fn test_callout_list_rendering_and_styles() {
        let adoc = r#"
[source,rust]
----
fn main() { // <1>
    println!("hello"); // <2>
}
----
<1> Entrypoint function
<2> Print statement
"#;
        let html = adoc_to_html5(adoc, "Callout Test", None);
        assert!(html.contains("<ol class=\"calloutlist\">"));
        assert!(html.contains("<li class=\"callout-item\"><b class=\"conum\"><span class=\"conum-badge\">1</span></b> <p>Entrypoint function</p></li>"));
        assert!(html.contains("<li class=\"callout-item\"><b class=\"conum\"><span class=\"conum-badge\">2</span></b> <p>Print statement</p></li>"));
        assert!(html.contains(".callout-item {"));
        assert!(html.contains("list-style-type: none;"));
    }

    #[test]
    fn test_footnote_deduplication_rendering() {
        let adoc = r#"
Here is a statement with footnote:defops[DefOps is great].
And here we refer to the same footnote:defops[].
And another reference: footnote:defops[].
"#;
        let html = adoc_to_html5(adoc, "Footnotes Test", None);
        assert!(html.contains("<sup class=\"footnote\" id=\"fnref-1\"><a href=\"#defops\">[1]</a></sup>"));
        assert!(html.contains("<sup class=\"footnote\"><a href=\"#defops\">[1]</a></sup>"));
        // Check that footnotes section contains only 1 footnote entry and no empty items
        assert!(html.contains("<li id=\"defops\"><p>DefOps is great <a href=\"#fnref-1\">&#8617;</a></p></li>"));
        assert!(!html.contains("<li id=\"fn-2\">"));
        assert!(!html.contains("<li id=\"fn-3\">"));
    }

    #[test]
    fn test_table_alignments_and_colspans_from_chronicles() {
        let adoc = r#"
[%header%footer,cols="2,2s,^4",grid=rows,frame=ends,width=75%,caption=]
|===
|Name |Title |Alias

|Sarah White
|President
|http://twitter.com/carbonfray[@carbonfray]

|Dan Allen
|Vice President
|http://twitter.com/mojavelinux[@mojavelinux]

3+^.e|Powered by Open Source
|===
"#;
        let html = adoc_to_html5(adoc, "Chronicles Table", None);
        assert!(html.contains("<table class=\"table frame-ends grid-rows\">"));
        assert!(html.contains("<col style=\"width: 25.0000%;\">"));
        assert!(html.contains("<col style=\"width: 50.0000%;\">"));
        assert!(html.contains("<td colspan=\"3\" style=\"text-align: center;\"><p><em>Powered by Open Source</em></p></td>"));
        assert!(html.contains("style=\"text-align: center;\""));
    }
}
