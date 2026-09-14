use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use crate::escape::escape_html;

thread_local! {
    static SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

/// Highlight code with syntax coloring and return HTML with inline `<span>` elements.
/// Falls back to escaped plain text if the language is unknown.
pub fn highlight_code(code: &str, language: &str) -> String {
    SYNTAX_SET.with(|ss| {
        THEME_SET.with(|ts| {
            let syntax = ss
                .find_syntax_by_token(language)
                .unwrap_or_else(|| ss.find_syntax_plain_text());
            let theme = &ts.themes["base16-ocean.dark"];
            let mut h = HighlightLines::new(syntax, theme);

            let mut html = String::new();
            for line in code.split('\n') {
                let ranges = h.highlight_line(line, ss).unwrap_or_default();
                let line_html = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
                    .unwrap_or_else(|_| escape_html(line));
                html.push_str(&line_html);
                html.push('\n');
            }
            // Remove trailing newline added by the loop
            if html.ends_with('\n') {
                html.pop();
            }
            html
        })
    })
}

/// Map common AsciiDoc language aliases to syntect syntax tokens.
pub fn normalize_language(lang: &str) -> String {
    match lang.to_ascii_lowercase().as_str() {
        "adoc" | "asciidoc" => "asciidoc",
        "rs" | "rust" => "rust",
        "js" | "javascript" => "javascript",
        "ts" | "typescript" => "typescript",
        "py" | "python" => "python",
        "sh" | "bash" | "shell" => "bash",
        "rb" | "ruby" => "ruby",
        "yml" | "yaml" => "yaml",
        "md" | "markdown" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "xml" => "xml",
        "sql" => "sql",
        "c" => "c",
        "cpp" | "c++" | "cxx" => "cpp",
        "h" | "hpp" => "cpp",
        "java" => "java",
        "go" => "go",
        "swift" => "swift",
        "kt" | "kotlin" => "kotlin",
        "toml" => "toml",
        "ini" => "ini",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        other => return other.to_string(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let html = highlight_code(code, "rust");
        assert!(html.contains("<span"), "should contain span tags for syntax highlighting");
        assert!(html.contains("fn"), "should contain the code content");
        assert!(html.contains("main"), "should contain function name");
    }

    #[test]
    fn test_highlight_python() {
        let code = "def hello():\n    print('world')";
        let html = highlight_code(code, "python");
        assert!(html.contains("<span"), "should contain span tags");
        assert!(html.contains("def"), "should contain Python keyword");
    }

    #[test]
    fn test_highlight_unknown_lang_falls_back() {
        let code = "some text";
        let html = highlight_code(code, "nonexistent_lang");
        assert!(html.contains("some text"), "should contain the original text");
    }

    #[test]
    fn test_highlight_newlines_preserved() {
        let tests = vec![
            ("ruby", "def hello\n  puts \"world\"\nend"),
            ("xml", "<root>\n  <child>text</child>\n</root>"),
            ("java", "class Foo {\n  void bar() {}\n}"),
            ("css", "body {\n  color: red;\n}"),
            ("rust", "fn main() {\n  println!(\"hi\");\n}"),
        ];
        for (lang, code) in &tests {
            let html = highlight_code(code, lang);
            let newline_count = html.matches('\n').count();
            assert_eq!(newline_count, 2, "{}: expected 2 newlines, got {}. HTML:\n{}", lang, newline_count, html);
            // Verify leading spaces are preserved (inside or outside span tags)
            assert!(html.contains("  "), "{}: expected leading spaces in output", lang);
        }
    }

    #[test]
    fn test_normalize_language() {
        assert_eq!(normalize_language("rs"), "rust");
        assert_eq!(normalize_language("Rust"), "rust");
        assert_eq!(normalize_language("python"), "python");
        assert_eq!(normalize_language("py"), "python");
        assert_eq!(normalize_language("js"), "javascript");
        assert_eq!(normalize_language("unknown"), "unknown");
    }
}
