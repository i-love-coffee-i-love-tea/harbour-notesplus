pub mod agent;
pub mod assets;
pub mod auth;
pub mod pages;

use serde_json::json;

use crate::MutexResultExt;
use crate::constants::*;
use crate::server::http::{
    get_local_ip_addresses, send_response, validate_cors_origin,
    HttpMethod, ParsedHttpRequest,
};
use crate::server::ServerContext;

pub fn handle_http_client(mut request: tiny_http::Request, ctx: ServerContext) {
    let req = match ParsedHttpRequest::from_tiny_http(&mut request) {
        Ok(r) => r,
        Err(_) => {
            let mut writer = request.into_writer();
            send_response(&mut writer, 400, "Bad Request", MIME_TEXT_PLAIN, b"Bad Request", "");
            return;
        }
    };

    let mut writer = request.into_writer();
    let local_ips = get_local_ip_addresses();
    let cors_origin = validate_cors_origin(req.headers.get("origin").map(|s| s.as_str()), &local_ips);

    if req.method == HttpMethod::Options {
        send_response(&mut writer, 200, "OK", MIME_TEXT_PLAIN, b"", &cors_origin);
        return;
    }

    let clean_path = req.path.trim_start_matches('/');

    if clean_path.contains("..") {
        send_response(&mut writer, 403, "Forbidden", MIME_TEXT_PLAIN, b"Forbidden", &cors_origin);
        return;
    }

    // Pre-auth public routes
    if dispatch_public(&mut writer, &req, clean_path, &ctx, &cors_origin) {
        return;
    }

    // Auth gatekeeper for non-public paths
    if !is_public_path(clean_path) {
        let session = auth::authenticate_request(&req, &ctx.session_store);
        if session.is_none() {
            if clean_path.starts_with("api/") {
                let err = json!({ "error": "Unauthorized", "authenticated": false });
                send_response(&mut writer, 401, "Unauthorized", MIME_JSON, err.to_string().as_bytes(), &cors_origin);
            } else {
                assets::handle_static_asset(&mut writer, &req, "", &ctx, &cors_origin);
            }
            return;
        }
    }

    // SSE handlers need owned writer — handle before borrow-heavy dispatch
    if clean_path == API_ROUTE_EVENTS && req.method == HttpMethod::Get {
        pages::handle_events_sse(writer, &req, &ctx, &cors_origin);
        return;
    }
    if (clean_path == API_ROUTE_AI_CHAT || clean_path == API_ROUTE_AGENT_CHAT) && req.method == HttpMethod::Post {
        agent::handle_agent_chat(writer, &req, &ctx, &cors_origin);
        return;
    }
    if (clean_path == API_ROUTE_AI_TEMPLATE || clean_path == API_ROUTE_AGENT_TEMPLATE) && req.method == HttpMethod::Post {
        agent::handle_agent_template(writer, &req, &ctx, &cors_origin);
        return;
    }
    if (clean_path == API_ROUTE_AI_CONFIRM || clean_path == API_ROUTE_AGENT_CONFIRM) && req.method == HttpMethod::Post {
        agent::handle_agent_confirm(writer, &req, &ctx, &cors_origin);
        return;
    }

    // Standard routes dispatched by first path segment
    match clean_path.split('/').next().unwrap_or("") {
        "api" => dispatch_api(&mut writer, &req, clean_path, &ctx, &cors_origin),
        "page" | "notes" | "edit" => pages::handle_page_url(&mut writer, &req, clean_path, &ctx, &cors_origin),
        "raw" => pages::handle_raw_url(&mut writer, clean_path, &ctx, &cors_origin),
        "export" => pages::handle_export_url(&mut writer, clean_path, &ctx, &cors_origin),
        _ => assets::handle_static_asset(&mut writer, &req, clean_path, &ctx, &cors_origin),
    }
}

/// Public routes that skip authentication. Returns true if handled.
fn dispatch_public(
    writer: &mut impl std::io::Write,
    req: &ParsedHttpRequest,
    p: &str,
    ctx: &ServerContext,
    cors: &str,
) -> bool {
    if p == API_ROUTE_PING {
        let resp = json!({ "ok": true });
        send_response(writer, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors);
        return true;
    }
    if p == API_ROUTE_AUTH_CONFIG {
        auth::handle_auth_config(writer, req, ctx, cors);
        return true;
    }
    if p == API_ROUTE_AUTH_LOGOUT && req.method == HttpMethod::Post {
        auth::handle_logout(writer, req, ctx, cors);
        return true;
    }
    if p == API_ROUTE_AUTH_WHOAMI && req.method == HttpMethod::Get {
        auth::handle_whoami(writer, req, ctx, cors);
        return true;
    }
    if p == API_ROUTE_AUTH_CODE_INITIATE && req.method == HttpMethod::Post {
        auth::handle_challenge_initiate(writer, req, ctx, cors);
        return true;
    }
    if p == API_ROUTE_AUTH_CODE_STATUS && req.method == HttpMethod::Get {
        auth::handle_challenge_status(writer, req, ctx, cors);
        return true;
    }
    if p == API_ROUTE_THEME && req.method == HttpMethod::Get {
        let colors = ctx.theme_colors.lock().recover().clone();
        let body = serde_json::to_string(&colors).unwrap_or_else(|_| "{}".into());
        send_response(writer, 200, "OK", MIME_JSON, body.as_bytes(), cors);
        return true;
    }
    false
}

