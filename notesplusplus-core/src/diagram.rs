/// Render svgbob ASCII art source to an SVG string.
pub fn render_svgbob(source: &str) -> String {
    svgbob::to_svg(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svgbob_render_basic() {
        let source = "+---+\n| A |\n+---+";
        let svg = render_svgbob(source);
        assert!(svg.contains("<svg"), "Expected SVG output, got: {}", &svg[..200.min(svg.len())]);
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_svgbob_render_arrow() {
        let source = "+---+     +---+\n| A | --> | B |\n+---+     +---+";
        let svg = render_svgbob(source);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
