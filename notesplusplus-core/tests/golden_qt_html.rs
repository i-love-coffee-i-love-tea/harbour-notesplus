/// Golden-file integration tests for qt_html renderer.
///
/// Run with UPDATE_GOLDEN=1 to regenerate golden files:
///   UPDATE_GOLDEN=1 cargo test -p notesplusplus-core -- golden
///
/// Without UPDATE_GOLDEN, tests assert byte-identical output against saved files.
use notesplusplus_core::html::qt_html::{QtRenderOptions, QtThemeColors, render_qt_block, render_qt_spans, highlight_search_terms};
use notesplusplus_core::inline::parse_inline;
use notesplusplus_core::parser;
use std::fs;
use std::path::Path;

fn default_theme() -> QtThemeColors { QtThemeColors::default() }
fn default_opts() -> QtRenderOptions { QtRenderOptions::default() }

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn assert_or_update_golden(name: &str, actual: &str) {
    let dir = golden_dir();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{}.html", name));

    if std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1") {
        fs::write(&path, actual).unwrap();
        return;
    }

    if !path.exists() {
        fs::write(&path, actual).unwrap();
        panic!("Golden file {} did not exist — created it. Re-run test to verify.", path.display());
    }

    let expected = fs::read_to_string(&path).unwrap();
    assert_eq!(
        expected, actual,
        "\n\nGolden file mismatch for '{}'.\nTo update: UPDATE_GOLDEN=1 cargo test -p notesplusplus-core -- golden\n\nExpected:\n{}\n\nActual:\n{}\n",
        name, expected, actual
    );
}

// ── Block type golden tests ──────────────────────────────────────────

#[test]
fn golden_paragraph() {
    let blocks = parser::parse_blocks("Hello *world* and _emphasis_.");
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("paragraph", &html);
}

#[test]
fn golden_heading_levels() {
    for level in 1..=4 {
        let prefix = "=".repeat(level as usize);
        let blocks = parser::parse_blocks(&format!("{} Heading Level {}", prefix, level));
        let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
        assert_or_update_golden(&format!("heading_l{}", level), &html);
    }
}

#[test]
fn golden_code_block() {
    let adoc = "----\nfn main() {\n    println!(\"hello\");\n}\n----";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("code_block", &html);
}

#[test]
fn golden_svgbob_block_qt() {
    let adoc = "[source,svgbob]\n----\n+---+\n| A |\n+---+\n----";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("svgbob_block_qt", &html);
}

#[test]
fn golden_unordered_list() {
    let blocks = parser::parse_blocks("* Item A\n* Item B\n* Item C");
    let mut full_html = String::new();
    for (i, b) in blocks.iter().enumerate() {
        full_html.push_str(&render_qt_block(b, i, &default_theme(), &default_opts()));
        full_html.push('\n');
    }
    assert_or_update_golden("unordered_list", &full_html.trim());
}

#[test]
fn golden_nested_checkbox_list() {
    let adoc = "* [x] Done task\n* [ ] Todo task\n  * [x] Nested done\n  * [ ] Nested todo";
    let blocks = parser::parse_blocks(adoc);
    let mut full_html = String::new();
    for (i, b) in blocks.iter().enumerate() {
        full_html.push_str(&render_qt_block(b, i, &default_theme(), &default_opts()));
        full_html.push('\n');
    }
    assert_or_update_golden("nested_checkbox_list", &full_html.trim());
}

#[test]
fn golden_ordered_list() {
    let blocks = parser::parse_blocks(". First\n. Second\n. Third");
    let mut full_html = String::new();
    for (i, b) in blocks.iter().enumerate() {
        full_html.push_str(&render_qt_block(b, i, &default_theme(), &default_opts()));
        full_html.push('\n');
    }
    assert_or_update_golden("ordered_list", &full_html.trim());
}

#[test]
fn golden_description_list() {
    let adoc = "Term 1:: Definition 1\nTerm 2:: Definition 2";
    let blocks = parser::parse_blocks(adoc);
    let mut full_html = String::new();
    for (i, b) in blocks.iter().enumerate() {
        full_html.push_str(&render_qt_block(b, i, &default_theme(), &default_opts()));
        full_html.push('\n');
    }
    assert_or_update_golden("description_list", &full_html.trim());
}

#[test]
fn golden_admonition() {
    let adoc = "[NOTE]\n====\nBe careful with this.\n====";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("admonition", &html);
}

