use serde::{Deserialize, Serialize};
use std::fmt;

use crate::inline::InlineSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AdmonitionKind {
    Note,
    Tip,
    Warning,
    Caution,
    Important,
}

impl AdmonitionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdmonitionKind::Note => "NOTE",
            AdmonitionKind::Tip => "TIP",
            AdmonitionKind::Warning => "WARNING",
            AdmonitionKind::Caution => "CAUTION",
            AdmonitionKind::Important => "IMPORTANT",
        }
    }
}

impl fmt::Display for AdmonitionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for AdmonitionKind {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TIP" => AdmonitionKind::Tip,
            "WARNING" => AdmonitionKind::Warning,
            "CAUTION" => AdmonitionKind::Caution,
            "IMPORTANT" => AdmonitionKind::Important,
            _ => AdmonitionKind::Note,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub blocks: Vec<Block>,
    #[serde(default = "default_colspan", skip_serializing_if = "is_default_colspan")]
    pub colspan: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valign: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<char>,
}

fn default_colspan() -> usize { 1 }
fn is_default_colspan(val: &usize) -> bool { *val <= 1 }

impl TableCell {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
            colspan: 1,
            align: None,
            valign: None,
            style: None,
        }
    }
}

impl From<Vec<Block>> for TableCell {
    fn from(blocks: Vec<Block>) -> Self {
        TableCell::new(blocks)
    }
}

impl std::ops::Deref for TableCell {
    type Target = [Block];
    fn deref(&self) -> &Self::Target {
        &self.blocks
    }
}

impl std::ops::DerefMut for TableCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.blocks
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<InlineSpan>,
        raw: String,
    },
    Paragraph {
        spans: Vec<InlineSpan>,
        raw: String,
    },
    OrderedListItem {
        level: u8,
        marker: String,
        #[serde(default)]
        reversed: bool,
        children: Vec<Block>,
        raw: String,
    },
    UnorderedListItem {
        level: u8,
        marker: String,
        checked: Option<bool>,
        children: Vec<Block>,
        raw: String,
    },
    DescriptionListItem {
        term: String,
        term_spans: Vec<InlineSpan>,
        children: Vec<Block>,
        raw: String,
    },
    CalloutListItem {
        number: usize,
        children: Vec<Block>,
        raw: String,
    },
    CodeBlock {
        title: Option<String>,
        language: Option<String>,
        lines: Vec<String>,
        raw: String,
    },
    LiteralBlock {
        title: Option<String>,
        lines: Vec<String>,
        raw: String,
    },
    Blockquote {
        title: Option<String>,
        attribution: Option<String>,
        citation: Option<String>,
        children: Vec<Block>,
        raw: String,
    },
    Verse {
        title: Option<String>,
        attribution: Option<String>,
        citation: Option<String>,
        lines: Vec<String>,
        spans: Vec<Vec<InlineSpan>>,
        raw: String,
    },
    Sidebar {
        title: Option<String>,
        children: Vec<Block>,
        raw: String,
    },
    Example {
        title: Option<String>,
        children: Vec<Block>,
        raw: String,
    },
    Table {
        title: Option<String>,
        rows: Vec<Vec<TableCell>>,
        col_widths: Vec<f64>,
        frame: Option<String>,
        grid: Option<String>,
        raw: String,
    },
    Image {
        title: Option<String>,
        target: String,
        alt: String,
        width: Option<String>,
        height: Option<String>,
        raw: String,
    },
    HorizontalRule {
        raw: String,
    },
    Admonition {
        title: Option<String>,
        kind: AdmonitionKind,
        children: Vec<Block>,
        raw: String,
    },
    Open {
        title: Option<String>,
        children: Vec<Block>,
        raw: String,
    },
    PageBreak {
        raw: String,
    },
    Comment {
        text: String,
        raw: String,
    },
    Toc {
        raw: String,
        depth: Option<u8>,
    },
    EmptyLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Heading,
    Paragraph,
    OrderedListItem,
    UnorderedListItem,
    DescriptionListItem,
    CalloutListItem,
    CodeBlock,
    LiteralBlock,
    Blockquote,
    Verse,
    Sidebar,
    Example,
    Table,
    Image,
    HorizontalRule,
    Admonition,
    Open,
    PageBreak,
    Comment,
    Toc,
    EmptyLine,
}

