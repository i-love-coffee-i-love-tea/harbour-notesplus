pub mod agent;
pub mod assets;
pub mod auth;
pub mod pages;

use serde_json::json;

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

    // 1. Handle CORS preflight requests
    if req.method == HttpMethod::Options {
        send_response(&mut writer, 200, "OK", MIME_TEXT_PLAIN, b"", &cors_origin);
        return;
    }

    let clean_path = req.path.trim_start_matches('/');

    // 2. Reject path traversal attempts
    if clean_path.contains("..") {
        send_response(&mut writer, 403, "Forbidden", MIME_TEXT_PLAIN, b"Forbidden", &cors_origin);
        return;
    }

    // 3. Health check & Ping
    if clean_path == API_ROUTE_PING {
        let resp = json!({
            "ok": true,
        });
        send_response(&mut writer, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), &cors_origin);
        return;
    }

    // 4. Public / Auth endpoints
    if clean_path == API_ROUTE_AUTH_CONFIG {
        auth::handle_auth_config(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_AUTH_LOGOUT && req.method == HttpMethod::Post {
        auth::handle_logout(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_AUTH_WHOAMI && req.method == HttpMethod::Get {
        auth::handle_whoami(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_AUTH_CODE_INITIATE && req.method == HttpMethod::Post {
        auth::handle_challenge_initiate(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_AUTH_CODE_STATUS && req.method == HttpMethod::Get {
        auth::handle_challenge_status(&mut writer, &req, &ctx, &cors_origin);
        return;
    }


    // 4b. Theme colors (Sailfish ambience sync)
    if clean_path == API_ROUTE_THEME && req.method == HttpMethod::Get {
        let colors = ctx.theme_colors.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let body = serde_json::to_string(&colors).unwrap_or_else(|_| "{}".into());
        send_response(&mut writer, 200, "OK", MIME_JSON, body.as_bytes(), &cors_origin);
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
        || clean_path.starts_with("composables/")
        || clean_path.starts_with("api/auth/");

    if !is_public_path {
        let session = auth::authenticate_request(&req, &ctx.session_store);
        if session.is_none() {
            if clean_path.starts_with("api/") {
                let err = json!({ "error": "Unauthorized", "authenticated": false });
                send_response(&mut writer, 401, "Unauthorized", MIME_JSON, err.to_string().as_bytes(), &cors_origin);
                return;
            } else {
                assets::handle_static_asset(&mut writer, &req, "", &ctx, &cors_origin);
                return;
            }
        }
    }

    // 6. Pages API
    if clean_path == API_ROUTE_PAGES {
        pages::handle_pages_api(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path.starts_with("api/pages/") {
        let filename = clean_path.strip_prefix("api/pages/").unwrap_or("");
        pages::handle_page_detail_api(&mut writer, &req, filename, &ctx, &cors_origin);
        return;
    }

    // 7. Search API
    if clean_path == API_ROUTE_SEARCH && req.method == HttpMethod::Get {
        pages::handle_search_api(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    // 7b. Groups API
    if clean_path == "api/groups" || clean_path.starts_with("api/groups/") {
        pages::handle_groups_api(&mut writer, &req, clean_path, &ctx, &cors_origin);
        return;
    }

    // 8. Notes API
    if clean_path == "api/notes" || clean_path.starts_with("api/notes/") {
        pages::handle_notes_api(&mut writer, &req, clean_path, &ctx, &cors_origin);
        return;
    }

    // 9. Document Rendering & Parsing APIs
    if clean_path == API_ROUTE_RENDER && req.method == HttpMethod::Post {
        pages::handle_render_api(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_BLOCKS_PARSE && req.method == HttpMethod::Post {
        pages::handle_blocks_parse_api(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_BLOCKS_TO_ADOC && req.method == HttpMethod::Post {
        pages::handle_blocks_to_adoc_api(&mut writer, &req, &cors_origin);
        return;
    }

    if clean_path == "api/export/html" {
        pages::handle_export_html_api(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if clean_path == "api/export/all" {
        pages::handle_export_all_api(&mut writer, &ctx, &cors_origin);
        return;
    }

    // 10. AI Agent APIs
    if (clean_path == API_ROUTE_AI_STATUS || clean_path == "api/agent/status") && req.method == HttpMethod::Get {
        agent::handle_agent_status(&mut writer, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_MODELS || clean_path == "api/agent/models") && req.method == HttpMethod::Get {
        agent::handle_agent_models(&mut writer, &ctx, &cors_origin);
        return;
    }

    if clean_path == API_ROUTE_AI_CONFIG || clean_path == "api/agent/config" {
        agent::handle_agent_config(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_CHAT || clean_path == "api/agent/chat") && req.method == HttpMethod::Post {
        agent::handle_agent_chat(writer, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_TEMPLATE || clean_path == "api/agent/template") && req.method == HttpMethod::Post {
        agent::handle_agent_template(writer, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_CONFIRM || clean_path == "api/agent/confirm") && req.method == HttpMethod::Post {
        agent::handle_agent_confirm(writer, &req, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_UNDO || clean_path == "api/agent/undo") && req.method == HttpMethod::Post {
        agent::handle_agent_undo(&mut writer, &ctx, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_FETCH_URL || clean_path == "api/agent/fetch_url" || clean_path == "api/fetch_url") && req.method == HttpMethod::Post {
        agent::handle_fetch_url(&mut writer, &req, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_PREPROCESS_HTML || clean_path == "api/agent/preprocess_html" || clean_path == "api/preprocess_html") && req.method == HttpMethod::Post {
        agent::handle_preprocess_html(&mut writer, &req, &cors_origin);
        return;
    }

    if (clean_path == API_ROUTE_AI_READ_FILE || clean_path == "api/agent/read_file" || clean_path == "api/read_file") && req.method == HttpMethod::Post {
        agent::handle_read_file(&mut writer, &req, &ctx, &cors_origin);
        return;
    }

    // 11. Page URLs
    if clean_path.starts_with("page/") || clean_path.starts_with("notes/") || clean_path.starts_with("edit/") {
        pages::handle_page_url(&mut writer, &req, clean_path, &ctx, &cors_origin);
        return;
    }

    // 12. Raw note text URL
    if clean_path.starts_with("raw/") {
        pages::handle_raw_url(&mut writer, clean_path, &ctx, &cors_origin);
        return;
    }

    // 13. Standalone HTML5 export download URL
    if clean_path.starts_with("export/") {
        pages::handle_export_url(&mut writer, clean_path, &ctx, &cors_origin);
        return;
    }

    // 14. Static Web Assets and Note Attachments
    assets::handle_static_asset(&mut writer, &req, clean_path, &ctx, &cors_origin);
}
