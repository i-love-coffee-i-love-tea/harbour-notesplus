use serde::{Deserialize, Serialize};

use crate::inline::InlineSpan;

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
        spans: Vec<InlineSpan>,
        raw: String,
    },
    UnorderedListItem {
        level: u8,
        marker: String,
        spans: Vec<InlineSpan>,
        raw: String,
    },
    CodeBlock {
        language: Option<String>,
        lines: Vec<String>,
        raw: String,
    },
    LiteralBlock {
        lines: Vec<String>,
        raw: String,
    },
    Blockquote {
        spans: Vec<InlineSpan>,
        raw: String,
    },
    Table {
        rows: Vec<Vec<String>>,
        raw: String,
    },
    HorizontalRule {
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
            Block::CodeBlock { .. } => "code_block",
            Block::LiteralBlock { .. } => "literal_block",
            Block::Blockquote { .. } => "blockquote",
            Block::Table { .. } => "table",
            Block::HorizontalRule { .. } => "horizontal_rule",
            Block::EmptyLine => "empty_line",
        }
    }

    pub fn raw_text(&self) -> &str {
        match self {
            Block::Heading { raw, .. } => raw,
            Block::Paragraph { raw, .. } => raw,
            Block::OrderedListItem { raw, .. } => raw,
            Block::UnorderedListItem { raw, .. } => raw,
            Block::CodeBlock { raw, .. } => raw,
            Block::LiteralBlock { raw, .. } => raw,
            Block::Blockquote { raw, .. } => raw,
            Block::Table { raw, .. } => raw,
            Block::HorizontalRule { raw } => raw,
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
            Block::OrderedListItem { level, marker, spans, .. } => {
                map.insert("level".into(), serde_json::Value::Number((*level).into()));
                map.insert("marker".into(), serde_json::Value::String(marker.clone()));
                map.insert("spans".into(), spans_to_json(spans));
            }
            Block::UnorderedListItem { level, marker, spans, .. } => {
                map.insert("level".into(), serde_json::Value::Number((*level).into()));
                map.insert("marker".into(), serde_json::Value::String(marker.clone()));
                map.insert("spans".into(), spans_to_json(spans));
            }
            Block::CodeBlock { language, lines, .. } => {
                if let Some(lang) = language {
                    map.insert("language".into(), serde_json::Value::String(lang.clone()));
                }
                map.insert("lines".into(), serde_json::Value::Array(
                    lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ));
            }
            Block::LiteralBlock { lines, .. } => {
                map.insert("lines".into(), serde_json::Value::Array(
                    lines.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ));
            }
            Block::Blockquote { spans, .. } => {
                map.insert("spans".into(), spans_to_json(spans));
            }
            Block::Table { rows, .. } => {
                map.insert("rows".into(), serde_json::Value::Array(
                    rows.iter().map(|row| {
                        serde_json::Value::Array(
                            row.iter().map(|cell| serde_json::Value::String(cell.clone())).collect()
                        )
                    }).collect()
                ));
            }
            Block::HorizontalRule { .. } | Block::EmptyLine => {}
        }

        serde_json::Value::Object(map)
    }
}

fn spans_to_json(spans: &[InlineSpan]) -> serde_json::Value {
    serde_json::Value::Array(
        spans.iter().map(|s| s.to_json()).collect()
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
            language: Some("rust".into()),
            lines: vec!["fn main() {}".into()],
            raw: "----\nfn main() {}\n----".into(),
        };
        let json = b.to_qvariant_map();
        assert_eq!(json["type"], "code_block");
        assert_eq!(json["language"], "rust");
        assert!(json["lines"].is_array());
    }

    #[test]
    fn all_types_serialize() {
        let blocks = vec![
            Block::Heading { level: 1, spans: vec![], raw: "= H".into() },
            Block::Paragraph { spans: vec![], raw: "p".into() },
            Block::OrderedListItem { level: 0, marker: "1.".into(), spans: vec![], raw: "1. item".into() },
            Block::UnorderedListItem { level: 0, marker: "-".into(), spans: vec![], raw: "- item".into() },
            Block::CodeBlock { language: None, lines: vec![], raw: "----\n----".into() },
            Block::LiteralBlock { lines: vec![], raw: "....\n....".into() },
            Block::Blockquote { spans: vec![], raw: "> quote".into() },
            Block::Table { rows: vec![], raw: "| a |".into() },
            Block::HorizontalRule { raw: "---".into() },
            Block::EmptyLine,
        ];
        for b in &blocks {
            let json = b.to_qvariant_map();
            assert!(json["type"].is_string(), "type missing for {:?}", b.block_type());
        }
    }
}
