use std::fs;
use std::io::Write;
use std::path::Path;
use serde_json::json;
use rusqlite::Connection;

use crate::block::Block;
use crate::constants::{MIME_HTML, MIME_JSON, MIME_TEXT_PLAIN};
use crate::html::{adoc_to_html5, adoc_to_html_body, blocks_to_html_body};
use crate::page;
use crate::page::ensure_adoc_extension;
use crate::parser;
use crate::server::http::{
    escape_html, send_attachment_response, send_json_error, send_json_ok, send_response,
    ParsedHttpRequest,
};
use crate::server::web_assets::INDEX_HTML;
use crate::server::ServerContext;

pub fn list_all_notes_json(notes_dir: &Path, search_query: Option<&str>) -> String {
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(notes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let mut title = name.strip_suffix(".adoc").unwrap_or(name).to_string();
                    let mut snippet = String::new();
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("= ") {
                                title = trimmed.trim_start_matches("= ").trim().to_string();
                            } else if snippet.is_empty() && !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with(':') {
                                snippet = trimmed.chars().take(120).collect();
                            }
                        }
                    }
                    notes.push((title, name.to_string(), snippet));
                }
            }
        }
    }

    notes.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    if let Some(q) = search_query {
        let q_lower = q.to_lowercase();
        notes.retain(|(title, name, snippet)| {
            title.to_lowercase().contains(&q_lower)
                || name.to_lowercase().contains(&q_lower)
                || snippet.to_lowercase().contains(&q_lower)
        });
    }

    let json_items: Vec<serde_json::Value> = notes
        .into_iter()
        .map(|(title, filename, snippet)| {
            json!({
                "title": title,
                "filename": filename,
                "snippet": snippet
            })
        })
        .collect();

    serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string())
}

pub fn make_slug_filename(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-');
    let final_slug = if trimmed.is_empty() { "untitled" } else { trimmed };
    format!("{}.adoc", final_slug)
}

pub fn extract_title_from_adoc(content: &str, fallback_filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("= ") {
            return trimmed.trim_start_matches("= ").trim().to_string();
        }
    }
    fallback_filename.trim_end_matches(".adoc").replace('_', " ")
}

pub fn render_web_page_html(adoc_content: &str, title: &str, notes_dir: &Path, filename: &str) -> String {
    let standalone = adoc_to_html5(adoc_content, title, Some(notes_dir));

    let top_bar = format!(
        r#"<div class="web-page-topbar">
            <div class="topbar-left">
                <a class="nav-btn" href="/">&larr; Notes++ Web Editor</a>
                <span class="page-current_title">{}</span>
            </div>
            <div class="topbar-right">
                <a class="action-btn" href="/raw/{}" target="_blank">Raw AsciiDoc</a>
                <a class="action-btn primary" href="/export/{}">Download HTML5</a>
            </div>
        </div>"#,
        escape_html(title),
        filename,
        filename
    );

    standalone.replacen(
        "<body class=\"notes-body\">",
        &format!("<body class=\"notes-body with-topbar\">\n{}", top_bar),
        1
    )
}

