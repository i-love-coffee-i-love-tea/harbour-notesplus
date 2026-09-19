use notesplus_core::page::{extract_doc_title, sanitize_note_filename, sanitize_note_path};
use notesplus_core::parser::{blocks_to_adoc, parse_blocks, parse_blocks_with_options, toggle_checkbox};

#[test]
fn test_sanitize_note_path_traversal_and_normalization() {
    let cases = vec![
        ("simple.adoc", "", "simple.adoc"),
        ("simple", "", "simple.adoc"),
        ("work/project.adoc", "work", "project.adoc"),
        ("work\\project.adoc", "work", "project.adoc"),
        ("../../../etc/passwd", "etc", "passwd.adoc"),
        ("a/b/c/d.adoc", "a/b/c", "d.adoc"),
        ("//a//b//c.adoc", "a/b", "c.adoc"),
        ("   spaced / note name   ", "spaced", "note_name.adoc"),
        ("./current/./dir.adoc", "current", "dir.adoc"),
        ("special:chars?*\"<>|.adoc", "", "special_chars.adoc"),
    ];

    for (input, expected_group, expected_file) in cases {
        let (group, file) = sanitize_note_path(input);
        assert_eq!(group, expected_group, "Group mismatch for input: {}", input);
        assert_eq!(file, expected_file, "Filename mismatch for input: {}", input);
        assert!(!file.contains('/'), "Filename should not contain slash: {}", file);
        assert!(!file.contains('\\'), "Filename should not contain backslash: {}", file);
        assert!(file.ends_with(".adoc"), "Filename must end with .adoc: {}", file);
    }
}

#[test]
fn test_sanitize_note_filename_edge_cases() {
    assert_eq!(sanitize_note_filename("normal.adoc"), "normal.adoc");
    assert_eq!(sanitize_note_filename("no_ext"), "no_ext.adoc");
    assert_eq!(sanitize_note_filename(""), "Untitled.adoc");
    assert_eq!(sanitize_note_filename("   "), "Untitled.adoc");
    assert_eq!(sanitize_note_filename("....adoc"), "Untitled.adoc");
    assert_eq!(sanitize_note_filename("path/to/file.adoc"), "path_to_file.adoc");
    assert_eq!(sanitize_note_filename("path\\to\\file.adoc"), "path_to_file.adoc");
}

#[test]
fn test_extract_doc_title_various_formats() {
    assert_eq!(extract_doc_title("= My Heading\nSome text", "fallback.adoc"), "My Heading");
    assert_eq!(extract_doc_title("   = Indented Heading   \nText", "fallback.adoc"), "Indented Heading");
    assert_eq!(extract_doc_title("== Level 2 Heading\nNo level 1", "my_fallback_note.adoc"), "my fallback note");
    assert_eq!(extract_doc_title("= 🚀 Unicode Title with Accents éàü", "fallback.adoc"), "🚀 Unicode Title with Accents éàü");
    assert_eq!(extract_doc_title("", "default_name.adoc"), "default name");
}

#[test]
fn test_toggle_checkbox_deeply_nested_hierarchy() {
    let source = "\
* [ ] Root 1
  * [x] Level 2 A
    * [ ] Level 3 A.1
    * [x] Level 3 A.2
  * [ ] Level 2 B
* [x] Root 2
";

    // Toggle Root 1 (index 0, root path) -> should check Root 1
    let r1 = toggle_checkbox(source, 0, "").expect("Toggle Root 1");
    assert!(r1.starts_with("* [x] Root 1\n  * [x] Level 2 A"));

    // Toggle Level 3 A.1 (index 0, path "1.1") -> should check Level 3 A.1
    let r2 = toggle_checkbox(source, 0, "1.1").expect("Toggle Level 3 A.1");
    assert!(r2.contains("* [x] Level 3 A.1"));

    // Toggle Level 3 A.2 (index 0, path "1.2") -> should uncheck Level 3 A.2
    let r3 = toggle_checkbox(source, 0, "1.2").expect("Toggle Level 3 A.2");
    assert!(r3.contains("* [ ] Level 3 A.2"));

    // Toggle Root 2 (index 1 in top-level list items)
    let r4 = toggle_checkbox(source, 1, "").expect("Toggle Root 2");
    assert!(r4.contains("* [ ] Root 2"));
}

#[test]
fn test_toggle_checkbox_with_dash_and_asterisk_markers() {
    let dash_source = "- [ ] Dash item 1\n- [x] Dash item 2\n";
    let toggled_dash = toggle_checkbox(dash_source, 0, "").expect("Toggle dash 1");
    assert_eq!(toggled_dash, "- [x] Dash item 1\n- [x] Dash item 2\n");

    let asterisk_source = "* [ ] Star item 1\n* [x] Star item 2\n";
    let toggled_star = toggle_checkbox(asterisk_source, 1, "").expect("Toggle star 2");
    assert_eq!(toggled_star, "* [ ] Star item 1\n* [ ] Star item 2\n");
}

#[test]
fn test_block_roundtrip_stability() {
    let source = "= Document Title\n\nParagraph with *bold* and _italic_ text.\n\n[source,rust]\n----\nfn main() {\n    println!(\"hello\");\n}\n----\n\n* Item 1\n* Item 2\n\nNOTE: This is an admonition note.\n";
    
    let blocks = parse_blocks(source);
    assert!(!blocks.is_empty());

    let adoc_output = blocks_to_adoc(&blocks);
    let re_parsed = parse_blocks(&adoc_output);
    assert_eq!(blocks.len(), re_parsed.len());

    for (b1, b2) in blocks.iter().zip(re_parsed.iter()) {
        assert_eq!(b1.block_type(), b2.block_type());
    }
}

#[test]
fn test_parse_blocks_with_drop_comments() {
    let source = "= Title\n\n// Line comment\nParagraph text\n\n// Another comment\n* [ ] Task\n";
    
    let with_comments = parse_blocks_with_options(source, false);
    let without_comments = parse_blocks_with_options(source, true);

    assert!(with_comments.len() > without_comments.len());
    let has_comment_block = without_comments.iter().any(|b| b.block_type() == "comment");
    assert!(!has_comment_block, "Should not contain comment blocks when drop_comments is true");
}
