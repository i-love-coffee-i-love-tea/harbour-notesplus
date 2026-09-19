//! Tool specifications and JSON Schema definitions for LLM function calling.

use std::net::ToSocketAddrs;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::NotesError;
pub use crate::net::{is_blocked_ip, parse_direct_ip};

/// Canonical tool name constants — use these everywhere instead of string literals.
pub const TOOL_READ_NOTE: &str = "read_note";
pub const TOOL_LIST_NOTES: &str = "list_notes";
pub const TOOL_SEARCH_NOTES: &str = "search_notes";
pub const TOOL_RETRIEVE_CONTEXT: &str = "retrieve_context";
pub const TOOL_LIST_GROUPS: &str = "list_groups";
pub const TOOL_MOVE_NOTE: &str = "move_note";
pub const TOOL_CREATE_NOTE: &str = "create_note";
pub const TOOL_EDIT_NOTE: &str = "edit_note";
pub const TOOL_EDIT_SECTION: &str = "edit_section";
pub const TOOL_APPEND_TO_NOTE: &str = "append_to_note";
pub const TOOL_INSERT_SECTION: &str = "insert_section";
pub const TOOL_FETCH_URL: &str = "fetch_url";

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
                name: TOOL_READ_NOTE.to_string(),
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
                name: TOOL_LIST_NOTES.to_string(),
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
                name: TOOL_SEARCH_NOTES.to_string(),
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
                name: TOOL_RETRIEVE_CONTEXT.to_string(),
                description: "Retrieve relevant note snippets and excerpts from the local SQLite FTS5 index to answer questions across notes.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query or topics to find relevant notes for"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of notes to retrieve snippets from (default: 5)"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_LIST_GROUPS.to_string(),
                description: "List all existing note groups and folders with their path, display name, and note count.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_MOVE_NOTE.to_string(),
                description: "Move an existing note to a target group/folder (e.g. 'Work/Projects' or '' for root/top-level). Automatically creates the target group if needed and rewrites cross-references. Requires user confirmation.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "Filename or path of the note to move (e.g. 'meeting.adoc' or 'Work/meeting.adoc')"
                        },
                        "target_group": {
                            "type": "string",
                            "description": "Destination group path (e.g. 'Archive', 'Work/Projects', or '' for root library level)"
                        },
                        "reason": {
                            "type": "string",
                            "description": "A concise explanation of why the note is being moved"
                        }
                    },
                    "required": ["filename", "target_group"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_CREATE_NOTE.to_string(),
                description: "Create a new note with a title and AsciiDoc formatted content in an optional group.".to_string(),
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
                        },
                        "group": {
                            "type": "string",
                            "description": "Optional destination group/folder path (e.g. 'Work' or 'Projects/App'; empty for root)"
                        }
                    },
                    "required": ["title", "content"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_EDIT_NOTE.to_string(),
                description: "Update or replace the entire content of an existing note. Requires user confirmation before applying.".to_string(),
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
                name: TOOL_EDIT_SECTION.to_string(),
                description: "Modify or replace a specific section of an existing note by its heading text, preserving the rest of the document. Requires user confirmation before applying.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "The filename of the note to modify"
                        },
                        "heading": {
                            "type": "string",
                            "description": "The heading title of the section to replace (e.g. 'Overview' or 'Action Items')"
                        },
                        "content": {
                            "type": "string",
                            "description": "The updated AsciiDoc content for this section"
                        },
                        "new_heading": {
                            "type": "string",
                            "description": "Optional new heading title if renaming the section"
                        },
                        "reason": {
                            "type": "string",
                            "description": "A concise explanation of the section edit"
                        }
                    },
                    "required": ["filename", "heading", "content"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_APPEND_TO_NOTE.to_string(),
                description: "Append text, checklist items, or notes to the end of a document or to the end of a specific named section. Requires user confirmation before applying.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "The filename of the note to append to"
                        },
                        "content": {
                            "type": "string",
                            "description": "The AsciiDoc content or checklist items to append"
                        },
                        "heading": {
                            "type": "string",
                            "description": "Optional section heading to append under; if omitted, appends to the end of the note"
                        },
                        "reason": {
                            "type": "string",
                            "description": "A concise explanation of what is being appended"
                        }
                    },
                    "required": ["filename", "content"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_INSERT_SECTION.to_string(),
                description: "Insert a new AsciiDoc section before/after a target heading, at the top, or at the bottom. Requires user confirmation before applying.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string",
                            "description": "The filename of the note to insert a section into"
                        },
                        "title": {
                            "type": "string",
                            "description": "Title of the new section heading"
                        },
                        "level": {
                            "type": "integer",
                            "description": "Heading level (1 to 5, default 2 for '==')"
                        },
                        "content": {
                            "type": "string",
                            "description": "AsciiDoc content of the new section"
                        },
                        "position": {
                            "type": "string",
                            "description": "Insertion position: 'after_heading', 'before_heading', 'top', or 'bottom' (default: 'after_heading')"
                        },
                        "target_heading": {
                            "type": "string",
                            "description": "Target section heading when using 'after_heading' or 'before_heading'"
                        },
                        "reason": {
                            "type": "string",
                            "description": "A concise explanation of the section insertion"
                        }
                    },
                    "required": ["filename", "title", "content"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_FETCH_URL.to_string(),
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

/// Parses target host, port, and scheme from an HTTP/HTTPS URL string.
pub fn parse_target_host_port(url_str: &str) -> Result<(String, u16, String), NotesError> {
    let trimmed = url_str.trim();
    let (scheme, rest) = if let Some(s) = trimmed.strip_prefix("http://") {
        ("http", s)
    } else if let Some(s) = trimmed.strip_prefix("https://") {
        ("https", s)
    } else if !trimmed.contains("://") {
        ("https", trimmed)
    } else {
        return Err(NotesError::Msg(format!("Unsupported URL scheme in '{}'", trimmed)));
    };

    let host_port_part = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host_port_part.is_empty() {
        return Err(NotesError::Msg("Missing host in URL".to_string()));
    }

    let default_port = if scheme == "https" { 443 } else { 80 };

    if host_port_part.starts_with('[') {
        if let Some(closing_bracket) = host_port_part.find(']') {
            let ip6_str = &host_port_part[1..closing_bracket];
            let after_bracket = &host_port_part[closing_bracket + 1..];
            let port = if let Some(colon) = after_bracket.strip_prefix(':') {
                colon.parse::<u16>().map_err(|_| NotesError::Msg("Invalid port".to_string()))?
            } else {
                default_port
            };
            return Ok((ip6_str.to_string(), port, scheme.to_string()));
        } else {
            return Err(NotesError::Msg("Malformed IPv6 host literal".to_string()));
        }
    }

    let (host, port) = if let Some((h, p)) = host_port_part.split_once(':') {
        let parsed_port = p.parse::<u16>().map_err(|_| NotesError::Msg("Invalid port".to_string()))?;
        (h, parsed_port)
    } else {
        (host_port_part, default_port)
    };

    Ok((host.to_string(), port, scheme.to_string()))
}

/// Returns true only if the URL resolves to a verified public (non-private) host.
/// Default-deny: parse failures, DNS failures, and any resolved private/loopback IP block the request.
pub fn is_allowed_host(url: &str) -> bool {
    let (host, port, _) = match parse_target_host_port(url) {
        Ok(res) => res,
        Err(_) => return false,
    };

    let lower_host = host.to_lowercase();
    if lower_host == "localhost"
        || lower_host.ends_with(".localhost")
        || lower_host == "localtest.me"
        || lower_host.ends_with(".localtest.me")
        || lower_host.ends_with(".local")
        || lower_host.ends_with(".internal")
        || lower_host.ends_with(".lan")
    {
        return false;
    }

    if let Some(ip) = parse_direct_ip(&host) {
        return !is_blocked_ip(&ip);
    }

    let socket_addr_str = format!("{}:{}", host, port);
    match socket_addr_str.to_socket_addrs() {
        Ok(addrs) => {
            let mut found_any = false;
            for addr in addrs {
                found_any = true;
                if is_blocked_ip(&addr.ip()) {
                    return false;
                }
            }
            // DNS resolved but returned no addresses — deny
            if !found_any {
                return false;
            }
            true
        }
        // DNS resolution failed — deny
        Err(_) => false,
    }
}

/// Resolves a host:port, verifies all IPs are safe, and returns the pinned addresses.
/// This is used to prevent DNS rebinding by resolving once and pinning the result.
fn resolve_and_verify(host: &str, port: u16) -> Result<Vec<std::net::SocketAddr>, NotesError> {
    let socket_addr_str = format!("{}:{}", host, port);
    let addrs: Vec<std::net::SocketAddr> = socket_addr_str
        .to_socket_addrs()
        .map_err(|e| NotesError::Http(format!("DNS resolution failed for '{}': {}", host, e)))?
        .collect();

    if addrs.is_empty() {
        return Err(NotesError::Http(format!("DNS resolution returned no addresses for '{}'", host)));
    }
    for addr in &addrs {
        if is_blocked_ip(&addr.ip()) {
            return Err(NotesError::Http(format!(
                "URL '{}' blocked: resolved to private/reserved IP {}",
                host, addr.ip()
            )));
        }
    }
    Ok(addrs)
}

/// Helper to download web text from an HTTP/HTTPS URL with SSRF mitigation and DNS rebinding protection.
pub fn fetch_url(url: &str) -> Result<String, NotesError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let mut current_url = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };

    const MAX_REDIRECTS: usize = 3;
    let mut redirects_followed = 0;

    let resp = loop {
        if !is_allowed_host(&current_url) {
            return Err(NotesError::Http(format!(
                "URL '{}' blocked: only verified public hosts are allowed",
                current_url
            )));
        }

        // Resolve DNS once and pin the result to prevent DNS rebinding
        let (host, port, _scheme) = parse_target_host_port(&current_url)?;
        let pinned_addrs = resolve_and_verify(&host, port)?;
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(std::time::Duration::from_secs(crate::constants::FETCH_URL_TIMEOUT_SECS))
            .resolver(move |_netloc: &str| Ok(pinned_addrs.clone()))
            .build();

        match agent.get(&current_url).call() {
            Ok(r) => break r,
            Err(ureq::Error::Status(code, r))
                if (301..=308).contains(&code) && redirects_followed < MAX_REDIRECTS =>
            {
                if let Some(location) = r.header("Location") {
                    let next_url = if location.starts_with("http://") || location.starts_with("https://") {
                        location.to_string()
                    } else if location.starts_with('/') {
                        let (host, port, scheme) = parse_target_host_port(&current_url)?;
                        let default_port = if scheme == "https" { 443 } else { 80 };
                        if port == default_port {
                            format!("{}://{}{}", scheme, host, location)
                        } else {
                            format!("{}://{}:{}{}", scheme, host, port, location)
                        }
                    } else {
                        return Err(NotesError::Http(format!("Unsupported relative redirect to '{}'", location)));
                    };

                    current_url = next_url;
                    redirects_followed += 1;
                    continue;
                } else {
                    return Err(NotesError::Http(format!("Redirect status {} with missing Location header", code)));
                }
            }
            Err(e) => return Err(NotesError::Http(format!("Failed to fetch URL '{}': {}", current_url, e))),
        }
    };

    let content_type = resp.header("Content-Type").unwrap_or("").to_lowercase();
    use std::io::Read;
    const MAX_RAW_FETCH_BYTES: u64 = 2 * 1024 * 1024; // 2 MB limit to prevent memory exhaustion
    let mut raw_bytes = Vec::new();
    let mut reader = resp.into_reader().take(MAX_RAW_FETCH_BYTES);
    if let Err(e) = reader.read_to_end(&mut raw_bytes) {
        log::warn!("Partial read from URL '{}': {}", url, e);
    }
    let raw_text = String::from_utf8_lossy(&raw_bytes).into_owned();

    let processed = if content_type.contains("html") || crate::html::preprocess::looks_like_html(&raw_text) {
        crate::html::preprocess_html(&raw_text)
    } else {
        raw_text
    };

    const MAX_FETCH_CHARS: usize = 250_000;
    if processed.len() > MAX_FETCH_CHARS {
        let mut end = MAX_FETCH_CHARS;
        while end > 0 && !processed.is_char_boundary(end) {
            end -= 1;
        }
        if let Some(last_nl) = processed[..end].rfind('\n') {
            if last_nl > MAX_FETCH_CHARS - crate::constants::FETCH_URL_TAIL_RESERVED {
                end = last_nl;
            }
        }
        Ok(format!(
            "{}\n\n... [Content truncated at {} characters]",
            &processed[..end],
            MAX_FETCH_CHARS
        ))
    } else {
        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_schema_validity() {
        let tools = get_available_tools();
        assert_eq!(tools.len(), 12);

        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"read_note".to_string()));
        assert!(names.contains(&"list_notes".to_string()));
        assert!(names.contains(&"search_notes".to_string()));
        assert!(names.contains(&"retrieve_context".to_string()));
        assert!(names.contains(&"list_groups".to_string()));
        assert!(names.contains(&"move_note".to_string()));
        assert!(names.contains(&"create_note".to_string()));
        assert!(names.contains(&"edit_note".to_string()));
        assert!(names.contains(&"edit_section".to_string()));
        assert!(names.contains(&"append_to_note".to_string()));
        assert!(names.contains(&"insert_section".to_string()));
        assert!(names.contains(&"fetch_url".to_string()));

        let serialized = serde_json::to_string(&tools).expect("Must serialize tools");
        assert!(serialized.contains("read_note"));
        assert!(serialized.contains("retrieve_context"));
        assert!(serialized.contains("list_groups"));
        assert!(serialized.contains("move_note"));
        assert!(serialized.contains("edit_section"));
        assert!(serialized.contains("append_to_note"));
        assert!(serialized.contains("insert_section"));
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

    #[test]
    fn blocked_host_localhost() {
        assert!(!is_allowed_host("http://localhost:8080/api"));
        assert!(!is_allowed_host("http://sub.localhost/api"));
        assert!(!is_allowed_host("http://localtest.me/api"));
    }

    #[test]
    fn blocked_host_private_10() {
        assert!(!is_allowed_host("http://10.0.0.1/internal"));
    }

    #[test]
    fn blocked_host_private_192() {
        assert!(!is_allowed_host("http://192.168.1.100/data"));
    }

    #[test]
    fn blocked_host_private_172() {
        assert!(!is_allowed_host("http://172.16.0.1/data"));
        assert!(!is_allowed_host("http://172.31.255.254/data"));
    }

    #[test]
    fn blocked_host_link_local() {
        assert!(!is_allowed_host("http://169.254.169.254/metadata"));
    }

    #[test]
    fn blocked_host_alternative_encodings() {
        // Decimal encoding for 127.0.0.1 (2130706433)
        assert!(!is_allowed_host("http://2130706433/"));
        // Hex encoding for 127.0.0.1 (0x7f000001)
        assert!(!is_allowed_host("http://0x7f000001/"));
    }

    #[test]
    fn blocked_host_ipv6_loopback_and_ula() {
        assert!(!is_allowed_host("http://[::1]:3000/"));
        assert!(!is_allowed_host("http://[fd00::1]:80/"));
    }

    #[test]
    fn allowed_public_host_no_false_positive_on_substrings() {
        assert!(!is_blocked_ip(&"93.184.216.34".parse().unwrap()));
        assert!(!is_blocked_ip(&"8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn default_deny_malformed_url() {
        assert!(!is_allowed_host(""));
        assert!(!is_allowed_host("not a url"));
        assert!(!is_allowed_host("ftp://10.0.0.1/file"));
    }
}