pub fn handle_pages_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let conn = match Connection::open(&ctx.db_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Database connection failed: {}", e);
            send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
            return;
        }
    };

    match req.method.as_str() {
        "GET" => match page::list_pages(&conn) {
            Ok(pages) => {
                let json_items: Vec<serde_json::Value> = pages.iter().map(|p| p.to_json_value()).collect();
                send_response(stream, 200, "OK", MIME_JSON, serde_json::to_string(&json_items).unwrap_or_default().as_bytes(), cors_origin);
            }
            Err(e) => {
                send_json_error(stream, 500, "Internal Server Error", &e, cors_origin);
            }
        },
        "POST" => {
            let parsed = req.json_body();
            let name = parsed.get("name").or_else(|| parsed.get("title")).and_then(|v| v.as_str()).unwrap_or("").trim();
            let is_journal = parsed.get("is_journal").and_then(|v| v.as_bool()).unwrap_or(false);

            if name.is_empty() {
                send_json_error(stream, 400, "Bad Request", "Page name cannot be empty", cors_origin);
                return;
            }

            match page::create_page(&conn, &ctx.notes_subdir, name, is_journal) {
                Ok(info) => {
                    send_response(stream, 201, "Created", MIME_JSON, info.to_json_value().to_string().as_bytes(), cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 400, "Bad Request", &e, cors_origin);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_page_detail_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    filename: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let filename = ensure_adoc_extension(filename);

    match req.method.as_str() {
        "GET" => {
            let file_path = ctx.notes_subdir.join(&filename);
            if !file_path.is_file() {
                send_json_error(stream, 404, "Not Found", &format!("Page '{}' not found", filename), cors_origin);
                return;
            }

            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    let title = extract_title_from_adoc(&content, &filename);
                    let blocks = parser::parse_blocks(&content);
                    let body_html = blocks_to_html_body(&blocks, Some(&ctx.notes_dir));
                    let resp = json!({
                        "filename": filename,
                        "title": title,
                        "raw": content,
                        "html": body_html,
                        "blocks": blocks
                    });
                    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                }
                Err(e) => {
                    log::error!("Failed to read page file: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                }
            }
        }
        "PUT" => {
            let parsed = req.json_body();
            
            // Check if block index update was requested
            if let Some(block_idx) = parsed.get("block_index").and_then(|v| v.as_i64()) {
                let count = parsed.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as usize;
                let raw_block = parsed.get("raw").and_then(|v| v.as_str()).unwrap_or("");
                let file_path = ctx.notes_subdir.join(&filename);

                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        let mut blocks = parser::parse_blocks(&content);
                        let idx = block_idx as usize;
                        if idx <= blocks.len() {
                            let end_idx = (idx + count).min(blocks.len());
                            let new_blocks = parser::parse_blocks(raw_block);
                            blocks.splice(idx..end_idx, new_blocks);
                            let new_content = parser::blocks_to_adoc(&blocks);
                            if let Err(e) = fs::write(&file_path, &new_content) {
                                log::error!("Failed to write page: {}", e);
                                send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                                return;
                            }
                            if let Ok(conn) = Connection::open(&ctx.db_path) {
                                let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                            }
                            let resp = json!({ "ok": true, "filename": filename });
                            send_json_ok(stream, &resp, cors_origin);
                            return;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read page file: {}", e);
                        send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                        return;
                    }
                }
            }

            // Full content update
            let new_content = if let Some(content) = parsed.get("content").and_then(|v| v.as_str()) {
                content.to_string()
            } else {
                String::from_utf8_lossy(&req.body).into_owned()
            };

            let file_path = ctx.notes_subdir.join(&filename);
            match fs::write(&file_path, &new_content) {
                Ok(_) => {
                    if let Ok(conn) = Connection::open(&ctx.db_path) {
                        let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                    }
                    let resp = json!({ "ok": true, "filename": filename });
                    send_json_ok(stream, &resp, cors_origin);
                }
                Err(e) => {
                    log::error!("Failed to write page: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                }
            }
        }
        "DELETE" => {
            if let Ok(conn) = Connection::open(&ctx.db_path) {
                match page::delete_page(&conn, &ctx.notes_subdir, &filename) {
                    Ok(_) => {
                        let resp = json!({ "ok": true });
                        send_json_ok(stream, &resp, cors_origin);
                    }
                    Err(e) => {
                        send_json_error(stream, 400, "Bad Request", &e, cors_origin);
                    }
                }
            } else {
                send_json_error(stream, 500, "Internal Server Error", "Database connection failed", cors_origin);
            }
        }
        _ => {}
    }
}

pub fn handle_search_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let q = req.query.as_deref().unwrap_or("");
    let query_val = q
        .split('&')
        .find_map(|p| p.strip_prefix("q="))
        .unwrap_or("");

    if let Ok(conn) = Connection::open(&ctx.db_path) {
        match crate::search::search_pages(&conn, query_val) {
            Ok(results) => {
                let json_items: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.page.id,
                            "title": r.page.title,
                            "filename": r.page.filename,
                            "is_journal": r.page.is_journal,
                            "snippet": r.snippet,
                        })
                    })
                    .collect();
                send_response(stream, 200, "OK", MIME_JSON, serde_json::to_string(&json_items).unwrap_or_default().as_bytes(), cors_origin);
                return;
            }
            Err(e) => {
                send_json_error(stream, 500, "Internal Server Error", &e, cors_origin);
                return;
            }
        }
    }
    send_json_error(stream, 500, "Internal Server Error", "Database connection failed", cors_origin);
}

