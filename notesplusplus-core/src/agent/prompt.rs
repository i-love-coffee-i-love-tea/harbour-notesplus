//! AsciiDoc Grammar System Prompt and Preset Action Templates.

pub const DEFAULT_SYSTEM_PROMPT: &str = "\
You are an AI Assistant integrated into Notes++, an AsciiDoc note-taking application for Sailfish OS.\n\
Your job is to assist the user in drafting, editing, organizing, and analyzing notes.\n\n\
=== Communication & Response Guidelines ===\n\
1. Conversational Chat & Tool Summaries:\n\
   - In chat messages and answers to general questions, speak naturally, politely, and concisely.\n\
   - Keep chat responses mobile-friendly and readable as plain text.\n\
   - When you receive tool outputs (such as JSON from `list_notes`, `search_notes`, or raw text from `read_note`):\n\
     * NEVER output or echo raw JSON payloads, keys, or technical database structures to the user.\n\
     * Always synthesize the information into clear, natural bullet points or short conversational summaries.\n\
     * For example, when listing notes, present them as readable bullet items: `• Title (filename.adoc) - Updated info`.\n\
     * Conclude multi-step tool actions with a helpful, friendly summary of what was done.\n\n\
2. Note Content Rules for `create_note` and `edit_note` (MANDATORY AsciiDoc):\n\
   - When generating note content for the `content` parameter in `create_note` or `edit_note`, you MUST strictly produce valid AsciiDoc markup. NEVER use Markdown syntax.\n\
   - Headings: Use `= Document Title` (Level 0/1), `== Section` (Level 2), `=== Subsection` (Level 3), `==== Heading 4` (Level 4). Do NOT use `#` or `##`.\n\
   - Task Lists / Checklists: Use `* [ ] Task item` for unchecked and `* [x] Task item` for checked. Do NOT use `- [ ]`.\n\
   - Inline Formatting:\n\
     * Bold: `*text*` (do NOT use `**`)\n\
     * Italic: `_text_` (do NOT use `__`)\n\
     * Monospace / Code: `+text+` or `` `text` ``\n\
     * Strikethrough: `[line-through]#text#`\n\
   - Admonitions: Use `NOTE: text`, `TIP: text`, or `WARNING: text` or block syntax:\n\
     [TIP]\n\
     ====\n\
     Multi-line tip content\n\
     ====\n\
   - Tables:\n\
     |===\n\
     |Header 1 |Header 2\n\
     |Cell 1   |Cell 2\n\
     |===\n\
   - Code Blocks:\n\
     [source,language]\n\
     ----\n\
     code here\n\
     ----\n\
   - Quotes and Sidebars:\n\
     [quote, Author]\n\
     ____\n\
     Quote text\n\
     ____\n\
   - Cross references / Links: `https://example.com[Label]` or `xref:other_note.adoc[Note Title]`.\n\
   - Lists: Bulleted with `* item`, `** sub-item`; numbered with `. item`, `.. sub-item`.\n\n\
=== Available Note Tools ===\n\
You have tools to manage notes: `read_note`, `list_notes`, `search_notes`, `create_note`, `edit_note`, and `fetch_url`.\n\
- When asked to list notes or search notes, call `list_notes` or `search_notes`, then summarize the results in a friendly list.\n\
- When asked to read a note, use `read_note` and answer the user's questions based on its content.\n\
- When creating a note, use `create_note` with valid AsciiDoc formatted content.\n\
- When modifying an existing note, use `edit_note` with the complete updated AsciiDoc content and provide a clear one-sentence summary in `reason`.\n\
- When external URL text is provided or requested, use `fetch_url` to inspect and analyze it.\n";

/// Builds the system prompt enforcing strict AsciiDoc syntax and providing context.
pub fn build_system_prompt(
    active_note: Option<(&str, &str)>,
    extra_context: Option<&str>,
) -> String {
    build_system_prompt_with_custom(None, active_note, extra_context)
}

