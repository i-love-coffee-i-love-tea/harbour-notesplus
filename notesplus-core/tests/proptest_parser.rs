use proptest::prelude::*;
use notesplus_core::parser::{parse_blocks, parse_blocks_with_options, toggle_checkbox, blocks_to_adoc};
use notesplus_core::html::format_code_for_qml_richtext;
use notesplus_core::page::{sanitize_note_path, extract_doc_title};

proptest! {
    #[test]
    fn parser_never_panics_on_arbitrary_input(s in "\\PC*") {
        let _ = parse_blocks(&s);
        let _ = parse_blocks_with_options(&s, false);
        let _ = parse_blocks_with_options(&s, true);
    }

    #[test]
    fn silica_adapter_never_panics_on_arbitrary_html(s in "\\PC*") {
        let _ = format_code_for_qml_richtext(&s);
    }

    #[test]
    fn path_sanitization_never_panics_on_arbitrary_input(s in "\\PC*") {
        let (group, filename) = sanitize_note_path(&s);
        prop_assert!(filename.ends_with(".adoc"));
        prop_assert!(!group.starts_with('/'));
        prop_assert!(!group.ends_with('/'));
    }

    #[test]
    fn doc_title_extraction_never_panics(content in "\\PC*", fallback in "\\PC*") {
        let title = extract_doc_title(&content, &fallback);
        prop_assert!(!title.is_empty() || fallback.is_empty());
    }

    #[test]
    fn checkbox_toggle_idempotency_on_single_item(
        prefix in "[ \t]{0,4}",
        marker in "[*\\-]",
        checked in proptest::bool::ANY,
        text in "[a-zA-Z0-9 ]{1,30}"
    ) {
        let check_char = if checked { 'x' } else { ' ' };
        let original = format!("{}{}[{}] {}\n", prefix, marker, check_char, text);

        if let Some(toggled) = toggle_checkbox(&original, 0, "") {
            prop_assert_ne!(&original, &toggled);
            if let Some(double_toggled) = toggle_checkbox(&toggled, 0, "") {
                prop_assert_eq!(&original, &double_toggled);
            }
        }
    }

    #[test]
    fn blocks_to_adoc_roundtrip_never_panics(s in "\\PC{0,200}") {
        let blocks = parse_blocks(&s);
        let adoc = blocks_to_adoc(&blocks);
        let _ = parse_blocks(&adoc);
    }
}
