use std::collections::HashMap;
use std::path::Path;

use crate::block::{AdmonitionKind, Block};
use crate::inline::InlineSpan;
use crate::escape::escape_html;
use super::{sanitize_url_scheme, base64_encode};
use super::icons::{get_admonition_svg_icon, get_standard_svg_icon};

#[derive(Debug, Clone)]
pub(crate) struct TocHeading {
    pub(crate) level: u8,
    pub(crate) title: String,
    pub(crate) id: String,
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

pub(crate) fn collect_headings_recursive(
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

pub(crate) fn render_toc_tree(headings: &[TocHeading]) -> String {
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

pub(crate) struct HtmlRenderContext<'a> {
    pub(crate) notes_dir: Option<&'a Path>,
    footnotes: Vec<FootnoteItem>,
    footnote_id_to_idx: HashMap<String, usize>,
    heading_counts: HashMap<String, usize>,
    toc_headings: Vec<TocHeading>,
}

impl<'a> HtmlRenderContext<'a> {
    pub(crate) fn new(notes_dir: Option<&'a Path>, blocks: &[Block]) -> Self {
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

    pub(crate) fn render_blocks(&mut self, blocks: &[Block]) -> String {
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
            Block::Toc { depth, .. } => {
                let filtered: Vec<TocHeading> = if let Some(max) = depth {
                    self.toc_headings.iter().filter(|h| h.level <= *max).cloned().collect()
                } else {
                    self.toc_headings.clone()
                };
                let toc_content = render_toc_tree(&filtered);
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

    fn render_title_html(title: Option<&str>) -> String {
        Self::render_title_html_class(title, "title")
    }

    fn render_title_html_class(title: Option<&str>, css_class: &str) -> String {
        title
            .map(|t| format!(r#"<div class="{}">{}</div>"#, css_class, escape_html(t)))
            .unwrap_or_default()
    }

    fn render_attribution_html(attribution: Option<&str>, citation: Option<&str>) -> String {
        match (attribution, citation) {
            (Some(a), Some(c)) => format!(
                r#"<div class="attribution">&#8212; {} <cite>{}</cite></div>"#,
                escape_html(a), escape_html(c)
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
        }
    }

    fn render_code_block(&self, title: Option<&str>, language: Option<&str>, lines: &[String]) -> String {
        // Svgbob: render ASCII art to inline SVG
        if language.map(|l| l.eq_ignore_ascii_case("svgbob")).unwrap_or(false) {
            let title_html = Self::render_title_html(title);
            let source = lines.join("\n");
            let svg = crate::diagram::render_svgbob(&source);
            return format!(
                r#"<div class="listingblock">{title}<div class="diagram">{svg}</div></div>"#,
                title = title_html,
                svg = svg
            );
        }

        let title_html = Self::render_title_html(title);
        let lang_class = language
            .map(|l| format!(" language-{}", escape_html(l)))
            .unwrap_or_default();

        // Use syntax highlighting when a language is specified
        if let Some(lang) = language {
            let code = lines.join("\n");
            let normalized = crate::highlight::normalize_language(lang);
            let highlighted = crate::highlight::highlight_code(&code, &normalized);
            return format!(
                r#"<div class="listingblock">{title}<pre class="highlight"><code class="code-block{lang}">{code}</code></pre></div>"#,
                title = title_html,
                lang = lang_class,
                code = highlighted
            );
        }

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
        let title_html = Self::render_title_html(title);
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
        let title_html = Self::render_title_html(title);
        let body = self.render_blocks(children);
        let attr_html = Self::render_attribution_html(attribution, citation);
        format!(
            r#"<div class="quoteblock">{title}<blockquote>{body}</blockquote>{attr}</div>"#,
            title = title_html,
            body = body,
            attr = attr_html
        )
    }

    fn render_verse_block(&self, title: Option<&str>, attribution: Option<&str>, citation: Option<&str>, lines: &[String]) -> String {
        let title_html = Self::render_title_html(title);
        let content = lines
            .iter()
            .map(|l| escape_html(l))
            .collect::<Vec<_>>()
            .join("\n");
        let attr_html = Self::render_attribution_html(attribution, citation);
        format!(
            r#"<div class="verseblock">{title}<pre class="verse">{content}</pre>{attr}</div>"#,
            title = title_html,
            content = content,
            attr = attr_html
        )
    }

    fn render_sidebar_block(&mut self, title: Option<&str>, children: &[Block]) -> String {
        let title_html = Self::render_title_html_class(title, "sidebar-title");
        let body = self.render_blocks(children);
        format!(
            r#"<aside class="sidebarblock">{title}<div class="sidebar-content">{body}</div></aside>"#,
            title = title_html,
            body = body
        )
    }

    fn render_example_block(&mut self, title: Option<&str>, children: &[Block]) -> String {
        let title_html = Self::render_title_html_class(title, "example-title");
        let body = self.render_blocks(children);
        format!(
            r#"<div class="exampleblock">{title}<div class="example-content">{body}</div></div>"#,
            title = title_html,
            body = body
        )
    }

    fn render_table_block(&mut self, title: Option<&str>, rows: &[Vec<crate::block::TableCell>], col_widths: &[f64], frame: Option<&str>, grid: Option<&str>) -> String {
        let title_html = Self::render_title_html_class(title, "table-title");
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
        let title_html = Self::render_title_html(title);
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

    fn render_admonition_block(&mut self, title: Option<&str>, kind: &AdmonitionKind, children: &[Block]) -> String {
        let k_lower = kind.as_str().to_ascii_lowercase();
        let svg_icon = get_admonition_svg_icon(*kind);
        let title_text = match title {
            Some(t) if !t.is_empty() => format!("{}: {}", kind.as_str(), escape_html(t)),
            _ => escape_html(kind.as_str()),
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
        let title_html = Self::render_title_html(title);
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
            InlineSpan::Link { url, display } => {
                let safe_url = sanitize_url_scheme(url);
                format!(
                    r#"<a href="{}" target="_blank" rel="noopener noreferrer">{}</a>"#,
                    escape_html(&safe_url),
                    escape_html(display)
                )
            }
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

    pub(crate) fn render_footnotes(&self) -> String {
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
