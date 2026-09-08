use serde::{Deserialize, Serialize};

use crate::inline::InlineSpan;

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
        kind: String,
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
    },
    EmptyLine,
}

impl Block {
    pub fn block_type(&self) -> &'static str {
        match self {
            Block::Heading { .. } => "heading",
            Block::Paragraph { .. } => "paragraph",
            Block::OrderedListItem { .. } => "ordered_list_item",
            Block::UnorderedListItem { .. } => "unordered_list_item",
            Block::DescriptionListItem { .. } => "description_list_item",
            Block::CalloutListItem { .. } => "callout_list_item",
            Block::CodeBlock { .. } => "code_block",
            Block::LiteralBlock { .. } => "literal_block",
            Block::Blockquote { .. } => "blockquote",
            Block::Verse { .. } => "verse",
            Block::Sidebar { .. } => "sidebar",
            Block::Example { .. } => "example",
            Block::Table { .. } => "table",
            Block::Image { .. } => "image",
            Block::HorizontalRule { .. } => "horizontal_rule",
            Block::Admonition { .. } => "admonition",
            Block::Open { .. } => "open",
            Block::PageBreak { .. } => "page_break",
            Block::Comment { .. } => "comment",
            Block::Toc { .. } => "toc",
            Block::EmptyLine => "empty_line",
        }
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
            Block::Toc { raw } => raw,
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
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                if let Some(lang) = language {
                    map.insert("language".into(), serde_json::Value::String(lang.clone()));
                }
                map.insert("lines".into(), serde_json::Value::Array(
                    lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ));
            }
            Block::LiteralBlock { title, lines, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                map.insert("lines".into(), serde_json::Value::Array(
                    lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ));
            }
            Block::Blockquote { title, attribution, citation, children, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                if let Some(a) = attribution {
                    map.insert("attribution".into(), serde_json::Value::String(a.clone()));
                }
                if let Some(c) = citation {
                    map.insert("citation".into(), serde_json::Value::String(c.clone()));
                }
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Verse { title, attribution, citation, lines, spans, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                if let Some(a) = attribution {
                    map.insert("attribution".into(), serde_json::Value::String(a.clone()));
                }
                if let Some(c) = citation {
                    map.insert("citation".into(), serde_json::Value::String(c.clone()));
                }
                map.insert("lines".into(), serde_json::Value::Array(
                    lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ));
                map.insert("lines_spans".into(), serde_json::Value::Array(
                    spans.iter().map(|s| spans_to_json(s)).collect()
                ));
            }
            Block::Sidebar { title, children, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Example { title, children, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Table { title, rows, col_widths, frame, grid, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
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
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                map.insert("target".into(), serde_json::Value::String(target.clone()));
                map.insert("alt".into(), serde_json::Value::String(alt.clone()));
                if let Some(w) = width {
                    map.insert("width".into(), serde_json::Value::String(w.clone()));
                }
                if let Some(h) = height {
                    map.insert("height".into(), serde_json::Value::String(h.clone()));
                }
            }
            Block::HorizontalRule { .. } | Block::Toc { .. } | Block::PageBreak { .. } | Block::EmptyLine => {}
            Block::Comment { text, .. } => {
                map.insert("text".into(), serde_json::Value::String(text.clone()));
            }
            Block::Admonition { title, kind, children, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
                map.insert("kind".into(), serde_json::Value::String(kind.clone()));
                map.insert("blocks".into(), blocks_to_json(children));
            }
            Block::Open { title, children, .. } => {
                if let Some(t) = title {
                    map.insert("title".into(), serde_json::Value::String(t.clone()));
                    map.insert("title_spans".into(), spans_to_json(&crate::inline::parse_inline(t)));
                }
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
            Block::Admonition { title: None, kind: "WARNING".into(), children: vec![], raw: "[WARNING]\n====\n====".into() },
            Block::Toc { raw: ":toc:".into() },
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
            kind: "WARNING".into(),
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