/// Builds the system prompt with an optional user-configured custom system prompt.
pub fn build_system_prompt_with_custom(
    custom_prompt: Option<&str>,
    active_note: Option<(&str, &str)>,
    extra_context: Option<&str>,
) -> String {
    let mut prompt = String::with_capacity(2048);

    if let Some(custom) = custom_prompt {
        if !custom.trim().is_empty() {
            prompt.push_str(custom.trim());
            prompt.push_str("\n\n");
        } else {
            prompt.push_str(DEFAULT_SYSTEM_PROMPT);
        }
    } else {
        prompt.push_str(DEFAULT_SYSTEM_PROMPT);
    }

    if let Some((filename, content)) = active_note {
        prompt.push_str(&format!(
            "\n=== Currently Active Note ===\nFilename: {}\nContent:\n{}\n=== End Active Note ===\n",
            filename, content
        ));
    }

    if let Some(extra) = extra_context {
        if !extra.trim().is_empty() {
            prompt.push_str(&format!(
                "\n=== Additional Context / Clipboard ===\n{}\n=== End Context ===\n",
                extra
            ));
        }
    }

    prompt
}

/// Returns a pre-formulated user prompt and instruction for preset action templates.
pub fn build_template_instruction(
    template_id: &str,
    user_input: &str,
    active_filename: Option<&str>,
    active_content: Option<&str>,
) -> String {
    build_template_instruction_ex(template_id, user_input, active_filename, active_content, None, None, None)
}

/// Extended version that accepts optional import parameters.
pub fn build_template_instruction_ex(
    template_id: &str,
    user_input: &str,
    active_filename: Option<&str>,
    active_content: Option<&str>,
    import_title: Option<&str>,
    import_mode: Option<&str>,
    import_custom: Option<&str>,
) -> String {
    let has_input = !user_input.trim().is_empty();
    let base_content = if has_input {
        user_input.trim()
    } else if let Some(content) = active_content {
        content.trim()
    } else {
        ""
    };

    match template_id {
        "expand_draft" => {
            if let (false, Some(fname)) = (has_input, active_filename) {
                format!(
                    "Please expand and draft the ideas in the active note '{}' into a well-structured AsciiDoc document with appropriate sections, headings, and detailed explanations. Call the `edit_note` tool with the complete expanded AsciiDoc content and filename.\n\nNote Content:\n{}",
                    fname, base_content
                )
            } else {
                format!(
                    "Please expand and draft the following ideas into a well-structured AsciiDoc document with appropriate sections, headings, and detailed explanations:\n\n{}",
                    base_content
                )
            }
        }
        "fix_grammar" => {
            if let (false, Some(fname)) = (has_input, active_filename) {
                format!(
                    "Please review and correct the spelling, grammar, punctuation, and formatting in the active note '{}' while strictly preserving and enforcing proper AsciiDoc syntax. Call the `edit_note` tool with the complete corrected AsciiDoc content and filename.\n\nNote Content:\n{}",
                    fname, base_content
                )
            } else {
                format!(
                    "Please review and correct the spelling, grammar, punctuation, and formatting in the following text while strictly preserving and enforcing proper AsciiDoc syntax:\n\n{}",
                    base_content
                )
            }
        }
        "beautify" => {
            if let (false, Some(fname)) = (has_input, active_filename) {
                format!(
                    "Please beautify the active note '{}' by adding visual structure, helpful admonition blocks (NOTE, TIP, WARNING), clean tables, and suitable emoji accents where appropriate. Call the `edit_note` tool with the complete beautified AsciiDoc content and filename.\n\nNote Content:\n{}",
                    fname, base_content
                )
            } else {
                format!(
                    "Please beautify the following AsciiDoc content by adding visual structure, helpful admonition blocks (NOTE, TIP, WARNING), clean tables, and suitable emoji accents where appropriate:\n\n{}",
                    base_content
                )
            }
        }
        "extract_todos" => {
            if let (false, Some(fname)) = (has_input, active_filename) {
                format!(
                    "Please analyze the active note '{}' and extract all actionable tasks and todo items into a clean AsciiDoc checklist using `* [ ]`.\n\nNote Content:\n{}",
                    fname, base_content
                )
            } else {
                format!(
                    "Please analyze the following text and extract all actionable tasks and todo items into a clean AsciiDoc checklist using `* [ ]`:\n\n{}",
                    base_content
                )
            }
        }
        "analyze_external" => {
            format!(
                "Please analyze the following external text or content, summarize key points, and extract relevant action items into structured AsciiDoc:\n\n{}",
                base_content
            )
        }
        "import_convert" => {
            let mode = import_mode.unwrap_or("convert_full");
            build_import_instruction(base_content, import_title, mode, import_custom)
        }
        _ => {
            if base_content.is_empty() {
                user_input.to_string()
            } else {
                format!("{}\n\nContent:\n{}", user_input, base_content)
            }
        }
    }
}

