//! Tool specifications and JSON Schema definitions for LLM function calling.

use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
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

/// Checks whether an IP address belongs to loopback, private, link-local, multicast, or reserved ranges.
pub fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // Loopback (127.0.0.0/8)
            if v4.is_loopback() || octets[0] == 127 {
                return true;
            }
            // Unspecified / Current network (0.0.0.0/8)
            if v4.is_unspecified() || octets[0] == 0 {
                return true;
            }
            // Private ranges:
            // 10.0.0.0/8
            if octets[0] == 10 {
                return true;
            }
            // 172.16.0.0/12
            if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                return true;
            }
            // 192.168.0.0/16
            if octets[0] == 192 && octets[1] == 168 {
                return true;
            }
            // Link-local / AWS & cloud metadata (169.254.0.0/16)
            if v4.is_link_local() || (octets[0] == 169 && octets[1] == 254) {
                return true;
            }
            // Carrier-grade NAT (100.64.0.0/10)
            if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                return true;
            }
            // IETF Protocol Assignments (192.0.0.0/24)
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
                return true;
            }
            // Documentation TEST-NET-1 (192.0.2.0/24)
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
                return true;
            }
            // Documentation TEST-NET-2 (198.51.100.0/24)
            if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
                return true;
            }
            // Documentation TEST-NET-3 (203.0.113.0/24)
            if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
                return true;
            }
            // Benchmarking (198.18.0.0/15)
            if octets[0] == 198 && (18..=19).contains(&octets[1]) {
                return true;
            }
            // Multicast (224.0.0.0/4) & Reserved (240.0.0.0/4) & Broadcast
            if v4.is_multicast() || v4.is_broadcast() || octets[0] >= 224 {
                return true;
            }
            false
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            // Loopback (::1)
            if v6.is_loopback() {
                return true;
            }
            // Unspecified (::)
            if v6.is_unspecified() {
                return true;
            }
            // IPv4-mapped IPv6 (::ffff:0:0/96 or ::ffff:0:0:0/96)
            if let Some(v4) = v6.to_ipv4() {
                return is_blocked_ip(&IpAddr::V4(v4));
            }
            // Unique Local Address ULA (fc00::/7 -> fc00:: & fd00::)
            if (segments[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // Link-local unicast (fe80::/10)
            if (segments[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // Multicast (ff00::/8)
            if v6.is_multicast() || (segments[0] & 0xff00) == 0xff00 {
                return true;
            }
            // Documentation (2001:db8::/32)
            if segments[0] == 0x2001 && segments[1] == 0x0db8 {
                return true;
            }
            // Discard prefix (100::/64)
            if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
                return true;
            }
            false
        }
    }
}

/// Parses target host, port, and scheme from an HTTP/HTTPS URL string.
pub fn parse_target_host_port(url_str: &str) -> Result<(String, u16, String), String> {
    let trimmed = url_str.trim();
    let (scheme, rest) = if let Some(s) = trimmed.strip_prefix("http://") {
        ("http", s)
    } else if let Some(s) = trimmed.strip_prefix("https://") {
        ("https", s)
    } else if !trimmed.contains("://") {
        ("https", trimmed)
    } else {
        return Err(format!("Unsupported URL scheme in '{}'", trimmed));
    };

    let host_port_part = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host_port_part.is_empty() {
        return Err("Missing host in URL".to_string());
    }

    let default_port = if scheme == "https" { 443 } else { 80 };

    if host_port_part.starts_with('[') {
        if let Some(closing_bracket) = host_port_part.find(']') {
            let ip6_str = &host_port_part[1..closing_bracket];
            let after_bracket = &host_port_part[closing_bracket + 1..];
            let port = if let Some(colon) = after_bracket.strip_prefix(':') {
                colon.parse::<u16>().map_err(|_| "Invalid port".to_string())?
            } else {
                default_port
            };
            return Ok((ip6_str.to_string(), port, scheme.to_string()));
        } else {
            return Err("Malformed IPv6 host literal".to_string());
        }
    }

    let (host, port) = if let Some((h, p)) = host_port_part.split_once(':') {
        let parsed_port = p.parse::<u16>().map_err(|_| "Invalid port".to_string())?;
        (h, parsed_port)
    } else {
        (host_port_part, default_port)
    };

    Ok((host.to_string(), port, scheme.to_string()))
}

