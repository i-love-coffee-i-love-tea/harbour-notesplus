use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::parser;
use crate::escape::escape_html;

pub mod assets;
pub mod icons;
pub mod preprocess;
pub mod qt_html;
pub mod render;

use render::HtmlRenderContext;

pub use assets::{DOCUMENT_CSS, DOCUMENT_JS};
pub use icons::{get_admonition_svg_icon, get_standard_svg_icon};
pub use preprocess::preprocess_html;

/// Convert raw AsciiDoc content into a standalone HTML5 document.
pub fn adoc_to_html5(adoc_content: &str, title: &str, notes_dir: Option<&Path>) -> String {
    let blocks = parser::parse_blocks(adoc_content);
    blocks_to_html5(&blocks, title, notes_dir)
}

/// Convert raw AsciiDoc content into an HTML body snippet (for web preview & embeds).
pub fn adoc_to_html_body(adoc_content: &str, notes_dir: Option<&Path>) -> String {
    let blocks = parser::parse_blocks(adoc_content);
    blocks_to_html_body(&blocks, notes_dir)
}

/// Convert an AST of blocks into an HTML body snippet.
pub fn blocks_to_html_body(blocks: &[Block], notes_dir: Option<&Path>) -> String {
    let mut ctx = HtmlRenderContext::new(notes_dir, blocks);
    let body = ctx.render_blocks(blocks);
    let footnotes = ctx.render_footnotes();
    if footnotes.is_empty() {
        body
    } else {
        format!("{}\n{}", body, footnotes)
    }
}

/// Convert an AST of blocks into a standalone HTML5 document.
pub fn blocks_to_html5(blocks: &[Block], title: &str, notes_dir: Option<&Path>) -> String {
    let mut ctx = HtmlRenderContext::new(notes_dir, blocks);
    
    // First pass: extract document title if not provided and collect headings for TOC
    let doc_title = if title.is_empty() {
        extract_title(blocks).unwrap_or_else(|| "Notes Plus Document".to_string())
    } else {
        title.to_string()
    };

    let body_html = ctx.render_blocks(blocks);
    let footnotes_html = ctx.render_footnotes();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="generator" content="Notes Plus HTML5 Exporter">
    <title>{title}</title>
    <style>
{css}
    </style>
</head>
<body class="notes-body">
    <div class="notes-container">
        <header class="document-header">
            <h1 class="document-title">{title}</h1>
        </header>
        <main class="document-content">
{body}
{footnotes}
        </main>
        <footer class="document-footer">
            <p>Exported by <strong>Notes Plus</strong> on {date}</p>
        </footer>
    </div>
    <script>
{js}
    </script>
</body>
</html>"#,
        title = escape_html(&doc_title),
        css = DOCUMENT_CSS,
        js = DOCUMENT_JS,
        body = body_html,
        footnotes = footnotes_html,
        date = chrono::Local::now().format("%Y-%m-%d %H:%M"),
    )
}

/// Export a specific note page from `notes_dir` to an output HTML file.
/// `notes_dir` is used for reading .adoc files; `assets_dir` is used for image resolution.
pub fn export_page_to_html5(
    notes_dir: &Path,
    assets_dir: &Path,
    filename: &str,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let adoc_path = notes_dir.join(filename);
    let content = std::fs::read_to_string(&adoc_path)
        .map_err(|e| format!("Failed to read {}: {}", adoc_path.display(), e))?;

    let title = filename.strip_suffix(".adoc").unwrap_or(filename);
    let html = adoc_to_html5(&content, title, Some(assets_dir));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output dir {}: {}", parent.display(), e))?;
    }

    std::fs::write(output_path, html.as_bytes())
        .map_err(|e| format!("Failed to write HTML to {}: {}", output_path.display(), e))?;

    Ok(output_path.to_path_buf())
}

