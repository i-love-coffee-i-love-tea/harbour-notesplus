pub mod agent;
pub mod assets;
pub mod auth;
pub mod pages;

use serde_json::json;

use crate::constants::{API_ROUTE_PING, MIME_JSON, MIME_TEXT_PLAIN};
use crate::server::http::{
    get_local_ip_addresses, parse_http_request, send_response, validate_cors_origin,
    StreamWrapper,
};
use crate::server::ServerContext;

pub fn handle_http_client(mut stream: StreamWrapper, ctx: ServerContext) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));

    let req = match parse_http_request(&mut stream) {
        Ok(r) => r,
        Err(_) => {
            send_response(&mut stream, 400, "Bad Request", MIME_TEXT_PLAIN, b"Bad Request", "");
            return;
        }
    };

    let local_ips = get_local_ip_addresses();
    let cors_origin = validate_cors_origin(req.headers.get("origin").map(|s| s.as_str()), &local_ips);

    // 1. Handle CORS preflight requests
    if req.method == "OPTIONS" {
        send_response(&mut stream, 200, "OK", MIME_TEXT_PLAIN, b"", &cors_origin);
        return;
    }

    let clean_path = req.path.trim_start_matches('/');

    // 2. Reject path traversal attempts
    if clean_path.contains("..") {
        send_response(&mut stream, 403, "Forbidden", MIME_TEXT_PLAIN, b"Forbidden", &cors_origin);
        return;
    }

    // 3. Health check & Ping
    if clean_path == API_ROUTE_PING {
        let resp = json!({
            "ok": true,
        });
        send_response(&mut stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    // 4. Public / Auth endpoints
    if clean_path == "api/auth/config" {
        auth::handle_auth_config(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/auth/logout" && req.method == "POST" {
        auth::handle_logout(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/auth/whoami" && req.method == "GET" {
        auth::handle_whoami(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/auth/code/initiate" || clean_path == "api/auth/fingerprint/initiate" || clean_path == "api/auth/phone/initiate") && req.method == "POST" {
        auth::handle_challenge_initiate(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/auth/code/status" || clean_path == "api/auth/fingerprint/status" || clean_path == "api/auth/phone/status") && req.method == "GET" {
        auth::handle_challenge_status(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/auth/code/approve" || clean_path == "api/auth/fingerprint/approve" || clean_path == "api/auth/phone/approve") && req.method == "POST" {
        auth::handle_challenge_approve(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/auth/code/deny" || clean_path == "api/auth/fingerprint/deny" || clean_path == "api/auth/phone/deny") && req.method == "POST" {
        auth::handle_challenge_deny(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    // 4b. Theme colors (Sailfish ambience sync)
    if clean_path == "api/theme" && req.method == "GET" {
        let colors = ctx.theme_colors.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let body = serde_json::to_string(&colors).unwrap_or_else(|_| "{}".into());
        send_response(&mut stream, 200, "OK", MIME_JSON, body.as_bytes(), &cors_origin);
        return;
    }

    // 5. Auth gatekeeper: enforce authentication for non-public paths
    let is_public_path = clean_path.is_empty()
        || clean_path == "index.html"
        || clean_path == "app.js"
        || clean_path == "vue.esm-browser.prod.js"
        || clean_path == "vue.js"
        || clean_path == "style.css"
        || clean_path == "icon.png"
        || clean_path == "favicon.ico"
        || clean_path.starts_with("assets/")
        || clean_path.starts_with("api/auth/");

    if !is_public_path {
        let session = auth::authenticate_request(&req, &ctx.session_store);
        if session.is_none() {
            if clean_path.starts_with("api/") {
                let err = json!({ "error": "Unauthorized", "authenticated": false });
                send_response(&mut stream, 401, "Unauthorized", MIME_JSON, err.to_string().as_bytes(), &cors_origin);
                return;
            } else {
                assets::handle_static_asset(&mut stream, &req, "", &ctx, &cors_origin);
                return;
            }
        }
    }

    // 6. Pages API
    if clean_path == "api/pages" {
        pages::handle_pages_api(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path.starts_with("api/pages/") {
        let filename = clean_path.strip_prefix("api/pages/").unwrap_or("");
        pages::handle_page_detail_api(&mut stream, &req, filename, &ctx, &cors_origin);
        return;
    }

    // 7. Search API
    if clean_path == "api/search" && req.method == "GET" {
        pages::handle_search_api(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    // 8. Notes API
    if clean_path == "api/notes" || clean_path.starts_with("api/notes/") {
        pages::handle_notes_api(&mut stream, &req, clean_path, &ctx, &cors_origin);
        return;
    }

    // 9. Document Rendering & Parsing APIs
    if clean_path == "api/render" && req.method == "POST" {
        pages::handle_render_api(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/blocks/parse" && req.method == "POST" {
        pages::handle_blocks_parse_api(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/blocks/to_adoc" && req.method == "POST" {
        pages::handle_blocks_to_adoc_api(&mut stream, &req, &cors_origin);
        return;
    }

    if clean_path == "api/export/html" {
        pages::handle_export_html_api(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/export/all" {
        pages::handle_export_all_api(&mut stream, &ctx, &cors_origin);
        return;
    }

    // 10. AI Agent APIs
    if (clean_path == "api/ai/models" || clean_path == "api/agent/models") && req.method == "GET" {
        agent::handle_agent_models(&mut stream, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/ai/config" || clean_path == "api/agent/config" {
        agent::handle_agent_config(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/chat" || clean_path == "api/agent/chat") && req.method == "POST" {
        agent::handle_agent_chat(stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/template" || clean_path == "api/agent/template") && req.method == "POST" {
        agent::handle_agent_template(stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/confirm" || clean_path == "api/agent/confirm") && req.method == "POST" {
        agent::handle_agent_confirm(stream, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/undo" || clean_path == "api/agent/undo") && req.method == "POST" {
        agent::handle_agent_undo(&mut stream, &ctx, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/fetch_url" || clean_path == "api/agent/fetch_url" || clean_path == "api/fetch_url") && req.method == "POST" {
        agent::handle_fetch_url(&mut stream, &req, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/preprocess_html" || clean_path == "api/agent/preprocess_html" || clean_path == "api/preprocess_html") && req.method == "POST" {
        agent::handle_preprocess_html(&mut stream, &req, &cors_origin);
        return;
    }

    if (clean_path == "api/ai/read_file" || clean_path == "api/agent/read_file" || clean_path == "api/read_file") && req.method == "POST" {
        agent::handle_read_file(&mut stream, &req, &ctx, &cors_origin);
        return;
    }

    // 11. Page URLs
    if clean_path.starts_with("page/") || clean_path.starts_with("notes/") || clean_path.starts_with("edit/") {
        pages::handle_page_url(&mut stream, &req, clean_path, &ctx, &cors_origin);
        return;
    }

    // 12. Raw note text URL
    if clean_path.starts_with("raw/") {
        pages::handle_raw_url(&mut stream, clean_path, &ctx, &cors_origin);
        return;
    }

    // 13. Standalone HTML5 export download URL
    if clean_path.starts_with("export/") {
        pages::handle_export_url(&mut stream, clean_path, &ctx, &cors_origin);
        return;
    }

    // 14. Static Web Assets and Note Attachments
    assets::handle_static_asset(&mut stream, &req, clean_path, &ctx, &cors_origin);
}
