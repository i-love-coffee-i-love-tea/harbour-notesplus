//! Tool specifications and JSON Schema definitions for LLM function calling.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", default = "default_tool_type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

fn default_tool_type() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

/// Returns the standard list of tool definitions compatible with Ollama and OpenAI/MiMoCode APIs.
pub fn get_available_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "read_note".to_string(),
                description: "Read the full raw AsciiDoc content of a note given its filename.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "Name of the note file (e.g. meeting.adoc or journal.adoc)"
                        }
                    },
                    "required": ["filename"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_notes".to_string(),
                description: "List all existing notes in the library with their filenames, titles, and updated timestamps.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_notes".to_string(),
                description: "Search notes by title and content using full-text search keywords.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Keywords or search phrases to look for in notes"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "create_note".to_string(),
                description: "Create a new note with a title and AsciiDoc formatted content.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Title of the new note (used for heading and filename derivation)"
                        },
                        "content": {
                            "type": "string",
                            "description": "The complete AsciiDoc content for the new note"
                        }
                    },
                    "required": ["title", "content"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "edit_note".to_string(),
                description: "Update or replace the content of an existing note. Requires user confirmation before applying.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "The filename of the existing note to update"
                        },
                        "content": {
                            "type": "string",
                            "description": "The complete updated AsciiDoc content of the note"
                        },
                        "reason": {
                            "type": "string",
                            "description": "A concise explanation of the changes made"
                        }
                    },
                    "required": ["filename", "content", "reason"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "fetch_url".to_string(),
                description: "Download external text or webpage content from an HTTP/HTTPS URL for analysis.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The full HTTP or HTTPS URL to fetch"
                        }
                    },
                    "required": ["url"]
                }),
            },
        },
    ]
}

/// Helper to download web text from an HTTP/HTTPS URL.
pub fn fetch_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let target = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };
    let blocked = ["localhost", "127.0.0.1", "0.0.0.0", "169.254.169.254", "::1"];
    if blocked.iter().any(|h| target.contains(h)) {
        return Err(format!("URL '{}' blocked: fetching localhost/private addresses is not allowed", target));
    }
    match ureq::get(&target).timeout(std::time::Duration::from_secs(15)).call() {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            if text.len() > 15000 {
                let mut end = 15000;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                Ok(format!("{}... [truncated]", &text[..end]))
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("Failed to fetch URL '{}': {}", target, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_schema_validity() {
        let tools = get_available_tools();
        assert_eq!(tools.len(), 6);

        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"read_note".to_string()));
        assert!(names.contains(&"list_notes".to_string()));
        assert!(names.contains(&"search_notes".to_string()));
        assert!(names.contains(&"create_note".to_string()));
        assert!(names.contains(&"edit_note".to_string()));
        assert!(names.contains(&"fetch_url".to_string()));

        let serialized = serde_json::to_string(&tools).expect("Must serialize tools");
        assert!(serialized.contains("read_note"));
        assert!(serialized.contains("parameters"));
    }

    #[test]
    fn test_parse_tool_call() {
        let json_data = json!({
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "edit_note",
                "arguments": {
                    "filename": "meeting.adoc",
                    "content": "= Meeting\nUpdated",
                    "reason": "Added summary"
                }
            }
        });

        let tool_call: ToolCall = serde_json::from_value(json_data).unwrap();
        assert_eq!(tool_call.function.name, "edit_note");
        assert_eq!(tool_call.function.arguments["filename"], "meeting.adoc");
    }
}
