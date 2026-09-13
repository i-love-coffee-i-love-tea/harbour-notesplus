use regex::Regex;

/// Preprocesses ASCII art source to quote text labels, annotations, file paths,
/// and filenames that contain characters svgbob would otherwise parse as drawing primitives
/// (such as `/` turning into diagonal slashes, `)` turning into arcs, `v` turning into arrowheads,
/// or `-` turning into line connectors).
pub fn preprocess_svgbob(source: &str) -> String {
    let mut result = Vec::new();
    let tree_line_re = Regex::new(r#"^(\s*(?:[|`+\\]\s*)*[+`|\\]--?\s+)(.*)$"#).unwrap();
    let token_re = Regex::new(concat!(
        r#"(?P<paren>\([^\)\r\n]*[a-zA-Z][^\)\r\n]*\))|"#,
        r#"(?P<slash_phrase>\b[a-zA-Z0-9_]+\s+/\s+[a-zA-Z0-9_]+\b)|"#,
        r#"(?P<path>[~a-zA-Z0-9_.+-]+/[a-zA-Z0-9_.+/+-]*)|"#,
        r#"(?P<plusplus>\b[a-zA-Z0-9_]+\+\+/?)|"#,
        r#"(?P<hyphen_file>\b[a-zA-Z0-9_]+-[a-zA-Z0-9_.-]+)|"#,
        r#"(?P<csv_file>\b[a-zA-Z0-9_.-]+\.[a-zA-Z0-9_-]*[vV]\b)"#
    )).unwrap();

    for line in source.lines() {
        if line.contains("# Legend:") {
            result.push(line.to_string());
            continue;
        }

        // 1. Check if line is a tree branch line (e.g. `+-- meeting-notes.adoc` or `|   +-- mockup.png`)
        if let Some(caps) = tree_line_re.captures(line) {
            let prefix = &caps[1];
            let rest = &caps[2];
            let mut processed_rest = String::new();

            // Check if rest contains parenthesized annotation like `(asset directory - hidden)`
            let parts: Vec<&str> = rest.split('"').collect();
            for (idx, part) in parts.iter().enumerate() {
                if idx % 2 == 1 {
                    processed_rest.push('"');
                    processed_rest.push_str(part);
                    processed_rest.push('"');
                } else {
                    let mut unquoted = part.to_string();
                    unquoted = token_re.replace_all(&unquoted, |c: &regex::Captures| {
                        let matched = c.get(0).unwrap().as_str();
                        if matched.chars().any(|ch| ch.is_alphabetic()) {
                            format!("\"{}\"", matched)
                        } else {
                            matched.to_string()
                        }
                    }).to_string();
                    processed_rest.push_str(&unquoted);
                }
            }
            result.push(format!("{}{}", prefix, processed_rest));
            continue;
        }

        // 2. Check if line is a standalone root directory/path line like `~/Documents/Notes++/`
        let trimmed = line.trim();
        if (trimmed.starts_with('~') || (trimmed.contains('/') && !trimmed.contains("+--")))
            && trimmed.chars().any(|c| c.is_alphabetic())
            && !trimmed.starts_with('"')
            && !trimmed.contains("+---")
            && !trimmed.contains("|")
        {
            let leading_spaces = line.len() - line.trim_start().len();
            result.push(format!("{}\"{}\"", &line[..leading_spaces], trimmed));
            continue;
        }

        // 3. General diagram line: process unquoted sections
        let mut out = String::new();
        let parts: Vec<&str> = line.split('"').collect();
        for (idx, part) in parts.iter().enumerate() {
            if idx % 2 == 1 {
                out.push('"');
                out.push_str(part);
                out.push('"');
            } else {
                let unquoted = token_re.replace_all(part, |c: &regex::Captures| {
                    let matched = c.get(0).unwrap().as_str();
                    if matched.chars().any(|ch| ch.is_alphabetic()) {
                        format!("\"{}\"", matched)
                    } else {
                        matched.to_string()
                    }
                }).to_string();
                out.push_str(&unquoted);
            }
        }
        result.push(out);
    }

    result.join("\n")
}

/// Render svgbob ASCII art source to an SVG string with inline presentation attributes
/// for compatibility with Qt's QSvgRenderer / SVG Tiny.
pub fn render_svgbob(source: &str) -> String {
    let preprocessed = preprocess_svgbob(source);
    let max_cols = preprocessed.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let num_rows = preprocessed.lines().count();
    let min_w = if max_cols > 0 { ((max_cols + 1) * 8) as f32 } else { 0.0 };
    let min_h = if num_rows > 0 { ((num_rows + 1) * 16) as f32 } else { 0.0 };

    let cb = svgbob::CellBuffer::from(preprocessed.as_str());
    let (_node, w, h): (svgbob::Node<()>, f32, f32) = cb.get_node_with_size(&svgbob::Settings::default());
    let final_w = w.max(min_w);
    let final_h = h.max(min_h);

    let raw_svg = svgbob::to_svg_with_override_size(preprocessed.as_str(), &svgbob::Settings::default(), final_w, final_h);
    inline_svg_styles(&raw_svg)
}

fn extract_class(attrs: &str) -> Option<&str> {
    if let Some(pos) = attrs.find("class=\"") {
        let rest = &attrs[pos + 7..];
        if let Some(end) = rest.find('"') {
            return Some(&rest[..end]);
        }
    }
    if let Some(pos) = attrs.find("class='") {
        let rest = &attrs[pos + 7..];
        if let Some(end) = rest.find('\'') {
            return Some(&rest[..end]);
        }
    }
    None
}

/// Post-processes SVG to add inline attributes (fill, stroke, marker, etc.) so that
/// SVG Tiny renderers like Qt's QSvgRenderer can display svgbob diagrams without
/// relying on CSS stylesheet classes.
pub fn inline_svg_styles(svg: &str) -> String {
    let tag_re = Regex::new(r"<([a-zA-Z]+)([^>]*)>").unwrap();
    tag_re.replace_all(svg, |caps: &regex::Captures| {
        let tag_name = &caps[1];
        let raw_attrs = &caps[2];
        let self_closing = raw_attrs.trim_end().ends_with('/');
        let attrs = if self_closing {
            raw_attrs.trim_end().trim_end_matches('/').trim_end()
        } else {
            raw_attrs
        };

        let class = extract_class(attrs).unwrap_or("");
        let mut extra = Vec::new();

        match tag_name {
            "rect" => {
                if class.contains("backdrop") {
                    if !attrs.contains("fill=") {
                        extra.push(r#"fill="white""#);
                    }
                    if !attrs.contains("stroke=") {
                        extra.push(r#"stroke="none""#);
                    }
                } else {
                    if !attrs.contains("stroke=") {
                        extra.push(r#"stroke="black""#);
                    }
                    if !attrs.contains("stroke-width=") {
                        extra.push(r#"stroke-width="2""#);
                    }
                    if !attrs.contains("stroke-linecap=") {
                        extra.push(r#"stroke-linecap="round""#);
                    }
                    if !attrs.contains("stroke-linejoin=") {
                        extra.push(r#"stroke-linejoin="miter""#);
                    }
                    if !attrs.contains("fill=") {
                        if class.contains("filled") {
                            extra.push(r#"fill="black""#);
                        } else if class.contains("nofill") || class.contains("bg_filled") {
                            extra.push(r#"fill="white""#);
                        } else {
                            extra.push(r#"fill="none""#);
                        }
                    }
                }
            }
            "line" => {
                if !attrs.contains("stroke=") {
                    extra.push(r#"stroke="black""#);
                }
                if !attrs.contains("stroke-width=") {
                    extra.push(r#"stroke-width="2""#);
                }
                if !attrs.contains("stroke-linecap=") {
                    extra.push(r#"stroke-linecap="round""#);
                }
                if !attrs.contains("stroke-linejoin=") {
                    extra.push(r#"stroke-linejoin="miter""#);
                }
                if !attrs.contains("fill=") {
                    extra.push(r#"fill="none""#);
                }
                if class.contains("broken") && !attrs.contains("stroke-dasharray=") {
                    extra.push(r#"stroke-dasharray="8""#);
                }
                apply_markers(class, attrs, &mut extra);
            }
            "path" => {
                if !attrs.contains("stroke=") {
                    extra.push(r#"stroke="black""#);
                }
                if !attrs.contains("stroke-width=") {
                    extra.push(r#"stroke-width="2""#);
                }
                if !attrs.contains("stroke-linecap=") {
                    extra.push(r#"stroke-linecap="round""#);
                }
                if !attrs.contains("stroke-linejoin=") {
                    extra.push(r#"stroke-linejoin="miter""#);
                }
                if !attrs.contains("fill=") {
                    if class.contains("filled") {
                        extra.push(r#"fill="black""#);
                    } else if class.contains("nofill") || class.contains("bg_filled") {
                        extra.push(r#"fill="white""#);
                    } else {
                        extra.push(r#"fill="none""#);
                    }
                }
                if class.contains("broken") && !attrs.contains("stroke-dasharray=") {
                    extra.push(r#"stroke-dasharray="8""#);
                }
                apply_markers(class, attrs, &mut extra);
            }
            "circle" => {
                if !attrs.contains("stroke=") {
                    extra.push(r#"stroke="black""#);
                }
                if !attrs.contains("stroke-width=") {
                    extra.push(r#"stroke-width="2""#);
                }
                if !attrs.contains("stroke-linecap=") {
                    extra.push(r#"stroke-linecap="round""#);
                }
                if !attrs.contains("stroke-linejoin=") {
                    extra.push(r#"stroke-linejoin="miter""#);
                }
                if !attrs.contains("fill=") {
                    if class.contains("filled") {
                        extra.push(r#"fill="black""#);
                    } else if class.contains("nofill") || class.contains("bg_filled") {
                        extra.push(r#"fill="white""#);
                    } else {
                        extra.push(r#"fill="none""#);
                    }
                }
            }
            "polygon" => {
                if !attrs.contains("stroke-width=") {
                    extra.push(r#"stroke-width="2""#);
                }
                if !attrs.contains("stroke-linecap=") {
                    extra.push(r#"stroke-linecap="round""#);
                }
                if !attrs.contains("stroke-linejoin=") {
                    extra.push(r#"stroke-linejoin="miter""#);
                }
                if !attrs.contains("fill=") {
                    if class.contains("nofill") || class.contains("bg_filled") {
                        extra.push(r#"fill="white""#);
                    } else {
                        extra.push(r#"fill="black""#);
                    }
                }
                if !attrs.contains("stroke=") {
                    if class.contains("nofill") || class.contains("bg_filled") {
                        extra.push(r#"stroke="black""#);
                    } else {
                        extra.push(r#"stroke="black""#);
                    }
                }
            }
            "text" => {
                if !attrs.contains("font-family=") {
                    extra.push(r#"font-family="monospace""#);
                }
                if !attrs.contains("font-size=") {
                    extra.push(r#"font-size="14px""#);
                }
                if !attrs.contains("fill=") {
                    extra.push(r#"fill="black""#);
                }
                if !attrs.contains("stroke=") {
                    extra.push(r#"stroke="none""#);
                }
            }
            _ => {}
        }

        if extra.is_empty() {
            caps[0].to_string()
        } else {
            let extra_str = extra.join(" ");
            if self_closing {
                format!("<{} {} {} />", tag_name, attrs.trim(), extra_str)
            } else if attrs.trim().is_empty() {
                format!("<{} {}>", tag_name, extra_str)
            } else {
                format!("<{} {} {}>", tag_name, attrs.trim(), extra_str)
            }
        }
    }).to_string()
}

fn apply_markers(class: &str, attrs: &str, extra: &mut Vec<&'static str>) {
    if class.contains("end_marked_arrow") && !attrs.contains("marker-end=") {
        extra.push(r#"marker-end="url(#arrow)""#);
    }
    if class.contains("start_marked_arrow") && !attrs.contains("marker-start=") {
        extra.push(r#"marker-start="url(#arrow)""#);
    }
    if class.contains("end_marked_diamond") && !attrs.contains("marker-end=") {
        extra.push(r#"marker-end="url(#diamond)""#);
    }
    if class.contains("start_marked_diamond") && !attrs.contains("marker-start=") {
        extra.push(r#"marker-start="url(#diamond)""#);
    }
    if class.contains("end_marked_circle") && !attrs.contains("marker-end=") {
        extra.push(r#"marker-end="url(#circle)""#);
    }
    if class.contains("start_marked_circle") && !attrs.contains("marker-start=") {
        extra.push(r#"marker-start="url(#circle)""#);
    }
    if class.contains("end_marked_open_circle") && !attrs.contains("marker-end=") {
        extra.push(r#"marker-end="url(#open_circle)""#);
    }
    if class.contains("start_marked_open_circle") && !attrs.contains("marker-start=") {
        extra.push(r#"marker-start="url(#open_circle)""#);
    }
    if class.contains("end_marked_big_open_circle") && !attrs.contains("marker-end=") {
        extra.push(r#"marker-end="url(#big_open_circle)""#);
    }
    if class.contains("start_marked_big_open_circle") && !attrs.contains("marker-start=") {
        extra.push(r#"marker-start="url(#big_open_circle)""#);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svgbob_render_basic() {
        let source = "+---+\n| A |\n+---+";
        let svg = render_svgbob(source);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(r#"fill="white""#));
        assert!(svg.contains(r#"stroke="black""#));
        assert!(svg.contains(r#"font-family="monospace""#));
    }

    #[test]
    fn test_svgbob_all_shapes() {
        let source = r#"
  +---+    *---*    (---)
  | A |    | B |    | C |
  +---+    *---*    (---)

  -->  <--  <-->  ==>  <==
  - - -  . . .  ===
  (O)  (o)  (*)  ( )
  /\
 /  \
 \  /
  \/
"#;
        let svg = render_svgbob(source);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(r#"fill="white""#));
        assert!(svg.contains(r#"stroke="black""#));
        assert!(svg.contains(r#"fill="none""#));
    }

    #[test]
    fn test_svgbob_doc_example() {
        let source = r#"    +----------+       +----------+
    |  Notes++ |       |  Browser |
    |  on phone|       |  on PC   |
    +----+-----+       +----+-----+
         |                  |
         |    HTTPS / TLS   |
         +-------> <--------+
                   |
            +------+------+
            |  Embedded   |
            |  Web Server |
            +-------------+"#;
        let svg = render_svgbob(source);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(r#"fill="white""#));
        assert!(svg.contains(r#"fill="none""#));
        assert!(svg.contains(r#"stroke="black""#));
        assert!(svg.contains(r#"font-family="monospace""#));
    }

    #[test]
    fn test_tree_diagram() {
        let unquoted = r#"~/Documents/Notes++/
|
+-- meeting-notes.adoc
+-- project-plan.adoc
+-- project-plan/              (asset directory - hidden)
|   +-- mockup.png
|   +-- data.csv
+-- Work/                      (group)
|   +-- report.adoc
|   +-- Ideas/                 (subgroup)
|       +-- brainstorm.adoc
+-- Personal/                  (group)
    +-- journal.adoc
    +-- recipes.adoc"#;
        let svg = render_svgbob(unquoted);
        println!("=== UNQUOTED FULL SVG ===\n{}", svg);
        assert!(svg.contains("~/Documents/Notes++/"));
        assert!(svg.contains("meeting-notes.adoc"));
        assert!(svg.contains("project-plan/"));
        assert!(svg.contains("(asset directory - hidden)"));
        assert!(svg.contains("mockup.png"));
        assert!(svg.contains("data.csv"));
        assert!(svg.contains("Work/"));
        assert!(svg.contains("(group)"));
        assert!(svg.contains("report.adoc"));
        assert!(svg.contains("Ideas/"));
        assert!(svg.contains("(subgroup)"));
        assert!(svg.contains("brainstorm.adoc"));
        assert!(svg.contains("Personal/"));
        assert!(svg.contains("journal.adoc"));
        assert!(svg.contains("recipes.adoc"));
        // Ensure no overlapping slash lines over Documents/
        assert!(!svg.contains(r#"<line x1="96" y1="0" x2="88" y2="16""#));
        // Ensure no arc paths over (asset directory - hidden)
        assert!(!svg.contains(r#"A 16,16 0,0,0 256,80"#));

        let quoted = r#""~/Documents/Notes++/"
 |
 +-- "meeting-notes.adoc"
 +-- "project-plan.adoc"
 +-- "project-plan/"               "(asset directory - hidden)"
 |    |
 |    +-- "mockup.png"
 |    `-- "data.csv"
 |
 +-- "Work/"                       "(group)"
 |    |
 |    +-- "report.adoc"
 |    `-- "Ideas/"                 "(subgroup)"
 |         |
 |         `-- "brainstorm.adoc"
 |
 `-- "Personal/"                   "(group)"
      |
      +-- "journal.adoc"
      `-- "recipes.adoc""#;
        let svg_q = render_svgbob(quoted);
        println!("=== QUOTED FULL SVG ===\n{}", svg_q);
        assert!(svg_q.contains("~/Documents/Notes++/"));
        assert!(svg_q.contains("meeting-notes.adoc"));
    }
}
