use std::fs;
use std::io::Write;

use crate::constants::{MIME_HTML, MIME_JSON};
use crate::server::http::{send_response, ParsedHttpRequest};
use crate::server::web_assets::{APP_JS, ICON_PNG, INDEX_HTML, STYLE_CSS, VUE_JS};
use crate::server::ServerContext;

pub fn handle_static_asset<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if req.method == "GET" && (clean_path.is_empty() || clean_path == "index.html") {
        send_response(stream, 200, "OK", MIME_HTML, INDEX_HTML.as_bytes(), cors_origin);
        return;
    }

    if req.method == "GET" && (clean_path == "icon.png" || clean_path == "favicon.ico") {
        send_response(stream, 200, "OK", "image/png", ICON_PNG, cors_origin);
        return;
    }

    if req.method == "GET" && clean_path == "app.js" {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", APP_JS.as_bytes(), cors_origin);
        return;
    }

    if req.method == "GET" && (clean_path == "vue.esm-browser.prod.js" || clean_path == "vue.js") {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", VUE_JS.as_bytes(), cors_origin);
        return;
    }

    if req.method == "GET" && clean_path == "style.css" {
        send_response(stream, 200, "OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes(), cors_origin);
        return;
    }

    // Static asset from notes directory (images, svgs, stylesheets, etc.)
    let asset_path = if clean_path.starts_with("assets/") {
        ctx.notes_dir.join(clean_path.strip_prefix("assets/").unwrap_or(clean_path))
    } else {
        ctx.notes_dir.join(clean_path)
    };

    if asset_path.is_file() {
        if let Ok(bytes) = fs::read(&asset_path) {
            let ext = asset_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            let mime = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "svg" => "image/svg+xml",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "css" => "text/css; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "json" => MIME_JSON,
                _ => "application/octet-stream",
            };
            send_response(stream, 200, "OK", mime, &bytes, cors_origin);
            return;
        }
    }

    send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
}