#[test]
fn golden_sidebar_with_title() {
    let adoc = ".My Sidebar Title\n****\nSome sidebar content here.\n****";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("sidebar_titled", &html);
}

#[test]
fn golden_blockquote() {
    let adoc = "[quote]\n____\nThe only way to do great work is to love what you do.\n____";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("blockquote", &html);
}

#[test]
fn golden_table() {
    let adoc = "|===\n| Name | Age\n| Alice | 30\n| Bob | 25\n|===";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("table", &html);
}

#[test]
fn golden_empty_line() {
    let blocks = parser::parse_blocks("Hello\n\nWorld");
    let html = render_qt_block(&blocks[1], 1, &default_theme(), &default_opts());
    assert_or_update_golden("empty_line", &html);
}

#[test]
fn golden_image() {
    let adoc = "image::photo.jpg[A photo]";
    let blocks = parser::parse_blocks(adoc);
    let html = render_qt_block(&blocks[0], 0, &default_theme(), &default_opts());
    assert_or_update_golden("image", &html);
}

// ── Inline span golden tests ─────────────────────────────────────────

#[test]
fn golden_spans_mixed() {
    let spans = parse_inline("Hello *bold* and _italic_ and `code`.");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("spans_mixed", &html);
}

#[test]
fn golden_spans_nested() {
    let spans = parse_inline("*bold _and italic_*");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("spans_nested", &html);
}

#[test]
fn golden_span_strikethrough() {
    let spans = parse_inline("this is ~~deleted~~ text");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("span_strikethrough", &html);
}

#[test]
fn golden_span_superscript() {
    let spans = parse_inline("x^2^ is a power");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("span_superscript", &html);
}

#[test]
fn golden_span_link() {
    let spans = parse_inline("visit https://example.com[Example Site] here");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("span_link", &html);
}

#[test]
fn golden_span_xref() {
    let spans = parse_inline("see <<other-page>> for details");
    let html = render_qt_spans(&spans, &default_theme(), &default_opts());
    assert_or_update_golden("span_xref", &html);
}

// ── Search highlighting golden tests ─────────────────────────────────

#[test]
fn golden_search_highlight() {
    let html = "<p style='margin:4px 0;'>Hello world, hello again</p>";
    let result = highlight_search_terms(html, "hello");
    assert_or_update_golden("search_highlight", &result);
}

#[test]
fn golden_search_highlight_multi_term() {
    let html = "<p style='margin:4px 0;'>The quick brown fox jumps</p>";
    let result = highlight_search_terms(html, "quick fox");
    assert_or_update_golden("search_highlight_multi", &result);
}

#[test]
fn golden_search_highlight_no_match_in_tags() {
    let html = "<p style='margin:4px 0;'>Hello</p>";
    let result = highlight_search_terms(html, "style");
    assert_or_update_golden("search_no_tag_match", &result);
}

// ── Custom theme golden test ─────────────────────────────────────────

#[test]
fn golden_custom_theme() {
    let theme = QtThemeColors {
        highlight_color: "#e91e63".into(),
        primary_color: "#212121".into(),
        highlight_background_color: "rgba(233,30,99,0.25)".into(),
    };
    let blocks = parser::parse_blocks("= Title with custom theme");
    let html = render_qt_block(&blocks[0], 0, &theme, &default_opts());
    assert_or_update_golden("custom_theme", &html);
}

// ── Full document golden test ────────────────────────────────────────

#[test]
fn golden_full_adoc_document() {
    let adoc = r#"= My Document

== Introduction

This is a *bold* statement with _italic_ and `code` inline.

== Lists

* Item one
* [x] Checked item
* [ ] Unchecked item
  * Nested item

. First ordered
. Second ordered

Term:: Definition

== Code

----
let x = 42;
println!("{}", x);
----

== Blockquote

[quote]
____
Famous words by someone.
____

== Table

|===
| Name | Role
| Alice | Engineer
| Bob | Designer
|===

== Admonition

[TIP]
====
Remember to save your work.
====

== Sidebar

.Important Note
****
This is sidebar content.
****
"#;
    let blocks = parser::parse_blocks(adoc);
    let mut full_html = String::new();
    for (i, b) in blocks.iter().enumerate() {
        full_html.push_str(&render_qt_block(b, i, &default_theme(), &default_opts()));
        full_html.push('\n');
    }
    assert_or_update_golden("full_document", &full_html.trim());
}
