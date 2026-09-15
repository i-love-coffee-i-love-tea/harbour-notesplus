use std::collections::HashMap;
use std::fs;
use std::io::Write;

use crate::constants::{SESSION_COOKIE_NAME, MIME_EVENT_STREAM, MIME_HTML, MIME_JSON, MIME_TEXT_PLAIN};
use crate::error::ApiErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Options,
    Head,
    Patch,
    Unknown,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Head => "HEAD",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Unknown => "UNKNOWN",
        }
    }
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "OPTIONS" => HttpMethod::Options,
            "HEAD" => HttpMethod::Head,
            "PATCH" => HttpMethod::Patch,
            _ => HttpMethod::Unknown,
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for HttpMethod {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl PartialEq<str> for HttpMethod {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other)
    }
}

impl PartialEq<HttpMethod> for &str {
    fn eq(&self, other: &HttpMethod) -> bool {
        self.eq_ignore_ascii_case(other.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ParsedHttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub client_ip: Option<String>,
}

impl ParsedHttpRequest {
    pub fn from_tiny_http(req: &mut tiny_http::Request) -> Result<Self, String> {
        let method = HttpMethod::from(req.method().as_str());
        let raw_url = req.url();
        let (path, query) = match raw_url.split_once('?') {
            Some((p, q)) => (url_decode(p), Some(url_decode(q))),
            None => (url_decode(raw_url), None),
        };
        let mut headers = HashMap::new();
        for h in req.headers() {
            headers.insert(h.field.to_string().to_ascii_lowercase(), h.value.to_string());
        }

        const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB
        let body_len = req.body_length().unwrap_or(0);
        if body_len > MAX_BODY_SIZE {
            return Err(format!(
                "Request body too large: {} bytes (max {})",
                body_len, MAX_BODY_SIZE
            ));
        }

        let mut body = Vec::new();
        if body_len > 0 {
            body.reserve(body_len);
        }
        let _ = req.as_reader().read_to_end(&mut body);

        let client_ip = req.remote_addr().map(|a| a.ip().to_string());

        Ok(ParsedHttpRequest {
            method,
            path,
            query,
            headers,
            body,
            client_ip,
        })
    }

    pub fn client_ip(&self) -> &str {
        if let Some(ref ip) = self.client_ip {
            ip.as_str()
        } else if let Some(hdr) = self.headers.get("x-forwarded-for") {
            hdr.split(',').next().map(|s| s.trim()).unwrap_or("127.0.0.1")
        } else {
            "127.0.0.1"
        }
    }

    pub fn json_body(&self) -> serde_json::Value {
        let body_str = String::from_utf8_lossy(&self.body);
        serde_json::from_str(&body_str).unwrap_or(serde_json::json!({}))
    }
}

pub fn sanitize_header_value(s: &str) -> String {
    s.chars().filter(|c| *c != '"' && *c != '\r' && *c != '\n').collect()
}

/// Validate CORS origin against localhost variants and known LAN IPs.
pub fn validate_cors_origin(origin: Option<&str>, allowed_ips: &[String]) -> String {
    let origin = match origin {
        Some(o) => o,
        None => return String::new(),
    };

    let host = origin
        .split("://")
        .nth(1)
        .unwrap_or(origin)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();

    // Allow localhost variants
    if host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1" {
        return origin.to_string();
    }

    // Allow IPs from the local interface list
    if allowed_ips.contains(&host) {
        return origin.to_string();
    }

    // Allow private LAN ranges
    if host.starts_with("192.168.") || host.starts_with("10.") {
        return origin.to_string();
    }
    // 172.16.0.0/12
    if host.starts_with("172.") {
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() >= 2 {
            if let Ok(second) = parts[1].parse::<u8>() {
                if (16..=31).contains(&second) {
                    return origin.to_string();
                }
            }
        }
    }

    String::new()
}

pub fn build_cors_headers(origin: &str) -> String {
    if origin.is_empty() {
        return String::new();
    }
    format!(
        "Access-Control-Allow-Origin: {}\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization, Cookie\r\n",
        origin
    )
}

pub fn make_session_cookie(session_id: &str, is_tls: bool, max_age: Option<u64>) -> String {
    let mut cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Lax", SESSION_COOKIE_NAME, session_id);
    if let Some(age) = max_age {
        cookie.push_str(&format!("; Max-Age={}", age));
    }
    if is_tls {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn extract_cookie_value(cookie_header: &str, key: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: &'static str,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub extra_headers: Vec<(String, String)>,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &'static str, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status_code,
            status_text,
            content_type,
            body,
            extra_headers: Vec::new(),
        }
    }

    pub fn ok_json(data: &serde_json::Value) -> Self {
        Self::new(200, "OK", MIME_JSON, data.to_string().into_bytes())
    }

    pub fn ok_html(html: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "OK", MIME_HTML, html.into())
    }