impl BlockKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockKind::Heading => "heading",
            BlockKind::Paragraph => "paragraph",
            BlockKind::OrderedListItem => "ordered_list_item",
            BlockKind::UnorderedListItem => "unordered_list_item",
            BlockKind::DescriptionListItem => "description_list_item",
            BlockKind::CalloutListItem => "callout_list_item",
            BlockKind::CodeBlock => "code_block",
            BlockKind::LiteralBlock => "literal_block",
            BlockKind::Blockquote => "blockquote",
            BlockKind::Verse => "verse",
            BlockKind::Sidebar => "sidebar",
            BlockKind::Example => "example",
            BlockKind::Table => "table",
            BlockKind::Image => "image",
            BlockKind::HorizontalRule => "horizontal_rule",
            BlockKind::Admonition => "admonition",
            BlockKind::Open => "open",
            BlockKind::PageBreak => "page_break",
            BlockKind::Comment => "comment",
            BlockKind::Toc => "toc",
            BlockKind::EmptyLine => "empty_line",
        }
    }
}

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Block {
    pub fn kind(&self) -> BlockKind {
        match self {
            Block::Heading { .. } => BlockKind::Heading,
            Block::Paragraph { .. } => BlockKind::Paragraph,
            Block::OrderedListItem { .. } => BlockKind::OrderedListItem,
            Block::UnorderedListItem { .. } => BlockKind::UnorderedListItem,
            Block::DescriptionListItem { .. } => BlockKind::DescriptionListItem,
            Block::CalloutListItem { .. } => BlockKind::CalloutListItem,
            Block::CodeBlock { .. } => BlockKind::CodeBlock,
            Block::LiteralBlock { .. } => BlockKind::LiteralBlock,
            Block::Blockquote { .. } => BlockKind::Blockquote,
            Block::Verse { .. } => BlockKind::Verse,
            Block::Sidebar { .. } => BlockKind::Sidebar,
            Block::Example { .. } => BlockKind::Example,
            Block::Table { .. } => BlockKind::Table,
            Block::Image { .. } => BlockKind::Image,
            Block::HorizontalRule { .. } => BlockKind::HorizontalRule,
            Block::Admonition { .. } => BlockKind::Admonition,
            Block::Open { .. } => BlockKind::Open,
            Block::PageBreak { .. } => BlockKind::PageBreak,
            Block::Comment { .. } => BlockKind::Comment,
            Block::Toc { .. } => BlockKind::Toc,
            Block::EmptyLine => BlockKind::EmptyLine,
        }
    }

    pub fn block_type(&self) -> &'static str {
        self.kind().as_str()
    }

    pub fn raw_text(&self) -> &str {
        match self {
            Block::Heading { raw, .. } => raw,
            Block::Paragraph { raw, .. } => raw,
            Block::OrderedListItem { raw, .. } => raw,
            Block::UnorderedListItem { raw, .. } => raw,
            Block::DescriptionListItem { raw, .. } => raw,
            Block::CalloutListItem { raw, .. } => raw,
            Block::CodeBlock { raw, .. } => raw,
            Block::LiteralBlock { raw, .. } => raw,
            Block::Blockquote { raw, .. } => raw,
            Block::Verse { raw, .. } => raw,
            Block::Sidebar { raw, .. } => raw,
            Block::Example { raw, .. } => raw,
            Block::Table { raw, .. } => raw,
            Block::Image { raw, .. } => raw,
            Block::HorizontalRule { raw } => raw,
            Block::Admonition { raw, .. } => raw,
            Block::Open { raw, .. } => raw,
            Block::PageBreak { raw } => raw,
            Block::Comment { raw, .. } => raw,
            Block::Toc { raw, .. } => raw,
            Block::EmptyLine => "",
        }
    }

    pub fn to_qvariant_map(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String(self.block_type().into()));
        map.insert("raw".into(), serde_json::Value::String(self.raw_text().into()));

        match self {
            Block::Heading { level, spans, .. } => {
                map.insert("level".into(), serde_json::Value::Number((*level).into()));
                map.insert("spans".into(), spans_to_json(spans));
            }
            Block::Paragraph { spans, .. } => {
                map.insert("spans".into(), spans_to_json(spans));
            }
            Block::OrderedListItem { level, marker, reversed, children, .. } => {
                map.insert("level".into(), serde_json::Value::Number((*level).into()));
                map.insert("marker".into(), serde_json::Value::String(marker.clone()));
                map.insert("reversed".into(), serde_json::Value::Bool(*reversed));
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::UnorderedListItem { level, marker, checked, children, .. } => {
                map.insert("level".into(), serde_json::Value::Number((*level).into()));
                map.insert("marker".into(), serde_json::Value::String(marker.clone()));
                if let Some(c) = checked {
                    map.insert("checked".into(), serde_json::Value::Bool(*c));
                }
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::DescriptionListItem { term, term_spans, children, .. } => {
                map.insert("term".into(), serde_json::Value::String(term.clone()));
                map.insert("term_spans".into(), spans_to_json(term_spans));
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::CalloutListItem { number, children, .. } => {
                map.insert("number".into(), serde_json::Value::Number((*number).into()));
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::CodeBlock { title, language, lines, .. } => {
                insert_title(&mut map, title);
                if let Some(lang) = language {
                    map.insert("language".into(), serde_json::Value::String(lang.clone()));
                    if lang.eq_ignore_ascii_case("svgbob") {
                        let source = lines.join("\n");
                        let svg = crate::diagram::render_svgbob(&source);
                        let b64 = crate::html::base64_encode(svg.as_bytes());
                        map.insert("svg".into(), serde_json::Value::String(svg));
                        map.insert("svg_data".into(), serde_json::Value::String(format!("data:image/svg+xml;base64,{}", b64)));
                    } else {
                        let code = lines.join("\n");
                        let normalized = crate::highlight::normalize_language(lang);
                        let highlighted = crate::highlight::highlight_code(&code, &normalized);
                        let qml_html = crate::html::format_code_for_qml_richtext(&highlighted);
                        map.insert("highlighted_html".into(), serde_json::Value::String(qml_html));
                    }
                }
                map.insert("lines".into(), lines_to_json_array(lines));
            }
            Block::LiteralBlock { title, lines, .. } => {
                insert_title(&mut map, title);
                map.insert("lines".into(), lines_to_json_array(lines));
            }
            Block::Blockquote { title, attribution, citation, children, .. } => {
                insert_title(&mut map, title);
                insert_attribution_citation(&mut map, attribution, citation);
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Verse { title, attribution, citation, lines, spans, .. } => {
                insert_title(&mut map, title);
                insert_attribution_citation(&mut map, attribution, citation);
                map.insert("lines".into(), lines_to_json_array(lines));
                map.insert("lines_spans".into(), serde_json::Value::Array(
                    spans.iter().map(|s| spans_to_json(s)).collect()
                ));
            }
            Block::Sidebar { title, children, .. } => {
                insert_title(&mut map, title);
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Example { title, children, .. } => {
                insert_title(&mut map, title);
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Table { title, rows, col_widths, frame, grid, .. } => {
                insert_title(&mut map, title);
                map.insert("rows".into(), serde_json::Value::Array(
                    rows.iter().map(|row| {
                        serde_json::Value::Array(
                            row.iter().map(|cell| {
                                let mut cell_map = serde_json::Map::new();
                                cell_map.insert("blocks".into(), blocks_to_json(&cell.blocks));
                                if cell.colspan > 1 {
                                    cell_map.insert("colspan".into(), serde_json::Value::Number(serde_json::Number::from(cell.colspan)));
                                }
                                if let Some(ref a) = cell.align {
                                    cell_map.insert("align".into(), serde_json::Value::String(a.clone()));
                                }
                                if let Some(ref v) = cell.valign {
                                    cell_map.insert("valign".into(), serde_json::Value::String(v.clone()));
                                }
                                if let Some(s) = cell.style {
                                    cell_map.insert("style".into(), serde_json::Value::String(s.to_string()));
                                }
                                serde_json::Value::Object(cell_map)
                            }).collect()
                        )
                    }).collect()
                ));
                if !col_widths.is_empty() {
                    map.insert("col_widths".into(), serde_json::Value::Array(
                        col_widths.iter().map(|w| serde_json::Value::Number(serde_json::Number::from_f64(*w).unwrap_or(serde_json::Number::from(1)))).collect()
                    ));
                }
                if let Some(f) = frame {
                    map.insert("frame".into(), serde_json::Value::String(f.clone()));
                }
                if let Some(g) = grid {
                    map.insert("grid".into(), serde_json::Value::String(g.clone()));
                }
            }
            Block::Image { title, target, alt, width, height, .. } => {
                insert_title(&mut map, title);
                map.insert("target".into(), serde_json::Value::String(target.clone()));
                map.insert("alt".into(), serde_json::Value::String(alt.clone()));
                if let Some(w) = width {
                    map.insert("width".into(), serde_json::Value::String(w.clone()));
                }
                if let Some(h) = height {
                    map.insert("height".into(), serde_json::Value::String(h.clone()));
                }
            }
            Block::HorizontalRule { .. } | Block::PageBreak { .. } | Block::EmptyLine => {}
            Block::Toc { depth, .. } => {
                if let Some(d) = depth {
                    map.insert("depth".into(), serde_json::Value::Number((*d).into()));
                }
            }
            Block::Comment { text, .. } => {
                map.insert("text".into(), serde_json::Value::String(text.clone()));
            }
            Block::Admonition { title, kind, children, .. } => {
                insert_title(&mut map, title);
                map.insert("kind".into(), serde_json::Value::String(kind.as_str().into()));
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Open { title, children, .. } => {
                insert_title(&mut map, title);
                map.insert("blocks".into(), blocks_to_json(children));
            }
        }

        serde_json::Value::Object(map)
    }
}

fn spans_to_json(spans: &[InlineSpan]) -> serde_json::Value {
    serde_json::Value::Array(
        spans.iter().map(|s| s.to_json()).collect()
    )
}

fn blocks_to_json(blocks: &[Block]) -> serde_json::Value {
    serde_json::Value::Array(
        blocks.iter().map(|b| b.to_qvariant_map()).collect()
    )
}

fn insert_title(map: &mut serde_json::Map<String, serde_json::Value>, title: &Option<String>) {
    if let Some(t) = title {
        map.insert("title".into(), serde_json::Value::String(t.clone()));
        map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
    }
}

fn insert_attribution_citation(
    map: &mut serde_json::Map<String, serde_json::Value>,
    attribution: &Option<String>,
    citation: &Option<String>,
) {
    if let Some(a) = attribution {
        map.insert("attribution".into(), serde_json::Value::String(a.clone()));
    }
    if let Some(c) = citation {
        map.insert("citation".into(), serde_json::Value::String(c.clone()));
    }
}

fn lines_to_json_array(lines: &[String]) -> serde_json::Value {
    serde_json::Value::Array(
        lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
    )
}

/// Collects all footnotes from a block tree, deduplicating by footnote id.
/// Returns a list of `(Option<id>, text)` pairs.
pub fn collect_footnotes(blocks: &[Block]) -> Vec<(Option<String>, String)> {
    let mut footnotes = Vec::new();
    let mut seen_ids = Vec::new();
    collect_footnotes_inner(blocks, &mut footnotes, &mut seen_ids);
    footnotes
}

fn collect_footnotes_inner(blocks: &[Block], footnotes: &mut Vec<(Option<String>, String)>, seen_ids: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Heading { spans, .. } | Block::Paragraph { spans, .. } => {
                collect_footnotes_from_spans(spans, footnotes, seen_ids);
            }
            Block::OrderedListItem { children, .. } | Block::UnorderedListItem { children, .. } |
            Block::DescriptionListItem { children, .. } | Block::CalloutListItem { children, .. } |
            Block::Blockquote { children, .. } | Block::Admonition { children, .. } |
            Block::Sidebar { children, .. } | Block::Example { children, .. } |
            Block::Open { children, .. } => {
                collect_footnotes_inner(children, footnotes, seen_ids);
            }
            Block::Verse { spans, .. } => {
                for line_spans in spans {
                    collect_footnotes_from_spans(line_spans, footnotes, seen_ids);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        collect_footnotes_inner(&cell.blocks, footnotes, seen_ids);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_footnotes_from_spans(spans: &[InlineSpan], footnotes: &mut Vec<(Option<String>, String)>, seen_ids: &mut Vec<String>) {
    for span in spans {
        match span {
            InlineSpan::Footnote { id, text } => {
                let key = id.clone().unwrap_or_default();
                if key.is_empty() || !seen_ids.contains(&key) {
                    if !key.is_empty() {
                        seen_ids.push(key);
                    }
                    footnotes.push((id.clone(), text.clone()));
                }
            }
            InlineSpan::Bold(inner) | InlineSpan::Italic(inner) | InlineSpan::Monospace(inner) |
            InlineSpan::Superscript(inner) | InlineSpan::Subscript(inner) | InlineSpan::Mark(inner) => {
                collect_footnotes_from_spans(inner, footnotes, seen_ids);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::InlineSpan;

    #[test]
    fn heading_block_type() {
        let b = Block::Heading { level: 1, spans: vec![], raw: "= Title".into() };
        assert_eq!(b.block_type(), "heading");
    }

    #[test]
    fn paragraph_block_type() {
        let b = Block::Paragraph { spans: vec![], raw: "hello".into() };
        assert_eq!(b.block_type(), "paragraph");
    }

    #[test]
    fn empty_line_type() {
        let b = Block::EmptyLine;
        assert_eq!(b.block_type(), "empty_line");
        assert_eq!(b.raw_text(), "");
    }

    #[test]
    fn heading_qvariant_map() {
        let b = Block::Heading {
            level: 2,
            spans: vec![InlineSpan::Text("Hello".into())],
            raw: "== Hello".into(),
        };
        let json = b.to_qvariant_map();
        assert_eq!(json["type"], "heading");
        assert_eq!(json["level"], 2);
        assert_eq!(json["raw"], "== Hello");
        assert!(json["spans"].is_array());
    }

    #[test]
    fn code_block_qvariant_map() {
        let b = Block::CodeBlock {
            title: Some("Sample Code".into()),
            language: Some("rust".into()),
            lines: vec!["fn main() {}".into()],
            raw: ".Sample Code\n----\nfn main() {}\n----".into(),
        };
        let json = b.to_qvariant_map();
        assert_eq!(json["type"], "code_block");
        assert_eq!(json["title"], "Sample Code");
        assert_eq!(json["language"], "rust");
        assert!(json["lines"].is_array());
    }

    #[test]
    fn code_block_highlighted_html_qml_format() {
        let b = Block::CodeBlock {
            title: None,
            language: Some("rust".into()),
            lines: vec!["fn main() {".into(), "    println!(\"hi\");".into(), "}".into()],
            raw: "----\nfn main() {\n    println!(\"hi\");\n}\n----".into(),
        };
        let json = b.to_qvariant_map();
        let html = json["highlighted_html"].as_str().unwrap();
        // Newlines must be <br/> for QML Text.RichText
        assert!(!html.contains('\n'), "should not contain raw newlines: {}", html);
        assert!(html.contains("<br/>"), "should contain <br/> tags: {}", html);
        // Leading 4-space indent must use &nbsp;
        assert!(html.contains("&nbsp;"), "should contain &nbsp; for indentation: {}", html);
    }

    #[test]
    fn svgbob_code_block_qvariant_map() {
        let b = Block::CodeBlock {
            title: Some("Architecture Diagram".into()),
            language: Some("svgbob".into()),
            lines: vec!["+---+".into(), "| A |".into(), "+---+".into()],
            raw: ".Architecture Diagram\n[source,svgbob]\n----\n+---+\n| A |\n+---+\n----".into(),
        };
        let json = b.to_qvariant_map();
        assert_eq!(json["type"], "code_block");
        assert_eq!(json["title"], "Architecture Diagram");
        assert_eq!(json["language"], "svgbob");
        assert!(json["svg"].is_string());
        assert!(json["svg_data"].is_string());
        assert!(json["svg_data"].as_str().unwrap().starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn all_types_serialize() {
        let blocks = vec![
            Block::Heading { level: 1, spans: vec![], raw: "= H".into() },
            Block::Paragraph { spans: vec![], raw: "p".into() },
            Block::OrderedListItem { level: 0, marker: "1.".into(), reversed: false, children: vec![], raw: "1. item".into() },
            Block::UnorderedListItem { level: 0, marker: "-".into(), checked: None, children: vec![], raw: "- item".into() },
            Block::CodeBlock { title: None, language: None, lines: vec![], raw: "----\n----".into() },
            Block::LiteralBlock { title: None, lines: vec![], raw: "....\n....".into() },
            Block::Blockquote { title: None, attribution: None, citation: None, children: vec![], raw: "> quote".into() },
            Block::Verse { title: None, attribution: None, citation: None, lines: vec!["Roses are red".into()], spans: vec![vec![InlineSpan::Text("Roses are red".into())]], raw: "[verse]\nRoses are red".into() },
            Block::Table { title: None, rows: vec![], col_widths: vec![], frame: None, grid: None, raw: "| a |".into() },
            Block::HorizontalRule { raw: "---".into() },
            Block::Admonition { title: None, kind: AdmonitionKind::Warning, children: vec![], raw: "[WARNING]\n====\n====".into() },
            Block::Toc { raw: ":toc:".into(), depth: None },
            Block::EmptyLine,
        ];
        for b in &blocks {
            let json = b.to_qvariant_map();
            assert!(json["type"].is_string(), "type missing for {:?}", b.block_type());
        }
    }

    #[test]
    fn list_item_with_code_block_child() {
        let item = Block::OrderedListItem {
            level: 0,
            marker: "1.".into(),
            reversed: false,
            children: vec![
                Block::Paragraph { spans: vec![InlineSpan::Text("step one".into())], raw: "step one".into() },
                Block::CodeBlock { title: None, language: Some("rust".into()), lines: vec!["fn main() {}".into()], raw: "----\nfn main() {}\n----".into() },
            ],
            raw: "1. step one".into(),
        };
        let json = item.to_qvariant_map();
        assert_eq!(json["type"], "ordered_list_item");
        assert!(json["blocks"].is_array());
        let blocks = json["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "paragraph");
        assert_eq!(blocks[1]["type"], "code_block");
        assert_eq!(blocks[1]["language"], "rust");
    }

    #[test]
    fn table_cell_with_blocks() {
        let table = Block::Table {
            title: None,
            rows: vec![vec![
                TableCell {
                    blocks: vec![
                        Block::Paragraph { spans: vec![InlineSpan::Text("cell text".into())], raw: "cell text".into() },
                        Block::CodeBlock { title: None, language: None, lines: vec!["code".into()], raw: "----\ncode\n----".into() },
                    ],
                    colspan: 1,
                    align: None,
                    valign: None,
                    style: None,
                },
            ]],
            raw: "|===\n| cell text\n|===\n".into(),
            col_widths: vec![],
            frame: None,
            grid: None,
        };
        let json = table.to_qvariant_map();
        let rows = json["rows"].as_array().unwrap();
        let cell = &rows[0][0];
        let blocks = cell["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "paragraph");
        assert_eq!(blocks[1]["type"], "code_block");
    }

    #[test]
    fn admonition_with_children() {
        let adm = Block::Admonition {
            title: None,
            kind: AdmonitionKind::Warning,
            children: vec![
                Block::Paragraph { spans: vec![InlineSpan::Text("be careful".into())], raw: "be careful".into() },
                Block::CodeBlock { title: None, language: None, lines: vec!["test".into()], raw: "----\ntest\n----".into() },
            ],
            raw: "[WARNING]\n====\nbe careful\n----\ntest\n----\n====".into(),
        };
        let json = adm.to_qvariant_map();
        assert_eq!(json["kind"], "WARNING");
        let blocks = json["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn blockquote_with_children() {
        let bq = Block::Blockquote {
            title: None,
            attribution: None,
            citation: None,
            children: vec![
                Block::Paragraph { spans: vec![InlineSpan::Text("line one".into())], raw: "line one".into() },
                Block::Paragraph { spans: vec![InlineSpan::Text("line two".into())], raw: "line two".into() },
            ],
            raw: "> line one\n> line two".into(),
        };
        let json = bq.to_qvariant_map();
        let blocks = json["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 2);
    }
}