pub fn handle_notes_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if clean_path == "api/notes" {
        match req.method.as_str() {
            "GET" => {
                let q_param = req.query.as_deref().and_then(|q| {
                    q.split('&').find_map(|p| p.strip_prefix("q="))
                });
                let json_str = list_all_notes_json(&ctx.notes_subdir, q_param);
                send_response(stream, 200, "OK", MIME_JSON, json_str.as_bytes(), cors_origin);
                return;
            }
            "POST" => {
                let json_val = req.json_body();
                let title = json_val.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Note");
                let content = json_val.get("content").and_then(|v| v.as_str()).unwrap_or("");

                let filename = make_slug_filename(title);
                let file_path = ctx.notes_subdir.join(&filename);

                let initial_content = if content.is_empty() {
                    format!("= {}\n\n", title)
                } else {
                    content.to_string()
                };

                if let Err(e) = fs::write(&file_path, &initial_content) {
                    log::error!("Failed to create note: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                    return;
                }

                if let Ok(conn) = Connection::open(&ctx.db_path) {
                    let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                }

                let resp = json!({
                    "title": title,
                    "filename": filename,
                    "content": initial_content
                });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                return;
            }
            _ => {}
        }
    }

    if req.method == "POST" && clean_path.starts_with("api/notes/") && clean_path.ends_with("/toggle") {
        let raw_name = clean_path
            .strip_prefix("api/notes/")
            .unwrap()
            .strip_suffix("/toggle")
            .unwrap();
        let filename = ensure_adoc_extension(raw_name);
        let file_path = ctx.notes_subdir.join(&filename);
        if file_path.is_file() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                let json_body = req.json_body();
                let block_idx = json_body.get("block_index").and_then(|v| v.as_u64()).map(|v| v as usize);
                let item_idx = json_body.get("item_index").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(0);
                let target_checked = json_body.get("checked").and_then(|v| v.as_bool());

                let mut blocks = parser::parse_blocks(&content);
                let mut updated = false;

                if let Some(b_idx) = block_idx {
                    if b_idx < blocks.len() {
                        if let Block::UnorderedListItem { ref mut checked, ref mut raw, .. } = blocks[b_idx] {
                            let new_val = target_checked.unwrap_or_else(|| !checked.unwrap_or(false));
                            *checked = Some(new_val);
                            if new_val {
                                *raw = raw.replacen("[ ]", "[x]", 1).replacen("[*]", "[x]", 1);
                            } else {
                                *raw = raw.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1).replacen("[*]", "[ ]", 1);
                            }
                            updated = true;
                        }
                    }
                } else {
                    let mut check_count = 0;
                    for b in &mut blocks {
                        if let Block::UnorderedListItem { ref mut checked, ref mut raw, .. } = b {
                            if checked.is_some() {
                                if check_count == item_idx {
                                    let new_val = target_checked.unwrap_or_else(|| !checked.unwrap_or(false));
                                    *checked = Some(new_val);
                                    if new_val {
                                        *raw = raw.replacen("[ ]", "[x]", 1).replacen("[*]", "[x]", 1);
                                    } else {
                                        *raw = raw.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1).replacen("[*]", "[ ]", 1);
                                    }
                                    updated = true;
                                    break;
                                }
                                check_count += 1;
                            }
                        }
                    }
                }

                if updated {
                    let new_adoc = parser::blocks_to_adoc(&blocks);
                    let _ = fs::write(&file_path, &new_adoc);
                    if let Ok(conn) = Connection::open(&ctx.db_path) {
                        let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                    }
                    let resp = json!({ "ok": true });
                    send_json_ok(stream, &resp, cors_origin);
                    return;
                }
            }
        }
        send_json_error(stream, 400, "Bad Request", "Failed to toggle checklist item", cors_origin);
        return;
    }

    if clean_path.starts_with("api/notes/") {
        let note_name = clean_path.strip_prefix("api/notes/").unwrap_or("");
        let filename = ensure_adoc_extension(note_name);

        let file_path = ctx.notes_subdir.join(&filename);

        match req.method.as_str() {
            "GET" => {
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        send_response(stream, 200, "OK", MIME_TEXT_PLAIN, content.as_bytes(), cors_origin);
                        return;
                    }
                }
                send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", filename), cors_origin);
                return;
            }
            "PUT" => {
                let body_str = String::from_utf8_lossy(&req.body);
                let content = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    val.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or(body_str.to_string())
                } else {
                    body_str.to_string()
                };

                if let Err(e) = fs::write(&file_path, &content) {
                    log::error!("Failed to write note: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                    return;
                }

                if let Ok(conn) = Connection::open(&ctx.db_path) {
                    let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                }

                let resp = json!({ "ok": true, "filename": filename });
                send_json_ok(stream, &resp, cors_origin);
                return;
            }
            "DELETE" => {
                if file_path.is_file() {
                    let _ = fs::remove_file(&file_path);
                    if let Ok(conn) = Connection::open(&ctx.db_path) {
                        let _ = page::sync_and_index_pages(&conn, &ctx.notes_subdir);
                    }
                    let resp = json!({ "ok": true });
                    send_json_ok(stream, &resp, cors_origin);
                    return;
                }
                send_json_error(stream, 404, "Not Found", "File not found", cors_origin);
                return;
            }
            _ => {}
        }
    }
}

