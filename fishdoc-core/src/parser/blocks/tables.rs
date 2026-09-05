use crate::block::Block;
use crate::inline::{parse_inline, InlineSpan};
use crate::parser::attributes::{is_attribute_line, parse_table_attributes};
use crate::parser::parse_blocks;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CellSpec {
    pub colspan: usize,
    pub align: Option<char>, // '<', '^', '>'
    pub style: Option<char>, // 'a', 'e', 's', 'm', 'h', 'l', 'v', 'd'
}

pub fn parse_cell_spec(s: &str) -> CellSpec {
    let mut spec = CellSpec {
        colspan: 1,
        align: None,
        style: None,
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return spec;
    }

    // Check colspan: e.g. "3+" or "2+^.e"
    if let Some(plus_idx) = trimmed.find('+') {
        let num_str = &trimmed[..plus_idx];
        if let Ok(n) = num_str.parse::<usize>() {
            if n > 0 {
                spec.colspan = n;
            }
        }
    }

    // Check align: '^', '>', '<'
    if trimmed.contains('^') {
        spec.align = Some('^');
    } else if trimmed.contains('>') {
        spec.align = Some('>');
    } else if trimmed.contains('<') {
        spec.align = Some('<');
    }

    // Check style: after '.' or single style letter
    if let Some(dot_idx) = trimmed.rfind('.') {
        let style_part = &trimmed[dot_idx + 1..];
        if let Some(first_char) = style_part.chars().next() {
            spec.style = Some(first_char.to_ascii_lowercase());
        }
    } else {
        for c in trimmed.chars() {
            if matches!(
                c,
                'a' | 'e'
                    | 'i'
                    | 's'
                    | 'b'
                    | 'm'
                    | 'c'
                    | 'h'
                    | 'l'
                    | 'v'
                    | 'd'
                    | 'A'
                    | 'E'
                    | 'I'
                    | 'S'
                    | 'B'
                    | 'M'
                    | 'C'
                    | 'H'
                    | 'L'
                    | 'V'
                    | 'D'
            ) {
                spec.style = Some(c.to_ascii_lowercase());
                break;
            }
        }
    }

    spec
}

pub fn is_valid_cell_prefix(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.len() > 12 {
        return false;
    }
    trimmed.chars().all(|c| {
        c.is_ascii_digit()
            || c.is_ascii_alphabetic()
            || matches!(c, '+' | '*' | '<' | '^' | '>' | '.' | '_')
    })
}

pub fn parse_cells_from_line(line: &str) -> Option<Vec<(CellSpec, String)>> {
    let trimmed_start = line.trim_start();
    if trimmed_start.starts_with("|===") {
        return None;
    }
    let first_pipe = trimmed_start.find('|')?;
    let prefix = &trimmed_start[..first_pipe];
    if !is_valid_cell_prefix(prefix) {
        return None;
    }

    let mut result = Vec::new();
    let mut current_spec = parse_cell_spec(prefix);
    let mut current_text = String::new();

    let s = &trimmed_start[first_pipe + 1..];
    let mut chars = s.char_indices().peekable();
    let mut last_idx = 0;

    while let Some((idx, c)) = chars.next() {
        if c == '\\' {
            chars.next();
            continue;
        }
        if c == '|' {
            let segment = &s[last_idx..idx];
            let (content, spec_str) = if let Some(last_ws) = segment.rfind(char::is_whitespace) {
                let pot_spec = segment[last_ws..].trim();
                if is_valid_cell_prefix(pot_spec) {
                    (segment[..last_ws].trim(), pot_spec)
                } else {
                    (segment.trim(), "")
                }
            } else if is_valid_cell_prefix(segment.trim()) {
                ("", segment.trim())
            } else {
                (segment.trim(), "")
            };

            current_text.push_str(content);
            result.push((current_spec, current_text.trim().to_string()));
            current_spec = parse_cell_spec(spec_str);
            current_text = String::new();
            last_idx = idx + 1;
        }
    }

    let remaining = s[last_idx..].trim();
    current_text.push_str(remaining);
    result.push((current_spec, current_text.trim().to_string()));

    Some(result)
}