/// Builds a customized prompt from a user-defined custom instruction template,
/// combining it with user input, active note filename, and active note content.
pub fn build_custom_instruction(
    instruction_template: &str,
    user_input: &str,
    active_filename: Option<&str>,
    active_content: Option<&str>,
) -> String {
    let has_input = !user_input.trim().is_empty();
    let has_content = active_content.map(|c| !c.trim().is_empty()).unwrap_or(false);
    let base_content = if has_input && has_content {
        format!("{}\n\nNote Content:\n{}", user_input.trim(), active_content.unwrap().trim())
    } else if has_input {
        user_input.trim().to_string()
    } else if let Some(content) = active_content {
        content.trim().to_string()
    } else {
        String::new()
    };

    let template = instruction_template.trim();
    if template.is_empty() {
        return base_content;
    }

    let mut result = template.to_string();
    let mut replaced_placeholder = false;

    if result.contains("{filename}") {
        result = result.replace("{filename}", active_filename.unwrap_or("active note"));
        replaced_placeholder = true;
    }
    if result.contains("{input}") {
        result = result.replace("{input}", user_input.trim());
        replaced_placeholder = true;
    }
    if result.contains("{content}") {
        let cnt = active_content.unwrap_or("").trim();
        result = result.replace("{content}", cnt);
        replaced_placeholder = true;
    }
    if result.contains("{context}") {
        result = result.replace("{context}", &base_content);
        replaced_placeholder = true;
    }

    if !replaced_placeholder {
        if let (Some(fname), false) = (active_filename, base_content.is_empty()) {
            format!("{}\n\nActive Note: '{}'\nContent:\n{}", result, fname, base_content)
        } else if !base_content.is_empty() {
            format!("{}\n\nContent:\n{}", result, base_content)
        } else {
            result
        }
    } else {
        result
    }
}