    pub fn ok_text(text: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "OK", MIME_TEXT_PLAIN, text.into())
    }

    pub fn error(code: ApiErrorCode, msg: &str) -> Self {
        let body = serde_json::json!({
            "error": msg,
            "code": code.as_str()
        }).to_string().into_bytes();
        Self::new(code.status_code(), code.reason(), MIME_JSON, body)
    }

    pub fn unauthorized(msg: &str) -> Self {
        Self::error(ApiErrorCode::Unauthorized, msg)
    }

    pub fn forbidden(msg: &str) -> Self {
        Self::error(ApiErrorCode::Forbidden, msg)
    }

    pub fn not_found(msg: &str) -> Self {
        Self::error(ApiErrorCode::NotFound, msg)
    }

    pub fn bad_request(msg: &str) -> Self {
        Self::error(ApiErrorCode::BadRequest, msg)
    }

    pub fn internal_error(msg: &str) -> Self {
        Self::error(ApiErrorCode::InternalError, msg)
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((key.into(), value.into()));
        self
    }

    pub fn send<W: Write>(&self, stream: &mut W, cors_origin: &str) {
        let headers: Vec<(&str, &str)> = self.extra_headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        send_response_full(
            stream,
            self.status_code,
            self.status_text,
            self.content_type,
            &self.body,
            cors_origin,
            &headers,
        );
    }
}

pub fn send_response_full<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    cors_origin: &str,
    extra_headers: &[(&str, &str)],
) {
    let cors = build_cors_headers(cors_origin);
    let mut extra = String::new();
    for (k, v) in extra_headers {
        let sanitized = sanitize_header_value(v);
        extra.push_str(&format!("{}: {}\r\n", k, sanitized));
    }
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'\r\n{}{}\r\n",
        status_code,
        status_text,
        content_type,
        body.len(),
        cors,
        extra
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

pub fn send_response<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    cors_origin: &str,
) {
    send_response_full(stream, status_code, status_text, content_type, body, cors_origin, &[]);
}

pub fn send_json_error<W: Write>(stream: &mut W, status_code: u16, reason: &str, msg: &str, cors_origin: &str) {
    let body = serde_json::json!({ "error": msg }).to_string();
    send_response(stream, status_code, reason, MIME_JSON, body.as_bytes(), cors_origin);
}

pub fn send_json_ok<W: Write>(stream: &mut W, data: &serde_json::Value, cors_origin: &str) {
    let body = data.to_string();
    send_response(stream, 200, "OK", MIME_JSON, body.as_bytes(), cors_origin);
}

pub fn send_redirect<W: Write>(
    stream: &mut W,
    location: &str,
    cors_origin: &str,
    extra_headers: &[(&str, &str)],
) {
    let mut headers = vec![("Location", location)];
    headers.extend_from_slice(extra_headers);
    send_response_full(
        stream,
        302,
        "Found",
        MIME_TEXT_PLAIN,
        b"Redirecting...",
        cors_origin,
        &headers,
    );
}