pub fn parse_table(lines: &[&str], title: Option<String>) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    // Consume optional attribute lines before |===
    let mut col_asciidoc: Vec<bool> = Vec::new();
    let mut col_widths: Vec<f64> = Vec::new();
    let mut frame: Option<String> = None;
    let mut grid: Option<String> = None;
    while consumed < lines.len() && is_attribute_line(lines[consumed].trim()) {
        let attr = lines[consumed].trim();
        raw_parts.push(lines[consumed].to_string());
        parse_table_attributes(attr, &mut col_widths, &mut col_asciidoc, &mut frame, &mut grid);
        consumed += 1;
    }

    // Check for |=== delimited table
    if consumed < lines.len() && lines[consumed].trim() == "|===" {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;

        let mut current_row_cells: Vec<(Vec<String>, CellSpec)> = Vec::new();
        let mut current_cell: Vec<String> = Vec::new();
        let mut current_spec = CellSpec::default();

        let flush_row = |row_cells: &mut Vec<(Vec<String>, CellSpec)>,
                         rows: &mut Vec<Vec<Vec<Block>>>,
                         col_asciidoc: &[bool]| {
            if !row_cells.is_empty() {
                let parsed: Vec<Vec<Block>> = row_cells
                    .iter()
                    .enumerate()
                    .map(|(ci, (cell_lines, spec))| {
                        let content = cell_lines.join("\n");
                        let use_ad = if spec.style == Some('a') {
                            true
                        } else {
                            col_asciidoc.get(ci).copied().unwrap_or(false)
                        };
                        if use_ad {
                            parse_blocks(&content)
                        } else {
                            let mut spans = parse_inline(&content);
                            if spec.style == Some('e') || spec.style == Some('i') {
                                spans = vec![InlineSpan::Italic(spans)];
                            } else if spec.style == Some('s') || spec.style == Some('b') {
                                spans = vec![InlineSpan::Bold(spans)];
                            } else if spec.style == Some('m') || spec.style == Some('c') {
                                spans = vec![InlineSpan::Code(content.clone())];
                            }
                            if spans.is_empty() {
                                vec![]
                            } else {
                                vec![Block::Paragraph {
                                    spans,
                                    raw: content,
                                }]
                            }
                        }
                    })
                    .collect();
                rows.push(parsed);
                row_cells.clear();
            }
        };

        while consumed < lines.len() {
            let line = lines[consumed];
            raw_parts.push(line.to_string());

            if line.trim() == "|===" {
                consumed += 1;
                if !current_cell.is_empty() {
                    current_row_cells.push((current_cell, current_spec));
                    current_cell = Vec::new();
                }
                flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
                break;
            }

            if line.trim().is_empty() {
                if !current_cell.is_empty() {
                    current_row_cells.push((current_cell, current_spec));
                    current_cell = Vec::new();
                }
                flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
                current_spec = CellSpec::default();
                consumed += 1;
                continue;
            }

            if let Some(cells) = parse_cells_from_line(line) {
                for (idx, (spec, text)) in cells.into_iter().enumerate() {
                    if idx == 0 {
                        if !current_cell.is_empty() {
                            current_row_cells.push((current_cell, current_spec));
                            current_cell = Vec::new();
                        }
                        current_spec = spec;
                        if !text.is_empty() {
                            current_cell.push(text);
                        }
                    } else {
                        if !current_cell.is_empty() || current_spec.colspan > 0 {
                            current_row_cells.push((current_cell, current_spec));
                            current_cell = Vec::new();
                        }
                        current_spec = spec;
                        if !text.is_empty() {
                            current_cell.push(text);
                        }
                    }
                }
            } else {
                current_cell.push(line.to_string());
            }
            consumed += 1;
        }

        if !current_cell.is_empty() {
            current_row_cells.push((current_cell, current_spec));
        }
        flush_row(&mut current_row_cells, &mut rows, &col_asciidoc);
    } else {
        // Simple pipe-delimited table (no |=== delimiters)
        while consumed < lines.len() {
            let line = lines[consumed];
            if !line.trim_start().starts_with('|') {
                break;
            }
            raw_parts.push(line.to_string());
            rows.push(parse_table_row(line));
            consumed += 1;
        }
    }

    (
        Block::Table {
            title,
            rows,
            col_widths,
            frame,
            grid,
            raw: raw_parts.join("\n"),
        },
        consumed,
    )
}

pub fn parse_table_row(line: &str) -> Vec<Vec<Block>> {
    let mut cells: Vec<Vec<Block>> = line
        .split('|')
        .skip(1)
        .map(|s| {
            let text = s.trim();
            let spans = parse_inline(text);
            if spans.is_empty() {
                vec![]
            } else {
                vec![Block::Paragraph {
                    spans,
                    raw: text.to_string(),
                }]
            }
        })
        .collect();
    // Remove trailing empty cell from trailing pipe
    while cells.last().map_or(false, |c| c.is_empty()) {
        cells.pop();
    }
    cells
}
