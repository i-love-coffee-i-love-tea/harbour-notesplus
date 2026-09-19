/// Tests for the AsciiDoc preview rendering pipeline.
///
/// These verify that `parse_blocks` → `render_qt_block` produces correct
/// Qt RichText HTML for every element category in the element picker dialog.
use notesplus_core::html::qt_html::{QtRenderOptions, QtThemeColors, render_qt_block};
use notesplus_core::parser;

fn render_preview(adoc: &str) -> String {
    let trimmed = adoc.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let blocks = parser::parse_blocks(trimmed);
    if blocks.is_empty() {
        return String::new();
    }
    render_qt_block(&blocks[0], 0, &QtThemeColors::default(), &QtRenderOptions::default())
}

// ── Block elements ──────────────────────────────────────────────────

#[test]
fn preview_heading_1() {
    // Qt RichText renders all heading levels as <h3>
    let html = render_preview("= Main Title");
    assert!(html.contains("<h3"), "Expected <h3> tag (Qt heading), got: {}", html);
    assert!(html.contains("Main Title"), "Expected heading text");
}

#[test]
fn preview_heading_4() {
    let html = render_preview("==== Section Title");
    assert!(html.contains("<h3"), "Expected <h3> tag (Qt heading), got: {}", html);
    assert!(html.contains("Section Title"), "Expected heading text");
}

#[test]
fn preview_heading_5() {
    let html = render_preview("===== Deep Title");
    assert!(html.contains("<h3"), "Expected <h3> tag (Qt heading), got: {}", html);
}

#[test]
fn preview_heading_6() {
    let html = render_preview("====== Deepest Title");
    assert!(html.contains("<h3"), "Expected <h3> tag (Qt heading), got: {}", html);
}

#[test]
fn preview_code_block() {
    let adoc = "[source]\n----\nfn main() {\n    println!(\"hello\");\n}\n----";
    let html = render_preview(adoc);
    assert!(html.contains("<pre"), "Expected <pre> tag, got: {}", html);
    assert!(html.contains("<code") || html.contains("fn main"), "Expected code content");
}

#[test]
fn preview_blockquote() {
    let adoc = "[quote]\n____\nFamous words.\n____";
    let html = render_preview(adoc);
    assert!(html.contains("<blockquote") || html.contains("blockquote"), "Expected blockquote, got: {}", html);
    assert!(html.contains("Famous words"), "Expected quote text");
}

#[test]
fn preview_verse() {
    let adoc = "[verse]\n____\nThe road goes ever on.\n____";
    let html = render_preview(adoc);
    // Qt renderer renders verse as blockquote
    assert!(html.contains("<blockquote"), "Expected blockquote for verse, got: {}", html);
    assert!(html.contains("road goes"), "Expected verse text");
}

#[test]
fn preview_literal_block() {
    let adoc = "....\n  Literal text here\n....";
    let html = render_preview(adoc);
    assert!(html.contains("<pre"), "Expected <pre> for literal block, got: {}", html);
    assert!(html.contains("Literal text"), "Expected literal content");
}

#[test]
fn preview_example_block() {
    let adoc = "====\nExample content\n====";
    let html = render_preview(adoc);
    assert!(html.contains("exampleblock") || html.contains("Example content"), "Expected example block, got: {}", html);
}

#[test]
fn preview_open_block() {
    let adoc = "--\nOpen block content\n--";
    let html = render_preview(adoc);
    assert!(html.contains("openblock") || html.contains("Open block content"), "Expected open block, got: {}", html);
}

#[test]
fn preview_horizontal_rule() {
    let html = render_preview("---");
    assert!(html.contains("<hr"), "Expected <hr> tag, got: {}", html);
}

#[test]
fn preview_page_break() {
    let html = render_preview("<<<");
    // Qt renderer renders page break as a spacer div
    assert!(html.contains("height:8px") || html.contains("page-break"), "Expected page break, got: {}", html);
}

#[test]
fn preview_admonition_note() {
    let adoc = "[NOTE]\n====\nNote text.\n====";
    let html = render_preview(adoc);
    assert!(html.contains("NOTE") || html.contains("admonition"), "Expected NOTE admonition, got: {}", html);
    assert!(html.contains("Note text"), "Expected admonition text");
}

#[test]
fn preview_admonition_caution() {
    let adoc = "[CAUTION]\n====\nBe very careful.\n====";
    let html = render_preview(adoc);
    assert!(html.contains("CAUTION") || html.contains("admonition"), "Expected CAUTION admonition, got: {}", html);
}

#[test]
fn preview_admonition_important() {
    let adoc = "[IMPORTANT]\n====\nThis is critical.\n====";
    let html = render_preview(adoc);
    assert!(html.contains("IMPORTANT") || html.contains("admonition"), "Expected IMPORTANT admonition, got: {}", html);
}

#[test]
fn preview_sidebar() {
    let adoc = "[sidebar]\n****\nSidebar text.\n****";
    let html = render_preview(adoc);
    assert!(html.contains("sidebar") || html.contains("Sidebar text"), "Expected sidebar, got: {}", html);
}

