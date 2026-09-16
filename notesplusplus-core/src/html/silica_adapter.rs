//! Adapter for converting AST / highlighted HTML tokens into formats compatible with
//! Qt Silica `Text.RichText` layout engines.

pub fn format_code_for_qml_richtext(highlighted_html: &str) -> String {
    let mut qml_html = String::with_capacity(highlighted_html.len());
    let mut at_line_start = true;
    let mut in_tag = false;

    for c in highlighted_html.chars() {
        match c {
            '\n' => {
                qml_html.push_str("<br/>");
                at_line_start = true;
                in_tag = false;
            }
            '<' => {
                qml_html.push(c);
                in_tag = true;
            }
            '>' => {
                qml_html.push(c);
                in_tag = false;
            }
            ' ' if at_line_start && !in_tag => {
                qml_html.push_str("&nbsp;");
            }
            ' ' if at_line_start && in_tag => {
                qml_html.push(c);
            }
            _ => {
                qml_html.push(c);
                if !in_tag {
                    at_line_start = false;
                }
            }
        }
    }

    qml_html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_code_for_qml_richtext() {
        let input = "<span>    let</span> x = 1;\n    let y = 2;";
        let formatted = format_code_for_qml_richtext(input);
        assert!(formatted.contains("<br/>"));
        assert!(formatted.contains("&nbsp;"));
    }
}