pub fn handle_render_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_val = req.json_body();
    let content = json_val.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let is_standalone = json_val.get("standalone").and_then(|v| v.as_bool()).unwrap_or(false);
    let title = json_val.get("title").and_then(|v| v.as_str()).unwrap_or("Rendered Document");

    let html = if is_standalone {
        adoc_to_html5(content, title, Some(&ctx.notes_dir))
    } else {
        adoc_to_html_body(content, Some(&ctx.notes_dir))
    };

    send_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), cors_origin);
}

pub fn handle_blocks_parse_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let body_str = String::from_utf8_lossy(&req.body);
    let content = if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
        json_body.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        body_str.to_string()
    };

    let blocks = parser::parse_blocks(&content);
    let block_items: Vec<serde_json::Value> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let html = blocks_to_html_body(&[b.clone()], Some(&ctx.notes_dir));
            json!({
                "index": i,
                "raw": b.raw_text(),
                "html": html
            })
        })
        .collect();

    let resp = json!({ "blocks": block_items });
    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
}

pub fn handle_blocks_to_adoc_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    cors_origin: &str,
) {
    let json_val = req.json_body();

    let blocks_res = json_val
        .get("blocks")
        .and_then(|v| serde_json::from_value::<Vec<Block>>(v.clone()).ok())
        .or_else(|| serde_json::from_slice::<Vec<Block>>(&req.body).ok());

    if let Some(blocks) = blocks_res {
        let adoc = parser::blocks_to_adoc(&blocks);
        let resp = json!({ "adoc": adoc });
        send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        return;
    }

    send_json_error(stream, 400, "Bad Request", "Invalid blocks payload", cors_origin);
}