/// Export all .adoc files in `notes_dir` to HTML5 in `output_dir`.
/// `notes_dir` is used for reading .adoc files; `assets_dir` is used for image resolution.
pub fn export_all_pages_to_html5(
    notes_dir: &Path,
    assets_dir: &Path,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let mut exported = Vec::new();
    let entries = std::fs::read_dir(notes_dir)
        .map_err(|e| format!("Failed to read notes directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let out_filename = format!("{}.html", name.strip_suffix(".adoc").unwrap_or(name));
                let out_path = output_dir.join(out_filename);
                match export_page_to_html5(notes_dir, assets_dir, name, &out_path) {
                    Ok(p) => exported.push(p),
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Ok(exported)
}

/// Sanitizes a URL for safe rendering in HTML `<a href="...">` tags, neutralizing dangerous schemes like `javascript:`, `vbscript:`, and `data:`.
pub fn sanitize_url_scheme(url: &str) -> String {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();
    // Allow relative paths or fragment anchors
    if trimmed.starts_with('/') || trimmed.starts_with('#') || trimmed.starts_with('?') || trimmed.starts_with("./") {
        return trimmed.to_string();
    }
    // Filter out whitespace and control characters to detect obfuscated schemes (e.g. "java\tscript:")
    let clean_scheme: String = lower.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect();
    if let Some((scheme, _)) = clean_scheme.split_once(':') {
        let allowed_schemes = ["http", "https", "mailto", "tel", "ftp", "ftps", "news", "geo", "sms"];
        if allowed_schemes.contains(&scheme) {
            return trimmed.to_string();
        }
        // Neutralize dangerous / disallowed URL scheme
        return format!("#blocked:{}", escape_html(trimmed));
    }
    trimmed.to_string()
}


fn extract_title(blocks: &[Block]) -> Option<String> {
    for b in blocks {
        if let Block::Heading { level: 1, spans, .. } = b {
            return Some(spans.iter().map(|s| s.plain_text()).collect::<String>());
        }
    }
    None
}

/// Simple RFC 4648 Base64 Encoder without extra external dependencies
pub(crate) fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(TABLE[(b0 >> 2) as usize] as char);
        result.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adoc_to_html5_simple() {
        let adoc = r#"
= My Test Document
Author Name

This is a paragraph with *bold*, _italic_, and `code` text.

== Section One

- [ ] Task 1
- [x] Task 2
- Normal item

[NOTE]
====
This is a helpful note!
====

[source,rust]
----
fn main() {
    println!("Hello World!");
}
----
"#;
        let html = adoc_to_html5(adoc, "My Test Document", None);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>My Test Document</title>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("class=\"admonitionblock note\""));
        assert!(html.contains("class=\"code-block language-rust\""));
        assert!(html.contains("<h2 id=\"section-one\" class=\"sect-heading sect2\">Section One</h2>"));
        assert!(!html.contains("<a class=\"anchor\""));
    }

    #[test]
    fn test_heading_rendering_without_anchor_hash() {
        let adoc = "== My Section\n\n=== My Subsection\n";
        let html = adoc_to_html_body(adoc, None);
        assert_eq!(
            html.trim(),
            "<h2 id=\"my-section\" class=\"sect-heading sect2\">My Section</h2>\n\n<h3 id=\"my-subsection\" class=\"sect-heading sect3\">My Subsection</h3>"
        );
    }

    #[test]
    fn test_html_table_rendering() {
        let adoc = r#"
|===
| Header 1 | Header 2

| Cell 1
| Cell 2
|===
"#;
        let html = adoc_to_html5(adoc, "Table Test", None);
        assert!(html.contains("<table class=\"table"));
        assert!(html.contains("<th>"));
        assert!(html.contains("Header 1"));
        assert!(html.contains("Cell 1"));
    }

    #[test]
    fn test_list_grouping_and_nesting() {
        let adoc = r#"
* Item 1
* Item 2
** Nested A
** Nested B
* Item 3

. First
. Second
.. Sub 1
.. Sub 2

[source,rust]
----
fn main() { // <1>
    println!("hi"); // <2>
}
----
<1> Entry point
<2> Print statement

Term 1:: Description 1
Term 2:: Description 2
"#;
        let html = adoc_to_html5(adoc, "List Test", None);
        // Unordered list is enclosed in <ul> or <div class="ulist"><ul>
        assert!(html.contains("<ul"));
        assert!(html.contains("</ul>"));
        // Ordered list is enclosed in <ol
        assert!(html.contains("<ol"));
        assert!(html.contains("</ol>"));
        // Description list is enclosed in <dl
        assert!(html.contains("<dl"));
        assert!(html.contains("</dl>"));
        // Callout list is enclosed in <ol class="calloutlist" or colist
        assert!(html.contains("callout"));
        // Check nesting: Nested A is inside a nested <ul>
        assert!(html.contains("Nested A"));
    }

    #[test]
    fn test_admonition_rendering_with_svg() {
        let adoc = r#"
[NOTE]
====
This is a note.
====

[TIP]
====
This is a tip.
====

[WARNING]
====
This is a warning.
====
"#;
        let html = adoc_to_html5(adoc, "Admonitions", None);
        assert!(html.contains("class=\"admonitionblock note\""));
        assert!(html.contains("class=\"admonitionblock tip\""));
        assert!(html.contains("class=\"admonitionblock warning\""));
        assert!(html.contains("<svg"));
        assert!(html.contains(".admonitionblock {\n    margin: 1.5em 0;\n    padding: 14px 18px;\n    border-radius: 0;"));
    }

    #[test]
    fn test_table_frames_and_grids() {
        let adoc = r#"
[frame="topbot",grid="rows"]
|===
| Header A | Header B

| Val 1 | Val 2
|===
"#;
        let html = adoc_to_html5(adoc, "Table Frame Grid", None);
        assert!(html.contains("frame-topbot"));
        assert!(html.contains("grid-rows"));
    }

    #[test]
    fn test_inline_macros_and_icons() {
        let adoc = r#"
Press kbd:[Ctrl+T] or click btn:[Save].
Navigate to menu:File[New > Project].
Marked text: #highlighted#
Standard icon: icon:star[] and icon:folder[]
Callout conum: <1>
Cross reference: xref:other-page.adoc[Other Page]
"#;
        let html = adoc_to_html5(adoc, "Inline Macros", None);
        assert!(html.contains("<kbd>Ctrl</kbd>"));
        assert!(html.contains("<kbd>T</kbd>"));
        assert!(html.contains("class=\"btn\""));
        assert!(html.contains("class=\"menuseq\""));
        assert!(html.contains("<mark>"));
        assert!(html.contains("class=\"conum\""));
        assert!(html.contains(".conum {"));
        assert!(html.contains("background-color: var(--note-bg);"));
        assert!(html.contains("color: var(--note-border);"));
        assert!(html.contains("class=\"xref\""));
        assert!(html.contains("class=\"notes-icon icon-star\""));
        assert!(html.contains("class=\"notes-icon icon-folder\""));
    }

    #[test]
    fn test_checklist_rendering_and_styling() {
        let adoc = r#"
= Checklist Note

* [ ] Buy groceries
* [x] Finish documentation
* Normal bullet item
"#;
        let html = adoc_to_html5(adoc, "Checklist Note", None);
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("<input type=\"checkbox\" disabled class=\"checklist-checkbox\">"));
        assert!(html.contains("<input type=\"checkbox\" checked disabled class=\"checklist-checkbox\">"));
        assert!(html.contains("Buy groceries"));
        assert!(html.contains("Finish documentation"));
        // Verify CSS includes inline paragraph styling for list items and proper checklist styling
        assert!(html.contains(".checklist-item {"));
        assert!(html.contains("li > p {"));
    }

    #[test]
    fn test_nested_checklist_rendering() {
        let adoc = r#"
* [ ] Parent task
  * [x] Child task 1
  * [ ] Child task 2
"#;
        let html = adoc_to_html5(adoc, "Nested Checklist", None);
        assert!(html.contains("class=\"checklist-item unchecked\""));
        assert!(html.contains("class=\"checklist-item checked\""));
        assert!(html.contains("Parent task"));
        assert!(html.contains("Child task 1"));
        assert!(html.contains("Child task 2"));
        // Ensure child list is nested inside parent list item correctly
        assert!(html.contains("<li class=\"checklist-item unchecked\"><input type=\"checkbox\" disabled class=\"checklist-checkbox\"><p>Parent task</p>\n<div class=\"ulist checklist\"><ul class=\"checklist\">"));
    }

    #[test]
    fn test_dynamic_toc_generation() {
        let adoc = r#"
= Master Document

toc::[]

== Chapter One
Intro text.

=== Sub-section A
Details.

=== Sub-section B
More details.

== Chapter Two
Conclusion.
"#;
        let html = adoc_to_html5(adoc, "Master Document", None);
        assert!(html.contains("class=\"toc\""));
        assert!(html.contains("Table of Contents"));
        assert!(html.contains("href=\"#chapter-one\""));
        assert!(html.contains("Chapter One"));
        assert!(html.contains("href=\"#sub-section-a\""));
        assert!(html.contains("Sub-section A"));
        assert!(html.contains("href=\"#chapter-two\""));
        assert!(html.contains("Chapter Two"));
    }

    #[test]
    fn escape_html_escapes_angle_brackets() {
        assert_eq!(escape_html("<script>alert(1)</script>"),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
    }

    #[test]
    fn escape_html_escapes_quotes() {
        assert_eq!(escape_html(r#"a"b'c"#), "a&quot;b&#39;c");
    }

    #[test]
    fn escape_html_escapes_ampersand() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
    }

    #[test]
    fn test_callout_list_rendering_and_styles() {
        let adoc = r#"
[source,rust]
----
fn main() { // <1>
    println!("hello"); // <2>
}
----
<1> Entrypoint function
<2> Print statement
"#;
        let html = adoc_to_html5(adoc, "Callout Test", None);
        assert!(html.contains("<ol class=\"calloutlist\">"));
        assert!(html.contains("<li class=\"callout-item\"><b class=\"conum\"><span class=\"conum-badge\">1</span></b> <p>Entrypoint function</p></li>"));
        assert!(html.contains("<li class=\"callout-item\"><b class=\"conum\"><span class=\"conum-badge\">2</span></b> <p>Print statement</p></li>"));
        assert!(html.contains(".callout-item {"));
        assert!(html.contains("list-style-type: none;"));
    }

    #[test]
    fn test_footnote_deduplication_rendering() {
        let adoc = r#"
Here is a statement with footnote:defops[DefOps is great].
And here we refer to the same footnote:defops[].
And another reference: footnote:defops[].
"#;
        let html = adoc_to_html5(adoc, "Footnotes Test", None);
        assert!(html.contains("<sup class=\"footnote\" id=\"fnref-1\"><a href=\"#defops\">[1]</a></sup>"));
        assert!(html.contains("<sup class=\"footnote\"><a href=\"#defops\">[1]</a></sup>"));
        // Check that footnotes section contains only 1 footnote entry and no empty items
        assert!(html.contains("<li id=\"defops\"><p>DefOps is great <a href=\"#fnref-1\">&#8617;</a></p></li>"));
        assert!(!html.contains("<li id=\"fn-2\">"));
        assert!(!html.contains("<li id=\"fn-3\">"));
    }

    #[test]
    fn test_table_alignments_and_colspans_from_chronicles() {
        let adoc = r#"
[%header%footer,cols="2,2s,^4",grid=rows,frame=ends,width=75%,caption=]
|===
|Name |Title |Alias

|Sarah White
|President
|http://twitter.com/carbonfray[@carbonfray]

|Dan Allen
|Vice President
|http://twitter.com/mojavelinux[@mojavelinux]

3+^.e|Powered by Open Source
|===
"#;
        let html = adoc_to_html5(adoc, "Chronicles Table", None);
        assert!(html.contains("<table class=\"table frame-ends grid-rows\">"));
        assert!(html.contains("<col style=\"width: 25.0000%;\">"));
        assert!(html.contains("<col style=\"width: 50.0000%;\">"));
        assert!(html.contains("<td colspan=\"3\" style=\"text-align: center;\"><p><em>Powered by Open Source</em></p></td>"));
        assert!(html.contains("style=\"text-align: center;\""));
    }

    #[test]
    fn test_sanitize_url_scheme() {
        // Allowed schemes
        assert_eq!(sanitize_url_scheme("https://example.com"), "https://example.com");
        assert_eq!(sanitize_url_scheme("http://example.com/test"), "http://example.com/test");
        assert_eq!(sanitize_url_scheme("mailto:user@example.com"), "mailto:user@example.com");
        assert_eq!(sanitize_url_scheme("tel:+1234567890"), "tel:+1234567890");
        assert_eq!(sanitize_url_scheme("/sub/page.html"), "/sub/page.html");
        assert_eq!(sanitize_url_scheme("#section-heading"), "#section-heading");

        // Dangerous schemes neutralized
        assert!(sanitize_url_scheme("javascript:alert(document.cookie)").starts_with("#blocked:"));
        assert!(sanitize_url_scheme("JAVASCRIPT:alert(1)").starts_with("#blocked:"));
        assert!(sanitize_url_scheme("java\tscript:alert(1)").starts_with("#blocked:"));
        assert!(sanitize_url_scheme("vbscript:msgbox(1)").starts_with("#blocked:"));
        assert!(sanitize_url_scheme("data:text/html,<script>alert(1)</script>").starts_with("#blocked:"));

        // AsciiDoc rendering test
        let adoc_xss = "link:javascript:alert(1)[Malicious Link]";
        let html_rendered = adoc_to_html_body(adoc_xss, None);
        assert!(!html_rendered.contains("href=\"javascript:"));
        assert!(html_rendered.contains("href=\"#blocked:"));
        assert!(html_rendered.contains("Malicious Link"));
    }
}