pub fn send_attachment_response<W: Write>(
    stream: &mut W,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
    filename: &str,
    cors_origin: &str,
) {
    let safe_filename = sanitize_header_value(filename);
    let disposition = format!("attachment; filename=\"{}\"", safe_filename);
    send_response_full(
        stream,
        status_code,
        status_text,
        content_type,
        body,
        cors_origin,
        &[("Content-Disposition", &disposition)],
    );
}

pub fn send_sse_header<W: Write>(stream: &mut W, cors_origin: &str) {
    let cors = build_cors_headers(cors_origin);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nCache-Control: no-cache, no-transform\r\nConnection: close\r\nX-Accel-Buffering: no\r\n{}\r\n",
        MIME_EVENT_STREAM,
        cors
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.flush();
}

pub fn send_sse_event<W: Write>(stream: &mut W, data: &serde_json::Value) {
    let payload = format!("data: {}\n\n", data);
    let _ = stream.write_all(payload.as_bytes());
    let _ = stream.flush();
}

pub fn send_sse_done<W: Write>(stream: &mut W) {
    let _ = stream.write_all(b"data: [DONE]\n\n");
    let _ = stream.flush();
}

pub fn get_local_ip_addresses() -> Vec<String> {
    let mut ips = Vec::new();

    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip_str = addr.ip().to_string();
                if ip_str != "0.0.0.0" && ip_str != "127.0.0.1" {
                    ips.push(ip_str);
                }
            }
        }
    }

    if let Ok(arp) = fs::read_to_string("/proc/net/arp") {
        for line in arp.lines().skip(1) {
            if let Some(ip) = line.split_whitespace().next() {
                if ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.") {
                    if let Ok(probe_sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                        if probe_sock.connect(format!("{}:80", ip)).is_ok() {
                            if let Ok(addr) = probe_sock.local_addr() {
                                let local = addr.ip().to_string();
                                if !ips.contains(&local) && local != "0.0.0.0" && local != "127.0.0.1" {
                                    ips.push(local);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }

    ips
}

/// Returns a list of available network interfaces as `(ip_address, interface_name)` pairs.
/// Always includes "0.0.0.0" (All interfaces) as the first entry.
pub fn get_network_interfaces() -> Vec<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut interfaces = Vec::new();

    // Always offer "all interfaces" first
    interfaces.push(("0.0.0.0".to_string(), "All interfaces".to_string()));
    seen.insert("0.0.0.0".to_string());

    // Parse /proc/net/arp to get IPs with their interface names
    if let Ok(arp) = fs::read_to_string("/proc/net/arp") {
        for line in arp.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 6 {
                let ip = fields[0];
                let iface = fields[5];
                if (ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172."))
                    && seen.insert(ip.to_string())
                {
                    interfaces.push((ip.to_string(), format!("{} ({})", iface, ip)));
                }
            }
        }
    }

    // Also discover the primary route interface via UDP probe
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip_str = addr.ip().to_string();
                if ip_str != "0.0.0.0" && ip_str != "127.0.0.1" && seen.insert(ip_str.clone()) {
                    interfaces.push((ip_str.clone(), format!("Default route ({})", ip_str)));
                }
            }
        }
    }

    // Add loopback
    if seen.insert("127.0.0.1".to_string()) {
        interfaces.push(("127.0.0.1".to_string(), "Localhost only".to_string()));
    }

    interfaces
}

/// Checks if an IP address belongs to a private, loopback, or local link network.
pub use crate::net::{is_private_or_local_ip, is_public_ip};

pub fn url_decode(s: &str) -> String {
    let mut result = Vec::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = format!("{}{}", c1, c2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte);
                    continue;
                }
            }
            result.push(b'%');
        } else if ch == '+' {
            result.push(b' ');
        } else {
            result.extend_from_slice(ch.to_string().as_bytes());
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Escapes all HTML-sensitive characters including quotes (for use in attributes).
pub use crate::escape::escape_html;

/// Escapes only &, <, > (for text content where quote escaping is unnecessary).
pub use crate::escape::escape_html_text;
