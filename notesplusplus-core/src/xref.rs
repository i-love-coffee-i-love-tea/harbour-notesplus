//! AsciiDoc cross-reference rewriting for page moves and renames.

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

    let mut result = content.to_string();

    // 1. Standard xref:old_adoc#anchor[label] or xref:old_adoc[label]
    if let Ok(re_std) = regex::Regex::new(&format!(r"xref:{}([#][^\]]*)?\[", regex::escape(&old_adoc))) {
        result = re_std.replace_all(&result, format!("xref:{}$1[", new_adoc).as_str()).to_string();
    }

    // 2. Standard xref:old_stem#anchor[label] or xref:old_stem[label]
    if old_stem != old_adoc {
        if let Ok(re_std_stem) = regex::Regex::new(&format!(r"xref:{}([#][^\]]*)?\[", regex::escape(old_stem))) {
            result = re_std_stem.replace_all(&result, format!("xref:{}$1[", new_adoc).as_str()).to_string();
        }
    }

    // 3. Shorthand <<old_adoc#anchor,label>> or <<old_adoc>>
    if let Ok(re_angle) = regex::Regex::new(&format!(r"<<{}([#][^,>]*)?(,[^>]*)?>>", regex::escape(&old_adoc))) {
        result = re_angle.replace_all(&result, format!("<<{}$1$2>>", new_adoc).as_str()).to_string();
    }

    // 4. Shorthand <<old_stem#anchor,label>> or <<old_stem>>
    if let Ok(re_angle_stem) = regex::Regex::new(&format!(r"<<{}([#][^,>]*)?(,[^>]*)?>>", regex::escape(old_stem))) {
        result = re_angle_stem.replace_all(&result, format!("<<{}$1$2>>", new_stem).as_str()).to_string();
    }

    result
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
