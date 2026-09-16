//! AsciiDoc cross-reference rewriting for page moves and renames.

use std::sync::LazyLock;
use regex::Regex;

static XREF_STD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"xref:([^\s#\[]+)([#][^\]]*)?\[([^\]]*)\]").unwrap()
});

static XREF_ANGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<<([^\s#,>]+)([#][^,>]*)?(,[^>]*)?>>").unwrap()
});

/// Rewrites cross-references in AsciiDoc content from old_target to new_target.
/// Handles:
/// - Standard: `xref:old_target#anchor[label]`, `xref:old_target[label]`, `xref:old_target[]`
/// - Standard without .adoc: `xref:old_target_stem#anchor[label]`
/// - Shorthand angle-brackets: `<<old_target#anchor,label>>`, `<<old_target,label>>`, `<<old_target>>`
/// - Shorthand angle-brackets without .adoc: `<<old_target_stem#anchor,label>>`, `<<old_target_stem>>`
pub fn rewrite_xrefs(content: &str, old_target: &str, new_target: &str) -> String {
    let old_clean = old_target.trim().trim_matches('/');
    let new_clean = new_target.trim().trim_matches('/');

    if old_clean.is_empty() || new_clean.is_empty() || old_clean == new_clean {
        return content.to_string();
    }

    let old_stem = old_clean.strip_suffix(".adoc").unwrap_or(old_clean);
    let new_stem = new_clean.strip_suffix(".adoc").unwrap_or(new_clean);
    let old_adoc = if old_clean.ends_with(".adoc") { old_clean.to_string() } else { format!("{}.adoc", old_clean) };
    let new_adoc = if new_clean.ends_with(".adoc") { new_clean.to_string() } else { format!("{}.adoc", new_clean) };

    // 1. Standard xref:target#anchor[label] or xref:target[label]
    let result = XREF_STD_RE.replace_all(content, |caps: &regex::Captures| {
        let target = &caps[1];
        let anchor = caps.get(2).map_or("", |m| m.as_str());
        let label = &caps[3];

        if target == old_adoc || target == old_stem {
            format!("xref:{}{}[{}]", new_adoc, anchor, label)
        } else {
            caps.get(0).unwrap().as_str().to_string()
        }
    });

    // 2. Shorthand <<target#anchor,label>> or <<target>>
    let result = XREF_ANGLE_RE.replace_all(&result, |caps: &regex::Captures| {
        let target = &caps[1];
        let anchor = caps.get(2).map_or("", |m| m.as_str());
        let label = caps.get(3).map_or("", |m| m.as_str());

        if target == old_adoc {
            format!("<<{}{}{}>>", new_adoc, anchor, label)
        } else if target == old_stem {
            format!("<<{}{}{}>>", new_stem, anchor, label)
        } else {
            caps.get(0).unwrap().as_str().to_string()
        }
    });

    result.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_xref_with_anchor_and_label() {
        let content = "See xref:OldNote.adoc#section-two[Custom Label] for info.";
        let result = rewrite_xrefs(content, "OldNote.adoc", "Work/NewNote.adoc");
        assert_eq!(result, "See xref:Work/NewNote.adoc#section-two[Custom Label] for info.");
    }

    #[test]
    fn standard_xref_without_anchor() {
        let content = "See xref:OldNote.adoc[Link] here.";
        let result = rewrite_xrefs(content, "OldNote.adoc", "NewNote.adoc");
        assert_eq!(result, "See xref:NewNote.adoc[Link] here.");
    }

    #[test]
    fn standard_xref_stem_without_extension() {
        let content = "See xref:OldNote[label] here.";
        let result = rewrite_xrefs(content, "OldNote.adoc", "NewNote.adoc");
        assert_eq!(result, "See xref:NewNote.adoc[label] here.");
    }

    #[test]
    fn angle_bracket_shorthand_with_anchor_and_label() {
        let content = "<<OldNote#section-two,Custom Label>> and <<OldNote>>";
        let result = rewrite_xrefs(content, "OldNote.adoc", "Work/NewNote.adoc");
        assert_eq!(result, "<<Work/NewNote#section-two,Custom Label>> and <<Work/NewNote>>");
    }

    #[test]
    fn angle_bracket_shorthand_without_label() {
        let content = "<<OldNote>>";
        let result = rewrite_xrefs(content, "OldNote.adoc", "NewNote.adoc");
        assert_eq!(result, "<<NewNote>>");
    }

    #[test]
    fn no_rewriting_when_old_equals_new() {
        let content = "xref:same.adoc[Same]";
        let result = rewrite_xrefs(content, "same.adoc", "same.adoc");
        assert_eq!(result, content);
    }

    #[test]
    fn no_rewriting_when_old_is_empty() {
        let content = "xref:any.adoc[Any]";
        let result = rewrite_xrefs(content, "", "new.adoc");
        assert_eq!(result, content);
    }

    #[test]
    fn no_rewriting_when_new_is_empty() {
        let content = "xref:old.adoc[Old]";
        let result = rewrite_xrefs(content, "old.adoc", "");
        assert_eq!(result, content);
    }

    #[test]
    fn multiple_xrefs_in_same_document() {
        let content = "See xref:Target.adoc[One] and xref:Target.adoc#anchor[Two]";
        let result = rewrite_xrefs(content, "Target.adoc", "New/Target.adoc");
        assert!(result.contains("xref:New/Target.adoc[One]"));
        assert!(result.contains("xref:New/Target.adoc#anchor[Two]"));
    }

    #[test]
    fn group_path_change_xrefs() {
        // Simulate moving from group "Old" to group "New"
        let content = "Link to <<Sibling#section>> here.";
        let result = rewrite_xrefs(content, "Sibling.adoc", "New/Sibling.adoc");
        assert_eq!(result, "Link to <<New/Sibling#section>> here.");
    }

    #[test]
    fn preserves_unrelated_xrefs() {
        let content = "xref:Other.adoc[Other] and xref:Old.adoc[Old]";
        let result = rewrite_xrefs(content, "Old.adoc", "New.adoc");
        assert!(result.contains("xref:Other.adoc[Other]"), "unrelated xref must be preserved");
        assert!(result.contains("xref:New.adoc[Old]"), "target xref must be rewritten");
    }
}
