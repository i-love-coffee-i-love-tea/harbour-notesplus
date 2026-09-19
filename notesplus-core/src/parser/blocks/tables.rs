use crate::block::{Block, TableCell};
use crate::inline::{parse_inline, InlineSpan};
use crate::parser::attributes::{is_attribute_line, parse_table_attributes, ColSpec};
use crate::parser::parse_blocks;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CellSpec {
    pub colspan: usize,
    pub align: Option<char>, // '<', '^', '>'
    pub valign: Option<char>, // '<', '^', '>'
    pub style: Option<char>, // 'a', 'e', 's', 'm', 'h', 'l', 'v', 'd'
}

pub fn parse_cell_spec(s: &str) -> CellSpec {
    let mut spec = CellSpec {
        colspan: 1,
        align: None,
        valign: None,
        style: None,
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return spec;
    }

    let mut rest = trimmed;
    // Check colspan: e.g. "3+" or "2+^.e"
    if let Some(plus_idx) = rest.find('+') {
        let num_str = &rest[..plus_idx];
        if let Ok(n) = num_str.parse::<usize>() {
            if n > 0 {
                spec.colspan = n;
            }
        }
        rest = &rest[plus_idx + 1..];
    }

    let (h_part, v_part) = if let Some(dot_idx) = rest.find('.') {
        let (h, v) = rest.split_at(dot_idx);
        (h, &v[1..])
    } else {
        (rest, "")
    };

    for c in h_part.chars() {
        if c == '^' {
            spec.align = Some('^');
        } else if c == '>' {
            spec.align = Some('>');
        } else if c == '<' {
            spec.align = Some('<');
        } else if matches!(c, 'a' | 'e' | 'i' | 's' | 'b' | 'm' | 'c' | 'h' | 'l' | 'v' | 'd' | 'A' | 'E' | 'I' | 'S' | 'B' | 'M' | 'C' | 'H' | 'L' | 'V' | 'D') {
            spec.style = Some(c.to_ascii_lowercase());
        }
    }

    for c in v_part.chars() {
        if c == '^' {
            spec.valign = Some('^');
        } else if c == '>' {
            spec.valign = Some('>');
        } else if c == '<' {
            spec.valign = Some('<');
        } else if matches!(c, 'a' | 'e' | 'i' | 's' | 'b' | 'm' | 'c' | 'h' | 'l' | 'v' | 'd' | 'A' | 'E' | 'I' | 'S' | 'B' | 'M' | 'C' | 'H' | 'L' | 'V' | 'D') {
            spec.style = Some(c.to_ascii_lowercase());
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
    let mut in_backtick = false;

    while let Some((idx, c)) = chars.next() {
        if c == '\\' {
            chars.next();
            continue;
        }
        if c == '`' {
            in_backtick = !in_backtick;
        }
        if c == '|' && !in_backtick {
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
    if !remaining.is_empty() || result.is_empty() {
        current_text.push_str(remaining);
        result.push((current_spec, current_text.trim().to_string()));
    }

    Some(result)
}

pub fn parse_table(lines: &[&str], title: Option<String>) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut consumed = 0;
    let mut raw_parts = Vec::new();

    // Consume optional attribute lines before |===
    let mut col_asciidoc: Vec<bool> = Vec::new();
    let mut col_widths: Vec<f64> = Vec::new();
    let mut col_specs: Vec<ColSpec> = Vec::new();
    let mut frame: Option<String> = None;
    let mut grid: Option<String> = None;
    while consumed < lines.len() && is_attribute_line(lines[consumed].trim()) {
        let attr = lines[consumed].trim();
        raw_parts.push(lines[consumed].to_string());
        parse_table_attributes(attr, &mut col_widths, &mut col_asciidoc, &mut col_specs, &mut frame, &mut grid);
        consumed += 1;
    }

    // Check for |=== delimited table
    if consumed < lines.len() && lines[consumed].trim() == "|===" {
        raw_parts.push(lines[consumed].to_string());
        consumed += 1;

        let mut num_cols: Option<usize> = if !col_widths.is_empty() {
            Some(col_widths.len())
        } else {
            None
        };

        let mut current_row_cells: Vec<(Vec<String>, CellSpec)> = Vec::new();
        let mut current_row_col_count: usize = 0;
        let mut open_cell_lines: Vec<String> = Vec::new();
        let mut open_cell_spec = CellSpec::default();
        let mut has_open_cell = false;

        let flush_row = |row_cells: &mut Vec<(Vec<String>, CellSpec)>,
                         rows: &mut Vec<Vec<TableCell>>,
                         col_specs: &[ColSpec]| {
            if !row_cells.is_empty() {
                let mut col_idx = 0;
                let parsed: Vec<TableCell> = row_cells
                    .drain(..)
                    .map(|(cell_lines, spec)| {
                        let content = cell_lines.join("\n");
                        let col_spec = col_specs.get(col_idx);
                        let effective_style = spec.style.or_else(|| col_spec.and_then(|c| c.style));
                        let use_ad = if effective_style == Some('a') {
                            true
                        } else {
                            col_spec.map(|c| c.is_asciidoc).unwrap_or(false)
                        };

                        let align_opt = spec.align.map(|c| match c {
                            '^' => "center",
                            '>' => "right",
                            _ => "left",
                        }.to_string()).or_else(|| col_spec.and_then(|c| c.align.clone()));

                        let valign_opt = spec.valign.map(|c| match c {
                            '^' => "middle",
                            '>' => "bottom",
                            _ => "top",
                        }.to_string()).or_else(|| col_spec.and_then(|c| c.valign.clone()));

                        col_idx += spec.colspan.max(1);
                        let blocks = if use_ad {
                            parse_blocks(&content)
                        } else {
                            let mut spans = parse_inline(&content);
                            if effective_style == Some('e') || effective_style == Some('i') {
                                spans = vec![InlineSpan::Italic(spans)];
                            } else if effective_style == Some('s') || effective_style == Some('b') {
                                spans = vec![InlineSpan::Bold(spans)];
                            } else if effective_style == Some('m') || effective_style == Some('c') {
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
                        };

                        TableCell {
                            blocks,
                            colspan: spec.colspan.max(1),
                            align: align_opt,
                            valign: valign_opt,
                            style: effective_style,
                        }
                    })
                    .collect();
                rows.push(parsed);
            }
        };

        let commit_open_cell = |has_open: &mut bool,
                                open_lines: &mut Vec<String>,
                                open_spec: CellSpec,
                                row_cells: &mut Vec<(Vec<String>, CellSpec)>,
                                row_col_count: &mut usize| {
            if *has_open {
                let span = open_spec.colspan.max(1);
                row_cells.push((std::mem::take(open_lines), open_spec));
                *row_col_count += span;
                *has_open = false;
            }
        };

        while consumed < lines.len() {
            let line = lines[consumed];
            raw_parts.push(line.to_string());

            if line.trim() == "|===" {
                consumed += 1;
                commit_open_cell(
                    &mut has_open_cell,
                    &mut open_cell_lines,
                    open_cell_spec,
                    &mut current_row_cells,
                    &mut current_row_col_count,
                );
                flush_row(&mut current_row_cells, &mut rows, &col_specs);
                break;
            }

            if line.trim().is_empty() {
                // Check if the current open cell is an AsciiDoc cell (a| or column-level a)
                let cell_col_idx = current_row_col_count;
                let is_ad_cell = has_open_cell && (
                    open_cell_spec.style == Some('a')
                    || col_asciidoc.get(cell_col_idx).copied().unwrap_or(false)
                );
                // Check if the NEXT column is an AsciiDoc column (blank line between
                // a regular cell and an upcoming a| cell is just spacing)
                let next_is_ad = col_asciidoc.get(current_row_col_count).copied().unwrap_or(false);
                if is_ad_cell {
                    // Blank line inside an AsciiDoc cell — treat as cell content
                    open_cell_lines.push(String::new());
                    consumed += 1;
                    continue;
                }
                if next_is_ad && !current_row_cells.is_empty() {
                    // Blank line between a committed cell and an upcoming a| cell — skip
                    consumed += 1;
                    continue;
                }
                commit_open_cell(
                    &mut has_open_cell,
                    &mut open_cell_lines,
                    open_cell_spec,
                    &mut current_row_cells,
                    &mut current_row_col_count,
                );
                if !current_row_cells.is_empty() {
                    if num_cols.is_none() {
                        num_cols = Some(current_row_col_count);
                    }
                    flush_row(&mut current_row_cells, &mut rows, &col_specs);
                    current_row_col_count = 0;
                }
                consumed += 1;
                continue;
            }

            // Handle |a|... as a single cell with AsciiDoc style prefix.
            // parse_cells_from_line splits on all |, so |a|* item becomes two cells.
            // Detect this pattern and treat as one cell with style 'a'.
            if line.trim_start().starts_with("|a|") || line.trim_start().starts_with("|A|") {
                let content = &line.trim_start()[3..]; // skip "|a|"
                let mut spec = CellSpec::default();
                spec.style = Some('a');
                commit_open_cell(
                    &mut has_open_cell,
                    &mut open_cell_lines,
                    open_cell_spec,
                    &mut current_row_cells,
                    &mut current_row_col_count,
                );
                if let Some(max_cols) = num_cols {
                    if current_row_col_count >= max_cols {
                        flush_row(&mut current_row_cells, &mut rows, &col_specs);
                        current_row_col_count = 0;
                    }
                }
                open_cell_spec = spec;
                has_open_cell = true;
                if !content.is_empty() {
                    open_cell_lines.push(content.to_string());
                }
                consumed += 1;
                continue;
            }

            if let Some(cells) = parse_cells_from_line(line) {
                let line_span_sum: usize = cells.iter().map(|(spec, _)| spec.colspan.max(1)).sum();
                if num_cols.is_none() && (cells.len() > 1 || line_span_sum > 1) {
                    num_cols = Some(line_span_sum);
                }

                for (spec, text) in cells {
                    commit_open_cell(
                        &mut has_open_cell,
                        &mut open_cell_lines,
                        open_cell_spec,
                        &mut current_row_cells,
                        &mut current_row_col_count,
                    );

                    if let Some(max_cols) = num_cols {
                        if current_row_col_count >= max_cols {
                            flush_row(&mut current_row_cells, &mut rows, &col_specs);
                            current_row_col_count = 0;
                        }
                    }

                    open_cell_spec = spec;
                    has_open_cell = true;
                    if !text.is_empty() {
                        open_cell_lines.push(text);
                    }
                }
            } else {
                if has_open_cell {
                    open_cell_lines.push(line.to_string());
                } else {
                    has_open_cell = true;
                    open_cell_spec = CellSpec::default();
                    open_cell_lines.push(line.to_string());
                }
            }
            consumed += 1;
        }

        commit_open_cell(
            &mut has_open_cell,
            &mut open_cell_lines,
            open_cell_spec,
            &mut current_row_cells,
            &mut current_row_col_count,
        );
        flush_row(&mut current_row_cells, &mut rows, &col_specs);
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

pub fn parse_table_row(line: &str) -> Vec<TableCell> {
    let mut cells: Vec<TableCell> = line
        .split('|')
        .skip(1)
        .map(|s| {
            let text = s.trim();
            let spans = parse_inline(text);
            let blocks = if spans.is_empty() {
                vec![]
            } else {
                vec![Block::Paragraph {
                    spans,
                    raw: text.to_string(),
                }]
            };
            TableCell::new(blocks)
        })
        .collect();
    // Remove trailing empty cell from trailing pipe
    while cells.last().is_some_and(|c| c.blocks.is_empty()) {
        cells.pop();
    }
    cells
}