/// Parses alternative direct IP representations (standard, hex, decimal, octal, dotted).
pub fn parse_direct_ip(host: &str) -> Option<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip);
    }
    if host.starts_with("0x") || host.starts_with("0X") {
        if let Ok(num) = u32::from_str_radix(&host[2..], 16) {
            return Some(IpAddr::V4(Ipv4Addr::from(num)));
        }
    }
    if let Ok(num) = host.parse::<u32>() {
        return Some(IpAddr::V4(Ipv4Addr::from(num)));
    }
    if host.starts_with('0') && host.len() > 1 && host.chars().all(|c| c.is_digit(8)) {
        if let Ok(num) = u32::from_str_radix(host, 8) {
            return Some(IpAddr::V4(Ipv4Addr::from(num)));
        }
    }
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        let mut octets = [0u8; 4];
        let mut valid = true;
        for (i, part) in parts.iter().enumerate() {
            let val = if part.starts_with("0x") || part.starts_with("0X") {
                u32::from_str_radix(&part[2..], 16).ok()
            } else if part.starts_with('0') && part.len() > 1 && part.chars().all(|c| c.is_digit(8)) {
                u32::from_str_radix(part, 8).ok()
            } else {
                part.parse::<u32>().ok()
            };
            if let Some(v) = val {
                if v <= 255 {
                    octets[i] = v as u8;
                } else {
                    valid = false;
                    break;
                }
            } else {
                valid = false;
                break;
            }
        }
        if valid {
            return Some(IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3])));
        }
    }
    None
}

/// Returns true if the URL targets a blocked (localhost/private/internal) host or IP.
pub fn is_blocked_host(url: &str) -> bool {
    let (host, port, _) = match parse_target_host_port(url) {
        Ok(res) => res,
        Err(_) => return true,
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
        return true;
    }

    if let Some(ip) = parse_direct_ip(&host) {
        return is_blocked_ip(&ip);
    }

    let socket_addr_str = format!("{}:{}", host, port);
    if let Ok(addrs) = socket_addr_str.to_socket_addrs() {
        for addr in addrs {
            if is_blocked_ip(&addr.ip()) {
                return true;
            }
        }
    }

    false
}

/// Helper to download web text from an HTTP/HTTPS URL with SSRF mitigation and secure redirect re-checking.
pub fn fetch_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let mut current_url = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };

    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(std::time::Duration::from_secs(20))
        .build();

    const MAX_REDIRECTS: usize = 3;
    let mut redirects_followed = 0;

    let resp = loop {
        if is_blocked_host(&current_url) {
            return Err(format!(
                "URL '{}' blocked: fetching localhost/private addresses is not allowed",
                current_url
            ));
        }

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
                        return Err(format!("Unsupported relative redirect to '{}'", location));
                    };

                    current_url = next_url;
                    redirects_followed += 1;
                    continue;
                } else {
                    return Err(format!("Redirect status {} with missing Location header", code));
                }
            }
            Err(e) => return Err(format!("Failed to fetch URL '{}': {}", current_url, e)),
        }
    };

    let content_type = resp.header("Content-Type").unwrap_or("").to_lowercase();
    let raw_text = resp.into_string().unwrap_or_default();

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
            if last_nl > MAX_FETCH_CHARS - 5000 {
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

    #[test]
    fn blocked_host_localhost() {
        assert!(is_blocked_host("http://localhost:8080/api"));
        assert!(is_blocked_host("http://sub.localhost/api"));
        assert!(is_blocked_host("http://localtest.me/api"));
    }

    #[test]
    fn blocked_host_private_10() {
        assert!(is_blocked_host("http://10.0.0.1/internal"));
    }

    #[test]
    fn blocked_host_private_192() {
        assert!(is_blocked_host("http://192.168.1.100/data"));
    }

    #[test]
    fn blocked_host_private_172() {
        assert!(is_blocked_host("http://172.16.0.1/data"));
        assert!(is_blocked_host("http://172.31.255.254/data"));
    }

    #[test]
    fn blocked_host_link_local() {
        assert!(is_blocked_host("http://169.254.169.254/metadata"));
    }

    #[test]
    fn blocked_host_alternative_encodings() {
        // Decimal encoding for 127.0.0.1 (2130706433)
        assert!(is_blocked_host("http://2130706433/"));
        // Hex encoding for 127.0.0.1 (0x7f000001)
        assert!(is_blocked_host("http://0x7f000001/"));
    }

    #[test]
    fn blocked_host_ipv6_loopback_and_ula() {
        assert!(is_blocked_host("http://[::1]:3000/"));
        assert!(is_blocked_host("http://[fd00::1]:80/"));
    }

    #[test]
    fn allowed_public_host_no_false_positive_on_substrings() {
        // URLs with substring '10.' or '172.16.' or '192.168.' in domain name should NOT be blocked by string match
        assert!(!is_blocked_ip(&"93.184.216.34".parse().unwrap())); // example.com
        assert!(!is_blocked_ip(&"8.8.8.8".parse().unwrap())); // dns.google
    }
}
