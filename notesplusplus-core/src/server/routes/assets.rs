use std::fs;
use std::io::Write;

use crate::constants::MIME_HTML;
use crate::server::http::{send_response, ParsedHttpRequest};
use crate::server::web_assets::{
    APP_JS, ICON_PNG, INDEX_HTML, STYLE_CSS, VUE_JS, PINIA_JS,
    STORES_INDEX_JS, STORES_API_JS, STORES_NOTES_JS, STORES_AUTH_JS,
    STORES_THEME_JS, STORES_HEALTH_JS, STORES_AI_JS, STORES_UI_JS,
    STORES_PRESENTATION_STORE_JS, STORES_IMPORT_STORE_JS,
    STORES_LINK_STORE_JS, STORES_EDITOR_STORE_JS,
    COMPOSABLE_UTILS_JS,
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

    if clean_path == "pinia.esm-browser.prod.js" || clean_path == "pinia.js" {
        send_response(stream, 200, "OK", "application/javascript; charset=utf-8", PINIA_JS.as_bytes(), cors_origin);
        return;
    }

    if clean_path == "style.css" {
        send_response(stream, 200, "OK", "text/css; charset=utf-8", STYLE_CSS.as_bytes(), cors_origin);
        return;
    }

    // Store modules & shared utilities
    let module_content = match clean_path {
        "stores/index.js" => Some(STORES_INDEX_JS),
        "stores/api.js" => Some(STORES_API_JS),
        "stores/notes.js" => Some(STORES_NOTES_JS),
        "stores/auth.js" => Some(STORES_AUTH_JS),
        "stores/theme.js" => Some(STORES_THEME_JS),
        "stores/health.js" => Some(STORES_HEALTH_JS),
        "stores/ai.js" => Some(STORES_AI_JS),
        "stores/ui.js" => Some(STORES_UI_JS),
        "stores/presentationStore.js" => Some(STORES_PRESENTATION_STORE_JS),
        "stores/importStore.js" => Some(STORES_IMPORT_STORE_JS),
        "stores/linkStore.js" => Some(STORES_LINK_STORE_JS),
        "stores/editorStore.js" => Some(STORES_EDITOR_STORE_JS),
        "composables/utils.js" => Some(COMPOSABLE_UTILS_JS),
        _ => None,
    };
    if let Some(content) = module_content {
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

    let asset_path = ctx.assets_dir.join(rel_path);

    if let Ok(canonical_asset) = asset_path.canonicalize() {
        if let Ok(canonical_assets_dir) = ctx.assets_dir.canonicalize() {
            if canonical_asset.starts_with(&canonical_assets_dir) {
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
