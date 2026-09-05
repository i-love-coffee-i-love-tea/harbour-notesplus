use std::path::Path;

/// Convert raw AsciiDoc source to a standalone HTML5 document for printpdf.
/// Uses asciidoc-html5's standalone mode which includes the full asciidoctor CSS.
/// Strips external font links (Google Fonts) since printpdf can't fetch them,
/// and fixes HTML5 boolean attributes for XML compatibility.
pub fn adoc_to_styled_html(adoc_content: &str, _adoc_dir: &Path) -> Result<String, String> {
    let options = asciidoc_html5::Options::new()
        .standalone(true)
        .safe_mode(asciidoc_html5::SafeMode::Unsafe)
        .unset("webfonts")
        .set("nofooter");

    let html = asciidoc_html5::convert_with(adoc_content, &options);

    // Strip external font links that printpdf can't resolve
    let html = strip_external_resources(&html);

    // Fix HTML5 boolean attributes to XHTML-compatible form
    let html = fix_html5_to_xhtml(&html);

    Ok(html)
}

/// Remove external stylesheet and font links that printpdf can't fetch.
fn strip_external_resources(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    for line in html.lines() {
        let trimmed = line.trim();
        // Skip Google Fonts link
        if trimmed.starts_with("<link") && trimmed.contains("fonts.googleapis.com") {
            continue;
        }
        // Skip external stylesheet link (asciidoctor.css)
        if trimmed.starts_with("<link") && trimmed.contains("asciidoctor.css") {
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Convert HTML5 boolean attributes to XHTML-compatible form.
/// HTML5: <ol reversed>  →  XHTML: <ol reversed="reversed">
fn fix_html5_to_xhtml(html: &str) -> String {
    let boolean_attrs = [
        "reversed", "checked", "selected", "disabled", "readonly",
        "multiple", "autofocus", "autoplay", "loop", "muted",
        "controls", "nowrap", "noshade", "compact", "declare",
        "noresize", "default", "open", "hidden", "required",
    ];

    let mut result = html.to_string();
    for attr in &boolean_attrs {
        // Match: attr> or attr\n (inside a tag)
        let from_space = format!(" {}>", attr);
        let to_space = format!(" {}=\"{}\">", attr, attr);
        result = result.replace(&from_space, &to_space);

        let from_newline = format!(" {}\n", attr);
        let to_newline = format!(" {}=\"{}\"\n", attr, attr);
        result = result.replace(&from_newline, &to_newline);
    }

    // Fix self-closing tags for XML compatibility
    for tag in &["br", "hr", "input", "col", "area", "base", "embed", "param", "source", "track", "wbr"] {
        let from = format!("<{}>", tag);
        let to = format!("<{}/>", tag);
        result = result.replace(&from, &to);
    }

    result
}