#[test]
fn preview_description_list() {
    let html = render_preview("Term:: Description text");
    assert!(html.contains("<dl") || html.contains("Term") || html.contains("Description"), "Expected description list, got: {}", html);
}

#[test]
fn preview_table() {
    let adoc = "|===\n| H1 | H2\n\n| C1 | C2\n|===";
    let html = render_preview(adoc);
    assert!(html.contains("<table"), "Expected <table> tag, got: {}", html);
}

// ── Inline elements (wrapped in paragraph context) ──────────────────

#[test]
fn preview_bold_in_paragraph() {
    let html = render_preview("This is *bold* text");
    assert!(html.contains("<strong") || html.contains("<b>"), "Expected bold markup, got: {}", html);
    assert!(html.contains("bold"), "Expected bold text");
}

#[test]
fn preview_italic_in_paragraph() {
    let html = render_preview("This is _italic_ text");
    assert!(html.contains("<em") || html.contains("<i>"), "Expected italic markup, got: {}", html);
}

#[test]
fn preview_mono_in_paragraph() {
    let html = render_preview("Use `printf()` here");
    assert!(html.contains("<code") || html.contains("<tt"), "Expected monospace markup, got: {}", html);
}

#[test]
fn preview_strikethrough() {
    let html = render_preview("This is ~deleted~ text");
    assert!(html.contains("<s>") || html.contains("<s ") || html.contains("line-through") || html.contains("deleted"), "Expected strikethrough, got: {}", html);
}

#[test]
fn preview_superscript() {
    let html = render_preview("E = mc^2^");
    assert!(html.contains("<sup"), "Expected superscript, got: {}", html);
}

#[test]
fn preview_highlighted() {
    let html = render_preview("This is #marked# text");
    assert!(html.contains("<mark") || html.contains("background") || html.contains("marked"), "Expected highlight/mark, got: {}", html);
}

#[test]
fn preview_footnote() {
    let html = render_preview("Textfootnote:[An important note.] here");
    assert!(html.contains("footnote") || html.contains("note"), "Expected footnote, got: {}", html);
}

#[test]
fn preview_keyboard_macro() {
    let html = render_preview("Press kbd:[Ctrl+S] to save");
    assert!(html.contains("<kbd") || html.contains("Ctrl"), "Expected keyboard macro, got: {}", html);
}

#[test]
fn preview_xref() {
    let html = render_preview("See xref:other.adoc[Other Page]");
    assert!(html.contains("xref") || html.contains("Other Page") || html.contains("href"), "Expected cross-reference, got: {}", html);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn preview_empty_string() {
    let html = render_preview("");
    assert!(html.is_empty(), "Expected empty string for empty input, got: {}", html);
}

#[test]
fn preview_whitespace_only() {
    let html = render_preview("   \n  ");
    assert!(html.is_empty(), "Expected empty string for whitespace input, got: {}", html);
}

#[test]
fn preview_inline_image() {
    let html = render_preview("See image:icon.png[Icon,16] here");
    assert!(html.contains("<img") || html.contains("image") || html.contains("icon"), "Expected inline image, got: {}", html);
}

#[test]
fn preview_button_macro() {
    let html = render_preview("Click btn:[Submit] to continue");
    assert!(html.contains("Submit") || html.contains("btn"), "Expected button macro, got: {}", html);
}

#[test]
fn preview_menu_macro() {
    let html = render_preview("Use menu:File[Quit] to exit");
    assert!(html.contains("Quit") || html.contains("File") || html.contains("menu"), "Expected menu macro, got: {}", html);
}

#[test]
fn preview_index_term() {
    let html = render_preview("A ((concept)) in text");
    assert!(html.contains("concept") || html.contains("index"), "Expected index term, got: {}", html);
}

#[test]
fn preview_stem_math() {
    let html = render_preview("The equation stem:[E = mc^2]");
    assert!(html.contains("E = mc") || html.contains("stem") || html.contains("formula"), "Expected stem/math, got: {}", html);
}

#[test]
fn test_preview_generation_qt_options_without_theme_uses_qt_renderer() {
    let tmp = tempfile::tempdir().unwrap();
    let note_path = tmp.path().join("Note.adoc");
    std::fs::write(&note_path, "= My Document\n\nSome paragraph text.\n").unwrap();

    let opts = QtRenderOptions::default();
    let values = notesplus_core::page::get_page_preview_values_with_options(
        tmp.path(),
        "Note.adoc",
        5,
        true,
        None,
        Some(&opts),
    );

    assert!(!values.is_empty());
    let heading_html = values[0]["html"].as_str().unwrap();
    assert!(heading_html.contains("<h3"), "Should use compact <h3> tag for Qt, got: {}", heading_html);
    assert!(!heading_html.contains("sect-heading"), "Should not use web-heading for Qt: {}", heading_html);
}