pub fn handle_export_html_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let filename = if req.method == "POST" {
        let json_val = req.json_body();
        json_val.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        req.query.as_deref().and_then(|q| {
            q.split('&').find_map(|p| p.strip_prefix("filename="))
        }).unwrap_or("").to_string()
    };

    if filename.is_empty() {
        send_json_error(stream, 400, "Bad Request", "filename query parameter or JSON property required", cors_origin);
        return;
    }

    let adoc_filename = ensure_adoc_extension(&filename);

    let file_path = ctx.notes_subdir.join(&adoc_filename);
    if !file_path.is_file() {
        send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", adoc_filename), cors_origin);
        return;
    }

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            let title = adoc_filename.strip_suffix(".adoc").unwrap_or(&adoc_filename);
            let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
            send_attachment_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), &format!("{}.html", title), cors_origin);
        }
        Err(e) => {
            log::error!("Failed to read file for export: {}", e);
            send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
        }
    }
}

pub fn handle_export_all_api<W: Write>(
    stream: &mut W,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let export_dir = ctx.notes_dir.parent().unwrap_or(&ctx.notes_dir).join("exports");
    match crate::html::export_all_pages_to_html5(&ctx.notes_subdir, &ctx.notes_dir, &export_dir) {
        Ok(paths) => {
            let files: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
            let resp = json!({
                "ok": true,
                "exported_dir": export_dir.to_string_lossy(),
                "files_count": files.len(),
                "files": files
            });
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        Err(e) => {
            log::error!("Export all failed: {}", e);
            send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
        }
    }
}

pub fn handle_page_url<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let note_name = clean_path
        .strip_prefix("page/")
        .or_else(|| clean_path.strip_prefix("notes/"))
        .or_else(|| clean_path.strip_prefix("edit/"))
        .unwrap_or("");

    let filename = ensure_adoc_extension(note_name);

    if let Some(q) = req.query.as_deref() {
        if q.contains("export=1") || q.contains("download=1") {
            let file_path = ctx.notes_subdir.join(&filename);
            if file_path.is_file() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                    let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
                    send_attachment_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), &format!("{}.html", title), cors_origin);
                    return;
                }
            }
        } else if q.contains("view=rendered") {
            let file_path = ctx.notes_subdir.join(&filename);
            if file_path.is_file() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
                    let html = render_web_page_html(&content, title, &ctx.notes_dir, &filename);
                    send_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), cors_origin);
                    return;
                }
            }
        }
    }

    send_response(stream, 200, "OK", MIME_HTML, INDEX_HTML.as_bytes(), cors_origin);
}

pub fn handle_raw_url<W: Write>(
    stream: &mut W,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let note_name = clean_path.strip_prefix("raw/").unwrap_or("");
    let filename = ensure_adoc_extension(note_name);

    let file_path = ctx.notes_subdir.join(&filename);
    if file_path.is_file() {
        if let Ok(content) = fs::read_to_string(&file_path) {
            send_response(stream, 200, "OK", MIME_TEXT_PLAIN, content.as_bytes(), cors_origin);
            return;
        }
    }
    send_json_error(stream, 404, "Not Found", "File not found", cors_origin);
}

pub fn handle_export_url<W: Write>(
    stream: &mut W,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let note_name = clean_path.strip_prefix("export/").unwrap_or("");
    let filename = if note_name.ends_with(".html") {
        ensure_adoc_extension(note_name.strip_suffix(".html").unwrap_or(note_name))
    } else {
        ensure_adoc_extension(note_name)
    };

    let file_path = ctx.notes_subdir.join(&filename);
    if file_path.is_file() {
        if let Ok(content) = fs::read_to_string(&file_path) {
            let title = filename.strip_suffix(".adoc").unwrap_or(&filename);
            let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
            send_attachment_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), &format!("{}.html", title), cors_origin);
            return;
        }
    }
    send_json_error(stream, 404, "Not Found", "File not found", cors_origin);
}