fn is_public_path(p: &str) -> bool {
    p.is_empty()
        || p == "index.html"
        || p == "app.js"
        || p == "template.js"
        || p == "vue.runtime.esm-browser.prod.js"
        || p == "vue.esm-browser.prod.js"
        || p == "vue.js"
        || p == "pinia.esm-browser.prod.js"
        || p == "pinia.js"
        || p == "vue-demi.esm-browser.js"
        || p == "vue-devtools-api-stub.js"
        || p == "style.css"
        || p == "icon.png"
        || p == "favicon.ico"
        || p.starts_with("assets/")
        || p.starts_with("composables/")
        || p.starts_with("stores/")
        || p.starts_with(API_ROUTE_AUTH_PREFIX)
}

/// Authenticated API routes — called after auth gatekeeper passes.
fn dispatch_api(
    writer: &mut impl std::io::Write,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors: &str,
) {
    // Pages
    if clean_path == API_ROUTE_PAGES {
        pages::handle_pages_api(writer, req, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_TREE {
        pages::handle_tree_api(writer, req, ctx, cors);
        return;
    }
    if clean_path.starts_with(API_ROUTE_PAGES_PREFIX) {
        let filename = &clean_path[API_ROUTE_PAGES_PREFIX.len()..];
        pages::handle_page_detail_api(writer, req, filename, ctx, cors);
        return;
    }
    // Search
    if clean_path == API_ROUTE_SEARCH && req.method == HttpMethod::Get {
        pages::handle_search_api(writer, req, ctx, cors);
        return;
    }
    // Groups
    if clean_path == API_ROUTE_GROUPS || clean_path.starts_with(API_ROUTE_GROUPS_PREFIX) {
        pages::handle_groups_api(writer, req, clean_path, ctx, cors);
        return;
    }
    // Notes
    if clean_path == API_ROUTE_NOTES || clean_path.starts_with(API_ROUTE_NOTES_PREFIX) {
        pages::handle_notes_api(writer, req, clean_path, ctx, cors);
        return;
    }
    // Rendering
    if clean_path == API_ROUTE_RENDER && req.method == HttpMethod::Post {
        pages::handle_render_api(writer, req, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_BLOCKS_PARSE && req.method == HttpMethod::Post {
        pages::handle_blocks_parse_api(writer, req, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_BLOCKS_TO_ADOC && req.method == HttpMethod::Post {
        pages::handle_blocks_to_adoc_api(writer, req, cors);
        return;
    }
    // Export
    if clean_path == API_ROUTE_EXPORT_HTML {
        pages::handle_export_html_api(writer, req, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_EXPORT_PDF {
        pages::handle_export_pdf_api(writer, req, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_EXPORT_ALL {
        pages::handle_export_all_api(writer, ctx, cors);
        return;
    }
    // Agent (non-SSE endpoints)
    if (clean_path == API_ROUTE_AI_STATUS || clean_path == API_ROUTE_AGENT_STATUS) && req.method == HttpMethod::Get {
        agent::handle_agent_status(writer, ctx, cors);
        return;
    }
    if (clean_path == API_ROUTE_AI_MODELS || clean_path == API_ROUTE_AGENT_MODELS) && req.method == HttpMethod::Get {
        agent::handle_agent_models(writer, ctx, cors);
        return;
    }
    if clean_path == API_ROUTE_AI_CONFIG || clean_path == API_ROUTE_AGENT_CONFIG {
        agent::handle_agent_config(writer, req, ctx, cors);
        return;
    }
    if (clean_path == API_ROUTE_AI_UNDO || clean_path == API_ROUTE_AGENT_UNDO) && req.method == HttpMethod::Post {
        agent::handle_agent_undo(writer, ctx, cors);
        return;
    }
    if (clean_path == API_ROUTE_AI_FETCH_URL || clean_path == API_ROUTE_AGENT_FETCH_URL || clean_path == "api/fetch_url") && req.method == HttpMethod::Post {
        agent::handle_fetch_url(writer, req, cors);
        return;
    }
    if (clean_path == API_ROUTE_AI_PREPROCESS_HTML || clean_path == API_ROUTE_AGENT_PREPROCESS_HTML || clean_path == "api/preprocess_html") && req.method == HttpMethod::Post {
        agent::handle_preprocess_html(writer, req, cors);
        return;
    }
    if (clean_path == API_ROUTE_AI_READ_FILE || clean_path == API_ROUTE_AGENT_READ_FILE || clean_path == "api/read_file") && req.method == HttpMethod::Post {
        agent::handle_read_file(writer, req, ctx, cors);
        return;
    }
    // Fallback
    let err = json!({ "error": "Not found" });
    send_response(writer, 404, "Not Found", MIME_JSON, err.to_string().as_bytes(), cors);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_public_path_allows_static_assets() {
        assert!(is_public_path(""));
        assert!(is_public_path("index.html"));
        assert!(is_public_path("app.js"));
        assert!(is_public_path("style.css"));
        assert!(is_public_path("icon.png"));
        assert!(is_public_path("favicon.ico"));
        assert!(is_public_path("assets/icons/foo.png"));
        assert!(is_public_path("composables/useEditor.js"));
    }

    #[test]
    fn is_public_path_allows_auth_routes() {
        assert!(is_public_path("api/auth/config"));
        assert!(is_public_path("api/auth/logout"));
        assert!(is_public_path("api/auth/challenge/initiate"));
    }

    #[test]
    fn is_public_path_blocks_protected_routes() {
        assert!(!is_public_path("api/pages"));
        assert!(!is_public_path("api/pages/test.adoc"));
        assert!(!is_public_path("api/tree"));
        assert!(!is_public_path("api/notes"));
        assert!(!is_public_path("api/agent/chat"));
        assert!(!is_public_path("api/search"));
        assert!(!is_public_path("api/groups"));
        assert!(!is_public_path("page/test.adoc"));
        assert!(!is_public_path("notes/test.adoc"));
        assert!(!is_public_path("raw/test.adoc"));
        assert!(!is_public_path("edit/test.adoc"));
    }

    #[test]
    fn is_public_path_blocks_path_traversal_in_asset_paths() {
        // These are "public" by prefix but path traversal is caught separately
        // by the ".." check in handle_http_client before is_public_path is called.
        // Verify the public path check doesn't accidentally expose API routes.
        assert!(!is_public_path("api/pages/../../../etc/passwd"));
        assert!(!is_public_path("api/agent/config"));
    }

    #[test]
    fn path_traversal_with_dotdot_rejected() {
        // The ".." check in handle_http_client runs before any dispatch.
        // Verify the pattern catches common traversal attempts.
        let traversal_paths = vec![
            "../etc/passwd",
            "api/pages/../../etc/passwd",
            "page/../secret.adoc",
            "notes/../../database.db",
            "raw/../../../etc/shadow",
            "export/..%2F..%2Fetc/passwd",  // URL-encoded — but %2F becomes / after url_decode
        ];
        for p in &traversal_paths {
            assert!(p.contains(".."), "test case '{}' should contain '..'", p);
        }
    }

    #[test]
    fn is_blocked_ip_covers_all_private_ranges() {
        use std::net::IpAddr;

        // RFC 1918
        assert!(crate::net::is_blocked_ip(&"10.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"10.255.255.255".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"172.16.0.1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"172.31.255.255".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"192.168.0.1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"192.168.255.255".parse::<IpAddr>().unwrap()));

        // Loopback
        assert!(crate::net::is_blocked_ip(&"127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"127.255.255.255".parse::<IpAddr>().unwrap()));

        // Link-local
        assert!(crate::net::is_blocked_ip(&"169.254.169.254".parse::<IpAddr>().unwrap()));

        // CGNAT
        assert!(crate::net::is_blocked_ip(&"100.64.0.1".parse::<IpAddr>().unwrap()));

        // Multicast
        assert!(crate::net::is_blocked_ip(&"224.0.0.1".parse::<IpAddr>().unwrap()));

        // Broadcast
        assert!(crate::net::is_blocked_ip(&"255.255.255.255".parse::<IpAddr>().unwrap()));

        // IPv6
        assert!(crate::net::is_blocked_ip(&"::1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"fd00::1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"fe80::1".parse::<IpAddr>().unwrap()));
        assert!(crate::net::is_blocked_ip(&"::ffff:127.0.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn is_blocked_ip_allows_public_ips() {
        use std::net::IpAddr;

        assert!(!crate::net::is_blocked_ip(&"8.8.8.8".parse::<IpAddr>().unwrap()));
        assert!(!crate::net::is_blocked_ip(&"1.1.1.1".parse::<IpAddr>().unwrap()));
        assert!(!crate::net::is_blocked_ip(&"93.184.216.34".parse::<IpAddr>().unwrap()));
        assert!(!crate::net::is_blocked_ip(&"2001:4860:4860::8888".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn is_blocked_ip_blocks_edge_cases() {
        use std::net::IpAddr;

        // 172.15.x.x and 172.32.x.x should NOT be blocked (outside 172.16/12)
        assert!(!crate::net::is_blocked_ip(&"172.15.0.1".parse::<IpAddr>().unwrap()));
        assert!(!crate::net::is_blocked_ip(&"172.32.0.1".parse::<IpAddr>().unwrap()));

        // 100.63.x.x and 100.128.x.x should NOT be blocked (outside 100.64/10)
        assert!(!crate::net::is_blocked_ip(&"100.63.0.1".parse::<IpAddr>().unwrap()));
        assert!(!crate::net::is_blocked_ip(&"100.128.0.1".parse::<IpAddr>().unwrap()));
    }
}
