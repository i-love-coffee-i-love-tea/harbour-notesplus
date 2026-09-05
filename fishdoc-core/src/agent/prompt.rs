//! AsciiDoc Grammar System Prompt and Preset Action Templates.

/// Builds the system prompt enforcing strict AsciiDoc syntax and providing context.
pub fn build_system_prompt(
    active_note: Option<(&str, &str)>,
    extra_context: Option<&str>,
) -> String {
    let mut prompt = String::with_capacity(2048);

    prompt.push_str(
        "You are an AI Assistant integrated into FishDoc, an AsciiDoc note-taking application for Sailfish OS.\n\
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
           - Admonitions: Use `NOTE: text`, `TIP: text`, `IMPORTANT: text`, `WARNING: text`, `CAUTION: text` or block syntax:\n\
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
        - When external URL text is provided or requested, use `fetch_url` to inspect and analyze it.\n\
        "
    );

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
    active_content: Option<&str>,
) -> String {
    let base_content = if !user_input.trim().is_empty() {
        user_input.trim()
    } else if let Some(content) = active_content {
        content.trim()
    } else {
        ""
    };

    match template_id {
        "expand_draft" => {
            format!(
                "Please expand and draft the following ideas into a well-structured AsciiDoc document with appropriate sections, headings, and detailed explanations:\n\n{}",
                base_content
            )
        }
        "fix_grammar" => {
            format!(
                "Please review and correct the spelling, grammar, punctuation, and formatting in the following text while strictly preserving and enforcing proper AsciiDoc syntax:\n\n{}",
                base_content
            )
        }
        "beautify" => {
            format!(
                "Please beautify the following AsciiDoc content by adding visual structure, helpful admonition blocks (TIP, NOTE, IMPORTANT), clean tables, and suitable emoji accents where appropriate:\n\n{}",
                base_content
            )
        }
        "extract_todos" => {
            format!(
                "Please analyze the following text and extract all actionable tasks and todo items into a clean AsciiDoc checklist using `* [ ]`:\n\n{}",
                base_content
            )
        }
        "analyze_external" => {
            format!(
                "Please analyze the following external text or content, summarize key points, and extract relevant action items into structured AsciiDoc:\n\n{}",
                base_content
            )
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
    fn test_template_instructions() {
        let extract = build_template_instruction("extract_todos", "Meeting notes: Alice will write doc", None);
        assert!(extract.contains("extract all actionable tasks"));
        assert!(extract.contains("Meeting notes: Alice will write doc"));
        assert!(extract.contains("* [ ]"));

        let beautify = build_template_instruction("beautify", "", Some("= Simple note\nSome text"));
        assert!(beautify.contains("beautify the following AsciiDoc"));
        assert!(beautify.contains("= Simple note"));
    }
}
