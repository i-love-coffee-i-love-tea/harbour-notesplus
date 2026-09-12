use std::fs;
use std::io::Write;

use crate::constants::MIME_HTML;
use crate::server::http::{send_response, ParsedHttpRequest};
use crate::server::web_assets::{
    APP_JS, ICON_PNG, INDEX_HTML, STYLE_CSS, VUE_JS,
    COMPOSABLE_UTILS_JS, COMPOSABLE_USE_AUTH_JS, COMPOSABLE_USE_HEALTH_CHECK_JS,
    COMPOSABLE_USE_PRESENTATION_JS, COMPOSABLE_USE_LINK_MODAL_JS,
    COMPOSABLE_USE_AI_ASSISTANT_JS, COMPOSABLE_USE_IMPORT_JS,
};
use crate::server::ServerContext;

pub fn handle_static_asset<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if req.method != "GET" {
        send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
        return;
    }

    if clean_path.is_empty() || clean_path == "index.html" {
        send_response(stream, 200, "OK", MIME_HTML, INDEX_HTML.as_bytes(), cors_origin);
        return;
    }

    if clean_path == "icon.png" || clean_path == "favicon.ico" {
        send_response(stream, 200, "OK", "image/png", ICON_PNG, cors_origin);
        return;
    }

    if clean_path == "app.js" {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", APP_JS.as_bytes(), cors_origin);
        return;
    }

    if clean_path == "vue.esm-browser.prod.js" || clean_path == "vue.js" {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", VUE_JS.as_bytes(), cors_origin);
        return;
    }

    if clean_path == "style.css" {
        send_response(stream, 200, "OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes(), cors_origin);
        return;
    }

    // Composable modules
    let composable_content = match clean_path {
        "composables/utils.js" => Some(COMPOSABLE_UTILS_JS),
        "composables/useAuth.js" => Some(COMPOSABLE_USE_AUTH_JS),
        "composables/useHealthCheck.js" => Some(COMPOSABLE_USE_HEALTH_CHECK_JS),
        "composables/usePresentation.js" => Some(COMPOSABLE_USE_PRESENTATION_JS),
        "composables/useLinkModal.js" => Some(COMPOSABLE_USE_LINK_MODAL_JS),
        "composables/useAiAssistant.js" => Some(COMPOSABLE_USE_AI_ASSISTANT_JS),
        "composables/useImport.js" => Some(COMPOSABLE_USE_IMPORT_JS),
        _ => None,
    };
    if let Some(content) = composable_content {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", content.as_bytes(), cors_origin);
        return;
    }

    let rel_path = if clean_path.starts_with("assets/") {
        clean_path.strip_prefix("assets/").unwrap_or("")
    } else {
        clean_path
    };

    if rel_path.is_empty() || rel_path.contains("..") {
        send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
        return;
    }

    let asset_path = ctx.notes_dir.join(rel_path);

    if let Ok(canonical_asset) = asset_path.canonicalize() {
        if let Ok(canonical_notes_dir) = ctx.notes_dir.canonicalize() {
            if canonical_asset.starts_with(&canonical_notes_dir) {
                // Ensure asset is NOT inside notes_subdir (which contains note source documents)
                if let Ok(canonical_notes_subdir) = ctx.notes_subdir.canonicalize() {
                    if canonical_asset.starts_with(&canonical_notes_subdir) {
                        send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
                        return;
                    }
                }

                // Disallow sensitive file extensions from being served as static assets
                let ext = canonical_asset.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                if ext == "adoc" || ext == "db" || ext == "sqlite" || ext == "json" || ext == "key" || ext == "pem" || ext == "shm" || ext == "wal" {
                    send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
                    return;
                }

                if canonical_asset.is_file() {
                    if let Ok(bytes) = fs::read(&canonical_asset) {
                        let mime = match ext.as_str() {
                            "jpg" | "jpeg" => "image/jpeg",
                            "png" => "image/png",
                            "svg" => "image/svg+xml",
                            "gif" => "image/gif",
                            "webp" => "image/webp",
                            "ico" => "image/x-icon",
                            "css" => "text/css; charset=utf-8",
                            "js" => "application/javascript; charset=utf-8",
                            "woff" => "font/woff",
                            "woff2" => "font/woff2",
                            "ttf" => "font/ttf",
                            "pdf" => "application/pdf",
                            _ => "application/octet-stream",
                        };
                        send_response(stream, 200, "OK", mime, &bytes, cors_origin);
                        return;
                    }
                }
            }
        }
    }

    send_response(stream, 404, "Not Found", MIME_HTML, b"<h1>404 Not Found</h1><p><a href=\"/\">Return to Notes++ Editor</a></p>", cors_origin);
}