/// Builds an import and conversion instruction for external text into an AsciiDoc note.
pub fn build_import_instruction(
    source_text: &str,
    target_title: Option<&str>,
    mode: &str,
    custom_instruction: Option<&str>,
) -> String {
    let mode_desc = match mode {
        "summarize" => "Summarize the key ideas and structure them logically with headings, bullet points, and tables in a clean AsciiDoc note.",
        "action_items" => "Analyze the text and extract all actionable tasks, checklist items (`* [ ]`), assignees, and deadlines into a structured AsciiDoc note.",
        _ => "Convert the entire source text faithfully into high-fidelity AsciiDoc markup, converting all headings, lists, checklists, tables, code blocks, and formatting into valid AsciiDoc.",
    };

    let title_instruction = match target_title {
        Some(t) if !t.trim().is_empty() => format!("Use '{}' as the note title for the document header (`= {}`) and create_note call.", t.trim(), t.trim()),
        _ => "Determine a concise, descriptive document title from the content and use it for the document header (`= Title`) and create_note call.".to_string(),
    };

    let custom = match custom_instruction {
        Some(c) if !c.trim().is_empty() => format!("\nAdditional User Instructions: {}\n", c.trim()),
        _ => String::new(),
    };

    format!(
        "Please import and convert the following external text into an AsciiDoc note.\n\n\
        === Conversion Goal ===\n\
        {}\n\
        {}\n\
        {}\n\
        === Strict AsciiDoc Rules ===\n\
        - Start with a Level 0/1 document title: `= Title`\n\
        - Use `== Section`, `=== Subsection` for headings (never `#` or `##`)\n\
        - Use `* [ ]` for unchecked task items and `* [x]` for checked items\n\
        - Use `|===` for tables, `[source,lang]----` for code blocks, and `NOTE:`, `TIP:`, `WARNING:` for admonitions\n\
        - Call the `create_note` tool with the chosen title and the complete converted AsciiDoc content.\n\
        - Conclude with a brief, friendly summary of what was imported.\n\n\
        === Source Text ===\n\
        {}\n\
        === End Source Text ===",
        mode_desc,
        title_instruction,
        custom,
        source_text.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_asciidoc_rules() {
        let prompt = build_system_prompt(None, None);
        assert!(prompt.contains("= Document Title"));
        assert!(prompt.contains("* [ ] Task item"));
        assert!(prompt.contains("NOTE:"));
        assert!(prompt.contains("|==="));
        assert!(prompt.contains("NEVER output or echo raw JSON"));
        assert!(prompt.contains("NEVER use Markdown syntax"));
    }

    #[test]
    fn test_system_prompt_with_active_note() {
        let prompt = build_system_prompt(Some(("todo.adoc", "= My Tasks\n* [ ] Buy milk")), None);
        assert!(prompt.contains("Filename: todo.adoc"));
        assert!(prompt.contains("= My Tasks"));
        assert!(prompt.contains("* [ ] Buy milk"));
    }

    #[test]
    fn test_system_prompt_with_extra_context() {
        let prompt = build_system_prompt(None, Some("Clipboard contents: check meeting link"));
        assert!(prompt.contains("=== Additional Context / Clipboard ==="));
        assert!(prompt.contains("check meeting link"));
    }

    #[test]
    fn test_custom_system_prompt() {
        let prompt = build_system_prompt_with_custom(
            Some("You are a specialized technical assistant focusing on rust documentation."),
            Some(("arch.adoc", "= Architecture")),
            None,
        );
        assert!(prompt.contains("You are a specialized technical assistant focusing on rust documentation."));
        assert!(prompt.contains("Filename: arch.adoc"));
        assert!(prompt.contains("= Architecture"));
    }

    #[test]
    fn test_template_instructions() {
        let extract = build_template_instruction("extract_todos", "Meeting notes: Alice will write doc", None, None);
        assert!(extract.contains("extract all actionable tasks"));
        assert!(extract.contains("Meeting notes: Alice will write doc"));
        assert!(extract.contains("* [ ]"));

        let beautify = build_template_instruction("beautify", "", Some("note.adoc"), Some("= Simple note\nSome text"));
        assert!(beautify.contains("beautify the active note 'note.adoc'"));
        assert!(beautify.contains("edit_note"));
        assert!(beautify.contains("= Simple note"));

        let import = build_template_instruction("import_convert", "# Markdown header\nSome markdown text", None, None);
        assert!(import.contains("import and convert the following external text"));
        assert!(import.contains("Markdown header"));
    }

    #[test]
    fn test_build_import_instruction_custom_title_and_mode() {
        let prompt = build_import_instruction(
            "Project planning meeting...",
            Some("Q4 Planning"),
            "action_items",
            Some("Keep it under 10 bullets"),
        );
        assert!(prompt.contains("Q4 Planning"));
        assert!(prompt.contains("extract all actionable tasks"));
        assert!(prompt.contains("Keep it under 10 bullets"));
        assert!(prompt.contains("Project planning meeting..."));
        assert!(prompt.contains("create_note"));
    }

    #[test]
    fn test_build_custom_instruction_with_placeholders() {
        let template = "Translate the following note ({filename}) into Spanish:\n{content}\nAdditional notes: {input}";
        let prompt = build_custom_instruction(
            template,
            "Formal tone",
            Some("my_note.adoc"),
            Some("= Hello\nThis is a test note.")
        );
        assert!(prompt.contains("my_note.adoc"));
        assert!(prompt.contains("= Hello\nThis is a test note."));
        assert!(prompt.contains("Formal tone"));
        assert!(prompt.contains("Translate the following note"));
    }

    #[test]
    fn test_build_custom_instruction_without_placeholders() {
        let template = "Summarize the key takeaways in 3 bullet points.";
        let prompt = build_custom_instruction(
            template,
            "",
            Some("notes.adoc"),
            Some("= Discussion\nItem 1\nItem 2")
        );
        assert!(prompt.contains("Summarize the key takeaways in 3 bullet points."));
        assert!(prompt.contains("Active Note: 'notes.adoc'"));
        assert!(prompt.contains("= Discussion\nItem 1\nItem 2"));
    }

    #[test]
    fn test_build_custom_instruction_with_user_input_and_content() {
        let template = "Compare and highlight differences.";
        let prompt = build_custom_instruction(
            template,
            "New proposal version",
            Some("current.adoc"),
            Some("Old version text")
        );
        assert!(prompt.contains("Compare and highlight differences."));
        assert!(prompt.contains("Active Note: 'current.adoc'"));
        assert!(prompt.contains("New proposal version"));
        assert!(prompt.contains("Old version text"));
    }
}
