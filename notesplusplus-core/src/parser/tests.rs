use super::*;
use crate::block::{AdmonitionKind, Block};
use crate::inline::InlineSpan;

#[test]
    fn parse_heading_level_1() {
        let blocks = parse_blocks("= Title");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { level, .. } => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_heading_level_3() {
        let blocks = parse_blocks("=== Sub heading");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { level, raw, .. } => {
                assert_eq!(*level, 3);
                assert_eq!(raw, "=== Sub heading");
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parse_paragraph_single_line() {
        let blocks = parse_blocks("Hello world");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Paragraph { raw, .. } => assert_eq!(raw, "Hello world"),
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_paragraph_multi_line() {
        let blocks = parse_blocks("Line one\nLine two\nLine three");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Paragraph { raw, .. } => assert_eq!(raw, "Line one\nLine two\nLine three"),
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parse_empty_lines() {
        let blocks = parse_blocks("hello\n\nworld");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type(), "paragraph");
        assert_eq!(blocks[1].block_type(), "empty_line");
        assert_eq!(blocks[2].block_type(), "paragraph");
    }

    #[test]
    fn parse_horizontal_rule() {
        let blocks = parse_blocks("---");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "horizontal_rule");
    }

    #[test]
    fn parse_horizontal_rule_stars() {
        let blocks = parse_blocks("***");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "horizontal_rule");
    }

    #[test]
    fn parse_code_block() {
        let text = "----\nfn main() {\n    println!(\"hi\");\n}\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { lines, .. } => assert_eq!(lines.len(), 3),
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_code_block_with_language() {
        let text = "----rust\nfn main() {}\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { language, .. } => assert_eq!(language.as_deref(), Some("rust")),
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_svgbob_code_block() {
        let text = "[source,svgbob]\n----\n+---+\n| A |\n+---+\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { language, lines, .. } => {
                assert_eq!(language.as_deref(), Some("svgbob"));
                assert_eq!(lines, &["+---+", "| A |", "+---+"]);
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_literal_block() {
        let text = "....\nliteral text\n....";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "literal_block");
    }

    #[test]
    fn parse_blockquote() {
        let blocks = parse_blocks("> quoted text");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Blockquote { raw, .. } => assert_eq!(raw, "> quoted text"),
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn parse_unordered_list() {
        let blocks = parse_blocks("- item one\n- item two");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type(), "unordered_list_item");
        assert_eq!(blocks[1].block_type(), "unordered_list_item");
    }

    #[test]
    fn parse_ordered_list() {
        let blocks = parse_blocks("1. first\n2. second");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type(), "ordered_list_item");
        assert_eq!(blocks[1].block_type(), "ordered_list_item");
    }

    #[test]
    fn parse_table() {
        let text = "| Name | Value |\n| foo  | bar   |";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_delimited() {
        // Cell-per-line format: blank lines separate rows
        let text = "|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[1].len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_cell_per_line() {
        // Matches the format used in invoice.adoc / technical-doc.adoc
        let text = "|===\n| Description | Amount\n\n| Website Design\n| $2,500.00\n\n| Hosting\n| $120.00\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 3); // header + 2 data rows
                assert_eq!(rows[0].len(), 2); // Description, Amount
                assert_eq!(rows[1].len(), 2); // Website Design, $2,500.00
                assert_eq!(rows[2].len(), 2); // Hosting, $120.00
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_with_cols_attribute() {
        // Real AsciiDoc format: [cols] BEFORE |===
        let text = "[cols=\"1,2\"]\n|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, col_widths, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(col_widths, &[1.0, 2.0]);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_with_frame_and_grid() {
        let text = "[frame=none, grid=rows, cols=\"1,2\"]\n|===\n| Name\n| Value\n\n| foo\n| bar\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, col_widths, frame, grid, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(col_widths, &[1.0, 2.0]);
                assert_eq!(frame.as_deref(), Some("none"));
                assert_eq!(grid.as_deref(), Some("rows"));
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_cols_attribute_not_as_paragraph() {
        // [cols] before |=== must NOT appear as a paragraph
        let text = "== Items\n\n[cols=\"2,1\"]\n|===\n| Name\n| Value\n|===";
        let blocks = parse_blocks(text);
        for b in &blocks {
            if let Block::Paragraph { raw, .. } = b {
                assert!(!raw.contains("[cols"), "cols attribute leaked into paragraph: {}", raw);
            }
        }
    }

    #[test]
    fn parse_admonition_block() {
        let text = "[WARNING]\n====\nBe careful!\n====";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, &AdmonitionKind::Warning);
                assert!(!children.is_empty());
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_admonition_paragraph_with_options() {
        let text = "[NOTE%unbreakable]\nThis note has an unbreakable option.";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, &AdmonitionKind::Note);
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Block::Paragraph { spans, .. } => {
                        assert_eq!(spans[0].plain_text(), "This note has an unbreakable option.");
                    }
                    _ => panic!("expected paragraph child"),
                }
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_admonition_block_with_options() {
        let text = "[NOTE%unbreakable]\n====\nThis delimited note has an unbreakable option.\n====";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, &AdmonitionKind::Note);
                assert!(!children.is_empty());
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_admonition_with_inline_code() {
        let text = "[WARNING]\n====\nAlways run `cargo test` first.\n====";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Admonition { children, .. } => {
                // Children should contain a Paragraph with a Code span
                match &children[0] {
                    Block::Paragraph { spans, .. } => {
                        assert!(spans.iter().any(|s| matches!(s, InlineSpan::Code(_))));
                    }
                    _ => panic!("expected paragraph child"),
                }
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_table_with_inline_formatting() {
        let text = "| *bold* | _italic_ |";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
                // First cell should have a Paragraph with a Bold span
                match &rows[0][0][0] {
                    Block::Paragraph { spans, .. } => {
                        assert!(spans.iter().any(|s| matches!(s, InlineSpan::Bold { .. })));
                    }
                    _ => panic!("expected paragraph in cell"),
                }
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_source_attribute_with_lang() {
        let text = "[source,sql]\n----\nSELECT * FROM users;\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { language, lines, .. } => {
                assert_eq!(language.as_deref(), Some("sql"));
                assert_eq!(lines.len(), 1);
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parse_source_attribute_not_as_paragraph() {
        let text = "[source,sql]\n----\ncode\n----";
        let blocks = parse_blocks(text);
        for b in &blocks {
            if let Block::Paragraph { raw, .. } = b {
                assert!(!raw.contains("[source"), "source attribute leaked into paragraph: {}", raw);
            }
        }
    }

    #[test]
    fn parse_mixed_document() {
        let text = "= Title\n\nSome text\n\n- item 1\n- item 2\n\n----\ncode\n----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 8); // heading, empty, paragraph, empty, list item, list item, empty, code
    }

    #[test]
    fn round_trip_heading() {
        let text = "== Hello World";
        let blocks = parse_blocks(text);
        let result = blocks_to_adoc(&blocks);
        assert_eq!(result, text);
    }

    #[test]
    fn round_trip_paragraph() {
        let text = "Hello world";
        let blocks = parse_blocks(text);
        let result = blocks_to_adoc(&blocks);
        assert_eq!(result, text);
    }

    // === Nested block tests (Step 2 - TDD RED phase) ===

    #[test]
    fn parse_list_item_with_continuation_code_block() {
        let text = "- step one\n\n  ----\n  let x = 1;\n  ----";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::UnorderedListItem { children, .. } => {
                // Should have: paragraph ("step one"), empty_line, code_block
                assert!(children.len() >= 3, "expected 3+ children, got {}: {:?}", children.len(), children.iter().map(|c| c.block_type()).collect::<Vec<_>>());
                assert_eq!(children[0].block_type(), "paragraph");
                assert_eq!(children[1].block_type(), "empty_line");
                assert_eq!(children[2].block_type(), "code_block");
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_blockquote_consecutive_lines() {
        let text = "> line one\n> line two";
        let blocks = parse_blocks(text);
        // Should be a single blockquote (not 2 separate ones)
        assert_eq!(blocks.len(), 1, "expected 1 blockquote, got {}", blocks.len());
        match &blocks[0] {
            Block::Blockquote { children, .. } => {
                // Lines joined as one paragraph (no blank line separator)
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected blockquote"),
        }
    }

    #[test]
    fn parse_admonition_with_multiple_blocks() {
        let text = "[NOTE]\n====\nFirst paragraph.\n\nSecond paragraph.\n\n----\ncode\n----\n====";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Admonition { kind, children, .. } => {
                assert_eq!(kind, &AdmonitionKind::Note);
                assert!(children.len() >= 3, "expected 3+ children, got {}", children.len());
                assert_eq!(children[0].block_type(), "paragraph");
                assert_eq!(children[1].block_type(), "empty_line");
                assert_eq!(children[2].block_type(), "paragraph");
            }
            _ => panic!("expected admonition"),
        }
    }

    #[test]
    fn parse_checkbox_unchecked() {
        let text = "- [ ] buy groceries";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, Some(false));
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_checked() {
        let text = "- [x] done task";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, Some(true));
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_none() {
        let text = "- regular item";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::UnorderedListItem { checked, .. } => {
                assert_eq!(*checked, None);
            }
            _ => panic!("expected unordered list item"),
        }
    }

    #[test]
    fn parse_checkbox_serializes() {
        let text = "- [ ] task\n- [x] done";
        let blocks = parse_blocks(text);
        let b0 = blocks[0].to_qvariant_map();
        assert_eq!(b0["checked"], false);
        let b1 = blocks[1].to_qvariant_map();
        assert_eq!(b1["checked"], true);
    }

    #[test]
    fn parse_three_level_nesting() {
        let text = "1. Planning\n  * Research\n    * Read code\n    * Check docs\n  * Design\n2. Development";
        let blocks = parse_blocks(text);
        // Top level: 2 OL items
        assert_eq!(blocks.len(), 2, "expected 2 top blocks, got {}", blocks.len());
        match &blocks[0] {
            Block::OrderedListItem { children, .. } => {
                // Children should include UL "Research" and UL "Design"
                let uls: Vec<_> = children.iter().filter(|c| c.block_type() == "unordered_list_item").collect();
                assert!(uls.len() >= 2, "expected 2+ UL children, got {}", uls.len());
                // "Research" should be at level 1
                match uls[0] {
                    Block::UnorderedListItem { level, children: research_children, .. } => {
                        assert_eq!(*level, 1, "Research should be level 1, got {}", level);
                        // "Read code" and "Check docs" should be children of Research at level 2
                        let deep_uls: Vec<_> = research_children.iter()
                            .filter(|c| c.block_type() == "unordered_list_item")
                            .collect();
                        assert!(deep_uls.len() >= 2, "expected 2+ deep children, got {}", deep_uls.len());
                        for deep in &deep_uls {
                            if let Block::UnorderedListItem { level, .. } = deep {
                                assert_eq!(*level, 2, "deep item should be level 2, got {}", level);
                            }
                        }
                    }
                    _ => panic!("expected UL"),
                }
                // "Design" should be at level 1
                if let Block::UnorderedListItem { level, .. } = uls[1] {
                    assert_eq!(*level, 1, "Design should be level 1, got {}", level);
                }
            }
            _ => panic!("expected OL"),
        }
    }

    #[test]
    fn parse_nested_list_with_asterisk_marker() {
        let text = "1. first\n  * sub a\n  * sub b\n2. second";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2, "expected 2 top-level blocks, got {}", blocks.len());
        match &blocks[0] {
            Block::OrderedListItem { level, marker, children, .. } => {
                assert_eq!(*level, 0);
                assert_eq!(marker, "1.");
                // Sub-items should be UL at level 1 (parent level 0 + 1)
                let ul_children: Vec<_> = children.iter().filter(|c| c.block_type() == "unordered_list_item").collect();
                assert!(ul_children.len() >= 2, "expected 2+ UL children, got {}", ul_children.len());
                for ul in &ul_children {
                    if let Block::UnorderedListItem { level, .. } = ul {
                        assert_eq!(*level, 1, "sub-item should be level 1");
                    }
                }
            }
            _ => panic!("expected ordered list item"),
        }
    }

    #[test]
    fn parse_ordered_list_enumeration_sequence() {
        let text = "1. Install\n2. Clone\n3. Configure\n  a. Open file\n  b. Set target\n4. Run\n5. Deploy\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5);
        if let Block::OrderedListItem { marker, .. } = &blocks[0] { assert_eq!(marker, "1."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[1] { assert_eq!(marker, "2."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, children, .. } = &blocks[2] {
            assert_eq!(marker, "3.");
            let ol_children: Vec<_> = children.iter().filter(|c| c.block_type() == "ordered_list_item").collect();
            assert_eq!(ol_children.len(), 2);
            if let Block::OrderedListItem { marker, .. } = ol_children[0] { assert_eq!(marker, "a."); }
            if let Block::OrderedListItem { marker, .. } = ol_children[1] { assert_eq!(marker, "b."); }
        } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[3] { assert_eq!(marker, "4."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[4] { assert_eq!(marker, "5."); } else { panic!("expected OL"); }
    }

    #[test]
    fn parse_ordered_list_dot_sequence() {
        let text = ". Step one\n. Step two\n. Step three\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3);
        if let Block::OrderedListItem { marker, .. } = &blocks[0] { assert_eq!(marker, "1."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[1] { assert_eq!(marker, "2."); } else { panic!("expected OL"); }
        if let Block::OrderedListItem { marker, .. } = &blocks[2] { assert_eq!(marker, "3."); } else { panic!("expected OL"); }
    }

    #[test]
    fn parse_table_asciidoc_cell_prefix() {
        let text = "|===\n| simple text\na| - list item 1\n- list item 2\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[0][0].len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
                assert!(rows[0][1].len() >= 2, "expected 2+ blocks in a| cell, got {}", rows[0][1].len());
                assert_eq!(rows[0][1][0].block_type(), "unordered_list_item");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_cols_attribute_asciidoc() {
        let text = "[cols=\"1,2a\"]\n|===\n| header\n| - item 1\n- item 2\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
                assert!(!rows[0][1].is_empty());
                assert_eq!(rows[0][1][0].block_type(), "unordered_list_item");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parse_table_text_cell_no_blocks() {
        let text = "|===\n| *bold* text\n|===";
        let blocks = parse_blocks(text);
        match &blocks[0] {
            Block::Table { rows, .. } => {
                assert_eq!(rows[0][0].len(), 1);
                assert_eq!(rows[0][0][0].block_type(), "paragraph");
            }
            _ => panic!("expected table"),
        }
    }

    // === Panic-proof tests: parse_blocks must never panic on any input ===

    #[test]
    fn parse_blocks_empty_string() {
        let blocks = parse_blocks("");
        assert!(blocks.is_empty());
    }

    #[test]
    fn parse_blocks_only_whitespace() {
        let blocks = parse_blocks("   \n  \n");
        // Should produce empty_line blocks, never panic
        assert!(!blocks.is_empty() || blocks.is_empty()); // just no panic
    }

    #[test]
    fn parse_blocks_attribute_at_eof() {
        // Attribute line at end of file — previously panicked (lines[i] after i += 1)
        let blocks = parse_blocks("[source,sql]");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "paragraph");
    }

    #[test]
    fn parse_blocks_cols_attribute_at_eof() {
        let blocks = parse_blocks("[cols=\"1,2\"]");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_cols_empty_spec() {
        // [cols=] or [cols=","] — empty spec should not panic
        let blocks = parse_blocks("[cols=]\n|===\n| a\n|===");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_cols_just_a_suffix() {
        // [cols="a"] — spec is just "a", num_str would be empty
        let blocks = parse_blocks("[cols=\"a\"]\n|===\n| cell\n|===");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_unclosed_code_block() {
        // Opening ---- without closing ----
        let blocks = parse_blocks("----\nlet x = 1;\nlet y = 2;");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "code_block");
    }

    #[test]
    fn parse_blocks_unclosed_table() {
        // |=== without closing |===
        let blocks = parse_blocks("|===\n| cell 1\n| cell 2");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "table");
    }

    #[test]
    fn parse_blocks_admonition_without_closing() {
        let blocks = parse_blocks("[WARNING]\n====\nBe careful!");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type(), "admonition");
    }

    #[test]
    fn parse_blocks_admonition_missing_closing_does_not_swallow_rest() {
        // [WARNING] with no closing ==== must end at the next heading,
        // leaving the heading and following paragraph in the document.
        let text = "[WARNING]\n====\nBe careful!\n\n== Next section\n\nMore text here.";
        let blocks = parse_blocks(text);
        assert!(blocks[0].block_type() == "admonition");
        assert!(blocks.iter().any(|b| b.block_type() == "heading"), "heading swallowed by admonition");
        assert!(blocks.iter().any(|b| b.block_type() == "paragraph" && b.raw_text().contains("More text")), "content after heading swallowed");
    }

    #[test]
    fn parse_blocks_nested_parse_does_not_panic() {
        // Deeply nested content that exercises recursive parse_blocks calls
        let text = "> > > deeply nested\n> > still nested\n> top level";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_table_with_asciidoc_cells() {
        // Table with a| cells calls parse_blocks recursively
        let text = "|===\na| - item 1\n  - sub item\n- item 2\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_blocks_list_item_with_nested_content() {
        // List item continuation calls parse_blocks recursively
        let text = "- item\n  - sub item\n    - sub sub item";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn parse_doc_attribute_vs_date_paragraph() {
        let text = "= Title\n2024-01-15\n:toc:\n:author: John Doe\n\nParagraph with date: 2024-01-15";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5);
        assert!(matches!(blocks[0], Block::Heading { .. }));
        assert!(matches!(blocks[1], Block::Paragraph { .. }));
        assert!(matches!(blocks[2], Block::Toc { .. }));
        assert!(matches!(blocks[3], Block::EmptyLine));
        assert!(matches!(blocks[4], Block::Paragraph { .. }));
    }

    #[test]
    fn parse_blocks_advanced_demo_does_not_panic() {
        // This is the actual file that caused the crash
        let text = include_str!("../../examples/advanced-demo.adoc");
        let _blocks = parse_blocks(text);
        // Must not panic — that's the only assertion
    }

    #[test]
    fn parse_blocks_fuzz_like_inputs() {
        // Various pathological inputs that must not panic
        let long_eq = "=".repeat(100);
        let long_dash = "- ".repeat(50);
        let inputs = vec![
            "|",
            "|||",
            "a|",
            "[",
            "]",
            "[]",
            "[[]]",
            "= ",
            "=======",
            "----",
            "....",
            "====",
            "> ",
            "- ",
            "* ",
            "1. ",
            "\n\n\n",
            "a\nb\nc\nd\ne\nf\ng\nh\ni\nj",
            &long_eq,
            &long_dash,
            "|===\n|===\n|===",
        ];
        for input in inputs {
            let _blocks = parse_blocks(input);
            // Must not panic
        }
    }

    #[test]
    fn test_toggle_checkbox() {
        let text = "= Title\n\n- [ ] Task 1\n- [x] Task 2\n- [ ] Task 3\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5); // heading, empty, item1, item2, item3

        let toggled1 = toggle_checkbox(text, 2, "").unwrap();
        assert_eq!(toggled1, "= Title\n\n- [x] Task 1\n- [x] Task 2\n- [ ] Task 3\n");

        let toggled2 = toggle_checkbox(text, 3, "").unwrap();
        assert_eq!(toggled2, "= Title\n\n- [ ] Task 1\n- [ ] Task 2\n- [ ] Task 3\n");

        // Non-checkbox block
        assert!(toggle_checkbox(text, 0, "").is_none());
        // Out of bounds block
        assert!(toggle_checkbox(text, 99, "").is_none());
    }

    #[test]
    fn test_toggle_checkbox_nested() {
        let text = "* [ ] Parent 1\n  * [x] Child 1.1\n  * [ ] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);

        // Toggle Child 1.1 (sub_path "1") -> should uncheck Child 1.1
        let t1 = toggle_checkbox(text, 0, "1").unwrap();
        assert_eq!(t1, "* [ ] Parent 1\n  * [ ] Child 1.1\n  * [ ] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n");

        // Toggle Child 1.2 (sub_path "2") -> should check Child 1.2
        let t2 = toggle_checkbox(text, 0, "2").unwrap();
        assert_eq!(t2, "* [ ] Parent 1\n  * [x] Child 1.1\n  * [x] Child 1.2\n    * [ ] Grandchild 1.2.1\n* [x] Parent 2\n");

        // Toggle Grandchild 1.2.1 (sub_path "2.1") -> should check Grandchild 1.2.1
        let t3 = toggle_checkbox(text, 0, "2.1").unwrap();
        assert_eq!(t3, "* [ ] Parent 1\n  * [x] Child 1.1\n  * [ ] Child 1.2\n    * [x] Grandchild 1.2.1\n* [x] Parent 2\n");
    }

    #[test]
    fn test_parse_image_block() {
        let text = "image::screenshots/screen1.png[Main Screen, width=400, height=300]\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Image { target, alt, width, height, .. } = &blocks[0] {
            assert_eq!(target, "screenshots/screen1.png");
            assert_eq!(alt, "Main Screen");
            assert_eq!(width.as_deref(), Some("400"));
            assert_eq!(height.as_deref(), Some("300"));
        } else {
            panic!("expected Block::Image");
        }
    }

    #[test]
    fn test_parse_sidebar_block() {
        let text = ".Tips and Tricks\n****\nThis is inside a sidebar.\n\n* Item A\n* Item B\n****\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Sidebar { title, children, .. } = &blocks[0] {
            assert_eq!(title.as_deref(), Some("Tips and Tricks"));
            assert_eq!(children.len(), 4); // paragraph, empty_line, 2 UL items
        } else {
            panic!("expected Block::Sidebar");
        }
    }

    #[test]
    fn test_parse_example_block() {
        let text = ".Example Result\n====\nSample output here.\n====\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Example { title, children, .. } = &blocks[0] {
            assert_eq!(title.as_deref(), Some("Example Result"));
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected Block::Example");
        }
    }

    #[test]
    fn test_parse_description_list() {
        let text = "CPU:: Central Processing Unit\nRAM:: Random Access Memory\nGPU::\n  Graphics Processing Unit\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3);
        if let Block::DescriptionListItem { term, children, .. } = &blocks[0] {
            assert_eq!(term, "CPU");
            assert_eq!(children.len(), 1);
        } else { panic!("expected DL item 0"); }
        if let Block::DescriptionListItem { term, .. } = &blocks[1] {
            assert_eq!(term, "RAM");
        } else { panic!("expected DL item 1"); }
        if let Block::DescriptionListItem { term, children, .. } = &blocks[2] {
            assert_eq!(term, "GPU");
            assert_eq!(children.len(), 1);
        } else { panic!("expected DL item 2"); }
    }

    #[test]
    fn test_parse_callout_list() {
        let text = "<1> Initialize the system\n<2> Connect to database\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        if let Block::CalloutListItem { number, children, .. } = &blocks[0] {
            assert_eq!(*number, 1);
            assert_eq!(children.len(), 1);
        } else { panic!("expected Callout 1"); }
        if let Block::CalloutListItem { number, children, .. } = &blocks[1] {
            assert_eq!(*number, 2);
            assert_eq!(children.len(), 1);
        } else { panic!("expected Callout 2"); }
    }

    #[test]
    fn test_parse_anonymous_sidebar_and_example() {
        let text = "****\nAnonymous sidebar\n****\n\n====\nAnonymous example\n====\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3); // sidebar, empty_line, example
        if let Block::Sidebar { title, children, .. } = &blocks[0] {
            assert!(title.is_none());
            assert_eq!(children.len(), 1);
        } else { panic!("expected anonymous sidebar"); }
        if let Block::Example { title, children, .. } = &blocks[2] {
            assert!(title.is_none());
            assert_eq!(children.len(), 1);
        } else { panic!("expected anonymous example"); }
    }

    #[test]
    fn test_description_list_formatted_term() {
        let text = "`CONFIG_PATH`:: Path to config file\n*Verbose Mode*:: Enable logging\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        if let Block::DescriptionListItem { term_spans, .. } = &blocks[0] {
            assert_eq!(term_spans, &vec![InlineSpan::Code("CONFIG_PATH".into())]);
        } else { panic!("expected DL 0"); }
        if let Block::DescriptionListItem { term_spans, .. } = &blocks[1] {
            assert_eq!(term_spans, &vec![InlineSpan::Bold(vec![InlineSpan::Text("Verbose Mode".into())])]);
        } else { panic!("expected DL 1"); }
    }

    #[test]
    fn test_toggle_checkbox_with_extended_blocks() {
        let text = "= Doc Title\n\nimage::pic.png[Photo]\n\n****\nSidebar text\n****\n\n- [ ] Target item\n\nTerm:: Description\n";
        let blocks = parse_blocks(text);
        // blocks:
        // 0: heading
        // 1: empty_line
        // 2: image
        // 3: empty_line
        // 4: sidebar
        // 5: empty_line
        // 6: unordered_list_item (- [ ] Target item)
        // 7: empty_line
        // 8: description_list_item
        assert_eq!(blocks.len(), 9);
        assert_eq!(blocks[6].block_type(), "unordered_list_item");

        let toggled = toggle_checkbox(text, 6, "").expect("should toggle Target item at index 6");
        assert!(toggled.contains("- [x] Target item"));
    }

    #[test]
    fn test_chronicles_features_parsing() {
        let text = "// Settings:\n:description: A chronicle of adventures \\\n  across the realms.\n:wolpertinger: Wolpertinger\n\n[%notitle]\n[abstract]\n{description}\n\n[#ravages]\n== Section With Anchor\n\nAt ((Antwerp)) (((Conference,Devoxx))) we saw the {wolpertinger}!\n\n--\nHere is some content inside an open block.\n--\n\n<<<\n\n[appendix]\n== Appendix Section\n";
        let blocks = parse_blocks(text);

        // Check that comments and document attributes were preprocessed/filtered
        assert!(!blocks.iter().any(|b| b.raw_text().contains("// Settings:")));

        // Check that {description} was substituted and [abstract] block attribute applied
        let desc_p = blocks.iter().find(|b| b.raw_text().contains("across the realms")).expect("description paragraph");
        assert!(desc_p.raw_text().contains("A chronicle of adventures across the realms."));

        // Check heading
        let h1 = blocks.iter().find(|b| b.block_type() == "heading").expect("heading");
        assert_eq!(h1.raw_text(), "== Section With Anchor");

        // Check open block
        let open_b = blocks.iter().find(|b| b.block_type() == "open").expect("open block");
        if let Block::Open { children, .. } = open_b {
            assert_eq!(children.len(), 1);
            assert!(children[0].raw_text().contains("inside an open block"));
        } else {
            panic!("expected Block::Open");
        }

        // Check page break
        assert!(blocks.iter().any(|b| b.block_type() == "page_break"));
    }

    #[test]
    fn test_conditionals_and_comments() {
        let text = "////\nComment block to ignore\nMore comments\n////\n:my-feature:\n\nifdef::my-feature[]\nFeature is active.\nendif::[]\n\nifndef::nonexistent[]\nFallback content.\nendif::[]\n";
        let blocks = parse_blocks(text);
        assert!(!blocks.iter().any(|b| b.raw_text().contains("Comment block")));
        assert!(blocks.iter().any(|b| b.raw_text().contains("Feature is active.")));
        assert!(blocks.iter().any(|b| b.raw_text().contains("Fallback content.")));
    }

    #[test]
    fn test_configurable_comment_dropping() {
        let text = "// Top comment\n= Doc Title\n\n// Section note\nParagraph text\n";
        let dropped = parse_blocks_with_options(text, true);
        assert!(!dropped.iter().any(|b| b.block_type() == "comment"));

        let preserved = parse_blocks_with_options(text, false);
        assert!(preserved.iter().any(|b| b.block_type() == "comment" && b.raw_text().contains("// Top comment")));
        assert!(preserved.iter().any(|b| b.block_type() == "comment" && b.raw_text().contains("// Section note")));
    }

    #[test]
    fn test_block_titles_on_code_table_image() {
        let text = ".Example Code\n[source,rust]\n----\nfn main() {}\n----\n\n.Data Table\n|===\n| A | B\n|===\n\n.Banner Image\nimage::banner.png[Banner]\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5); // code, empty, table, empty, image

        if let Block::CodeBlock { title, .. } = &blocks[0] {
            assert_eq!(title.as_deref(), Some("Example Code"));
        } else { panic!("expected CodeBlock with title"); }

        if let Block::Table { title, .. } = &blocks[2] {
            assert_eq!(title.as_deref(), Some("Data Table"));
        } else { panic!("expected Table with title"); }

        if let Block::Image { title, .. } = &blocks[4] {
            assert_eq!(title.as_deref(), Some("Banner Image"));
        } else { panic!("expected Image with title"); }
    }

    #[test]
    fn test_table_cell_prefix_math_and_colspan() {
        let text = "|===\n3+^.e| Centered Emphasized Header Across 3 Cols\n\n| Cell 1 | Cell 2 | Cell 3\n|===";
        let blocks = parse_blocks(text);
        if let Block::Table { rows, .. } = &blocks[0] {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 1);
            let cell = &rows[0][0];
            if let Block::Paragraph { spans, .. } = &cell[0] {
                // Style 'e' wraps in italic
                assert!(matches!(&spans[0], InlineSpan::Italic(_)));
            }
        } else { panic!("expected Table"); }
    }

    #[test]
    fn test_ordered_list_deep_dots_not_literal_block() {
        let text = "... Take a picture of the diagram of its leaves\n.... Don't rip out the picture like a troglodyte\n..... Don't do it, I'm watching you\n. Put on your hiking boots\n\n== Subsequent Heading\n\nThis is a normal paragraph.";
        let blocks = parse_blocks(text);

        // Ensure we have list items, heading, and paragraph — not a single unclosed literal block
        assert!(blocks.iter().any(|b| b.block_type() == "ordered_list_item" && b.raw_text().contains("Don't do it")));
        assert!(blocks.iter().any(|b| b.block_type() == "heading" && b.raw_text().contains("Subsequent Heading")));
        assert!(blocks.iter().any(|b| b.block_type() == "paragraph" && b.raw_text().contains("This is a normal paragraph.")));
        assert!(!blocks.iter().any(|b| b.block_type() == "literal_block"));
    }

    #[test]
    fn test_verse_and_quote_blocks_and_paragraphs() {
        let text = "[quote, Mark Tobey, Bark Journeys]\nOn pavements and the bark of trees I have found whole worlds.\n\n[verse, The documentation attorneys]\n____\nNo bark was harmed in the making of this potion.\n____\n\n[verse]\nRoses are red.\nViolets are blue.\n";
        let blocks = parse_blocks(text);

        assert_eq!(blocks.len(), 5); // quote, empty, verse_block, empty, verse_para
        if let Block::Blockquote { attribution, citation, children, .. } = &blocks[0] {
            assert_eq!(attribution.as_deref(), Some("Mark Tobey"));
            assert_eq!(citation.as_deref(), Some("Bark Journeys"));
            assert_eq!(children.len(), 1);
            assert!(children[0].raw_text().contains("On pavements"));
        } else {
            panic!("expected Block::Blockquote");
        }

        if let Block::Verse { attribution, lines, .. } = &blocks[2] {
            assert_eq!(attribution.as_deref(), Some("The documentation attorneys"));
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0], "No bark was harmed in the making of this potion.");
        } else {
            panic!("expected Block::Verse block");
        }

        if let Block::Verse { lines, .. } = &blocks[4] {
            assert_eq!(lines.len(), 2);
            assert_eq!(lines[0], "Roses are red.");
            assert_eq!(lines[1], "Violets are blue.");
        } else {
            panic!("expected Block::Verse paragraph");
        }
    }

    #[test]
    fn test_ordered_list_levels_and_start_offset() {
        let text = ". Locate dusty botany\n.. Sneeze\n... Sneeze some more\n. Find section on Burdockian\n.. Review characteristics\n... Take picture\n.... Don't rip out\n..... Don't do it\n. Put on your hiking boots\n\n[start=10]\n. arabic (10)\n.. loweralpha (a)\n... lowerroman (i)\n... lowerroman (ii)\n";
        let blocks = parse_blocks(text);

        let items: Vec<&Block> = blocks.iter().filter(|b| b.block_type() == "ordered_list_item").collect();
        assert_eq!(items.len(), 13);

        // First list
        if let Block::OrderedListItem { marker, level, .. } = items[0] { assert_eq!(marker, "1."); assert_eq!(*level, 0); }
        if let Block::OrderedListItem { marker, level, .. } = items[1] { assert_eq!(marker, "a."); assert_eq!(*level, 1); }
        if let Block::OrderedListItem { marker, level, .. } = items[2] { assert_eq!(marker, "i."); assert_eq!(*level, 2); }
        if let Block::OrderedListItem { marker, level, .. } = items[3] { assert_eq!(marker, "2."); assert_eq!(*level, 0); }
        if let Block::OrderedListItem { marker, level, .. } = items[4] { assert_eq!(marker, "a."); assert_eq!(*level, 1); }
        if let Block::OrderedListItem { marker, level, .. } = items[5] { assert_eq!(marker, "i."); assert_eq!(*level, 2); }
        if let Block::OrderedListItem { marker, level, .. } = items[6] { assert_eq!(marker, "A."); assert_eq!(*level, 3); }
        if let Block::OrderedListItem { marker, level, .. } = items[7] { assert_eq!(marker, "I."); assert_eq!(*level, 4); }
        if let Block::OrderedListItem { marker, level, .. } = items[8] { assert_eq!(marker, "3."); assert_eq!(*level, 0); }

        // Second list with [start=10]
        if let Block::OrderedListItem { marker, level, .. } = items[9] { assert_eq!(marker, "10."); assert_eq!(*level, 0); }
        if let Block::OrderedListItem { marker, level, .. } = items[10] { assert_eq!(marker, "a."); assert_eq!(*level, 1); }
        if let Block::OrderedListItem { marker, level, .. } = items[11] { assert_eq!(marker, "i."); assert_eq!(*level, 2); }
        if let Block::OrderedListItem { marker, level, .. } = items[12] { assert_eq!(marker, "ii."); assert_eq!(*level, 2); }
    }

    #[test]
    fn test_parse_full_chronicles_example() {
        let chronicles_adoc = include_str!("../../examples/chronicles.adoc");
        let blocks = parse_blocks(chronicles_adoc);

        // Verify that blocks after the deep dots ordered list item are correctly parsed
        assert!(blocks.len() > 20);
        assert!(blocks.iter().any(|b| b.block_type() == "heading" && b.raw_text().contains("Dawn on the Plateau")));
        assert!(blocks.iter().any(|b| b.block_type() == "ordered_list_item" && b.raw_text().contains("Don't do it, I'm watching you")));
        assert!(blocks.iter().any(|b| b.block_type() == "heading" && b.raw_text().contains("Words Seasoned with Power")));
        assert!(blocks.iter().any(|b| b.block_type() == "ordered_list_item" && b.raw_text().contains("Put on your hiking boots")));

        let mark_tobey = blocks.iter().find(|b| {
            if let Block::Blockquote { attribution, .. } = b {
                attribution.as_deref() == Some("Mark Tobey")
            } else {
                false
            }
        });
        assert!(mark_tobey.is_some(), "Mark Tobey quote block must be found in chronicles.adoc!");

        let doc_attorneys = blocks.iter().find(|b| {
            if let Block::Verse { attribution, .. } = b {
                attribution.as_deref() == Some("The documentation attorneys")
            } else {
                false
            }
        });
        assert!(doc_attorneys.is_some(), "The documentation attorneys verse block must be found in chronicles.adoc!");

        let roses = blocks.iter().find(|b| {
            if let Block::Verse { lines, .. } = b {
                lines.iter().any(|l| l.contains("Roses are"))
            } else {
                false
            }
        });
        assert!(roses.is_some(), "Roses are verse paragraph must be found in chronicles.adoc!");
    }

    #[test]
    fn test_unordered_list_repeat_asterisks() {
        let text = "* all the headings\n** syntax highlighted source code\n*** non-syntax highlighted\n* quote block\n** verse block\n*** table\n**** sequential paragraphs\n***** admonition blocks";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 8);

        let levels: Vec<u8> = blocks.iter().map(|b| {
            if let Block::UnorderedListItem { level, .. } = b {
                *level
            } else {
                panic!("expected unordered list item");
            }
        }).collect();

        assert_eq!(levels, vec![0, 1, 2, 0, 1, 2, 3, 4]);
    }
    #[test]
    fn test_hardbreaks_option_and_plus_suffix() {
        let text = "[%hardbreaks]\nSure.\nHave a listing block.\n\nFirst line +\nSecond line with plus\nThird line";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3); // para1, empty, para2

        if let Block::Paragraph { spans, .. } = &blocks[0] {
            assert!(spans.iter().any(|s| matches!(s, InlineSpan::Pass(v) if v == "<br/>")));
            assert!(spans.iter().any(|s| s.plain_text().contains("Sure.")));
            assert!(spans.iter().any(|s| s.plain_text().contains("Have a listing block.")));
        } else {
            panic!("expected hardbreaks paragraph");
        }

        if let Block::Paragraph { spans, .. } = &blocks[2] {
            assert!(spans.iter().any(|s| matches!(s, InlineSpan::Pass(v) if v == "<br/>")));
            assert!(spans.iter().any(|s| s.plain_text().contains("First line")));
            assert!(spans.iter().any(|s| s.plain_text().contains("Second line with plus")));
        } else {
            panic!("expected plus suffix hardbreaks paragraph");
        }
    }

    #[test]
    fn test_reversed_ordered_lists() {
        let text = "[%reversed]\n. Stone Imperial Russian Stout\n. Pliny the Elder\n. Chimay Grande Réserve (Blue)\n. St. Bernardus Abt 12\n. Westvleteren 12 (XII)\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 5);

        let markers: Vec<String> = blocks.iter().map(|b| {
            if let Block::OrderedListItem { marker, reversed, .. } = b {
                assert!(*reversed);
                marker.clone()
            } else {
                panic!("expected ordered list item");
            }
        }).collect();

        assert_eq!(markers, vec!["5.", "4.", "3.", "2.", "1."]);
    }

    #[test]
    fn test_reversed_ordered_list_with_start() {
        let text = "[start=10, %reversed]\n. ten\n. nine\n. eight\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 3);

        let markers: Vec<String> = blocks.iter().map(|b| {
            if let Block::OrderedListItem { marker, reversed, .. } = b {
                assert!(*reversed);
                marker.clone()
            } else {
                panic!("expected ordered list item");
            }
        }).collect();

        assert_eq!(markers, vec!["10.", "9.", "8."]);
    }

    #[test]
    fn test_reversed_ordered_list_with_nested_items() {
        let text = "[%reversed]\n. Top two\n.. sub a\n.. sub b\n. Top one\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 4);

        if let Block::OrderedListItem { marker, level, reversed, .. } = &blocks[0] {
            assert_eq!(marker, "2.");
            assert_eq!(*level, 0);
            assert!(*reversed);
        }
        if let Block::OrderedListItem { marker, level, reversed, .. } = &blocks[1] {
            assert_eq!(marker, "a.");
            assert_eq!(*level, 1);
            assert!(!*reversed);
        }
        if let Block::OrderedListItem { marker, level, reversed, .. } = &blocks[2] {
            assert_eq!(marker, "b.");
            assert_eq!(*level, 1);
            assert!(!*reversed);
        }
        if let Block::OrderedListItem { marker, level, reversed, .. } = &blocks[3] {
            assert_eq!(marker, "1.");
            assert_eq!(*level, 0);
            assert!(*reversed);
        }
    }

    #[test]
    fn test_table_rows_without_empty_lines() {
        let text = "|===\n| Header 1 | Header 2\n| Row 1 Col 1 | Row 1 Col 2\n| Row 2 Col 1 | Row 2 Col 2\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { rows, .. } = &blocks[0] {
            assert_eq!(rows.len(), 3, "Expected 3 rows in table without empty lines");
            assert_eq!(rows[0].len(), 2);
            assert_eq!(rows[1].len(), 2);
            assert_eq!(rows[2].len(), 2);
            assert_eq!(rows[0][0][0].raw_text(), "Header 1");
            assert_eq!(rows[0][1][0].raw_text(), "Header 2");
            assert_eq!(rows[1][0][0].raw_text(), "Row 1 Col 1");
            assert_eq!(rows[1][1][0].raw_text(), "Row 1 Col 2");
            assert_eq!(rows[2][0][0].raw_text(), "Row 2 Col 1");
            assert_eq!(rows[2][1][0].raw_text(), "Row 2 Col 2");
        } else {
            panic!("expected Block::Table");
        }
    }

    #[test]
    fn test_table_cell_per_line_with_cols_without_empty_lines() {
        let text = "[cols=\"2\"]\n|===\n| Header 1\n| Header 2\n| Row 1 Col 1\n| Row 1 Col 2\n| Row 2 Col 1\n| Row 2 Col 2\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { rows, col_widths, .. } = &blocks[0] {
            assert_eq!(col_widths.len(), 2);
            assert_eq!(rows.len(), 3, "Expected 3 rows in cell-per-line table with cols=2");
            assert_eq!(rows[0].len(), 2);
            assert_eq!(rows[1].len(), 2);
            assert_eq!(rows[2].len(), 2);
            assert_eq!(rows[0][0][0].raw_text(), "Header 1");
            assert_eq!(rows[0][1][0].raw_text(), "Header 2");
            assert_eq!(rows[1][0][0].raw_text(), "Row 1 Col 1");
            assert_eq!(rows[1][1][0].raw_text(), "Row 1 Col 2");
        } else {
            panic!("expected Block::Table");
        }
    }

    #[test]
    fn test_table_with_trailing_pipes() {
        let text = "|===\n| H1 | H2 |\n| R1C1 | R1C2 |\n| R2C1 | R2C2 |\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { rows, .. } = &blocks[0] {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0].len(), 2);
            assert_eq!(rows[1].len(), 2);
            assert_eq!(rows[2].len(), 2);
            assert_eq!(rows[0][0][0].raw_text(), "H1");
            assert_eq!(rows[0][1][0].raw_text(), "H2");
        } else {
            panic!("expected Block::Table");
        }
    }

    #[test]
    fn test_table_cols_multiplier() {
        let text = "[cols=\"3*\"]\n|===\n| A | B | C\n| D | E | F\n|===";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { rows, col_widths, .. } = &blocks[0] {
            assert_eq!(col_widths.len(), 3);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 3);
            assert_eq!(rows[1].len(), 3);
            assert_eq!(rows[1][2][0].raw_text(), "F");
        } else {
            panic!("expected Block::Table");
        }
    }

#[test]
fn bench_parse_all_examples() {
    use std::time::Instant;

    let examples_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    if !examples_dir.exists() {
        eprintln!("Examples dir not found at {:?}, skipping benchmark", examples_dir);
        return;
    }

    let mut files: Vec<_> = std::fs::read_dir(&examples_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "adoc"))
        .collect();
    files.sort_by_key(|e| e.file_name());

    let total_start = Instant::now();
    let mut total_lines = 0usize;
    let mut total_blocks = 0usize;

    for entry in &files {
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let lines_count = content.lines().count();
        total_lines += lines_count;

        let start = Instant::now();
        let blocks = parse_blocks(&content);
        let parse_elapsed = start.elapsed();

        let start_qv = Instant::now();
        for block in &blocks {
            let _ = block.to_qvariant_map();
        }
        let qv_elapsed = start_qv.elapsed();

        total_blocks += blocks.len();
        eprintln!(
            "  {:<30} {:>5} lines {:>4} blocks  parse={:>8.2?}  qvariant={:>8.2?}",
            entry.file_name().to_string_lossy(),
            lines_count,
            blocks.len(),
            parse_elapsed,
            qv_elapsed,
        );
    }

    let total_elapsed = total_start.elapsed();
    eprintln!(
        "\n  TOTAL: {} files, {} lines, {} blocks in {:?}",
        files.len(),
        total_lines,
        total_blocks,
        total_elapsed,
    );
}
