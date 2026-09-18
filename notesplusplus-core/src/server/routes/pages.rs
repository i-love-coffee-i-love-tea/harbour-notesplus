use std::io::Write;
use std::path::Path;
use serde_json::json;

use crate::block::Block;
use crate::constants::{MIME_HTML, MIME_JSON, MIME_TEXT_PLAIN, API_ROUTE_GROUPS_PREFIX, API_ROUTE_NOTES, API_ROUTE_NOTES_PREFIX};
use crate::html::{adoc_to_html5, adoc_to_html_body, blocks_to_html_body};
use crate::page;
use crate::page::sanitize_note_filename;
use crate::parser;
use crate::server::http::{
    escape_html, send_attachment_response, send_json_error, send_json_ok, send_response,
    ParsedHttpRequest,
};
use crate::server::web_assets::INDEX_HTML;
use crate::server::ServerContext;

pub fn list_all_notes_json(notes_dir: &Path, search_query: Option<&str>) -> String {
    list_all_notes_json_with_db(notes_dir, None, search_query)
}

pub fn list_all_notes_json_with_db(notes_dir: &Path, db_path: Option<&Path>, search_query: Option<&str>) -> String {
    let conn = if let Some(db_p) = db_path {
        crate::db::open_db(db_p).ok()
    } else {
        rusqlite::Connection::open_in_memory().ok().and_then(|c| {
            let _ = crate::db::init_schema(&c);
            let _ = page::sync_and_index_pages(&c, notes_dir);
            Some(c)
        })
    };

    if let Some(conn) = conn {
        let q_trimmed = search_query.unwrap_or("").trim();
        if !q_trimmed.is_empty() {
            if let Ok(results) = crate::search::search_pages(&conn, q_trimmed) {
                let json_items: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|r| {
                        json!({
                            "id": r.page.id,
                            "title": r.page.title,
                            "name": r.page.title,
                            "filename": r.page.filename,
                            "group_path": r.page.group_path,
                            "full_path": r.page.full_path(),
                            "color": r.page.effective_color(),
                            "custom_color": r.page.color,
                            "snippet": r.snippet
                        })
                    })
                    .collect();
                return serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string());
            }
        } else if let Ok(pages) = page::list_pages(&conn) {
            let json_items: Vec<serde_json::Value> = pages
                .into_iter()
                .map(|p| {
                    json!({
                        "id": p.id,
                        "title": p.title,
                        "name": p.title,
                        "filename": p.filename,
                        "group_path": p.group_path,
                        "full_path": p.full_path(),
                        "color": p.effective_color(),
                        "custom_color": p.color,
                        "snippet": ""
                    })
                })
                .collect();
            return serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string());
        }
    }

    "[]".to_string()
}

pub fn render_web_page_html(adoc_content: &str, title: &str, notes_dir: &Path, filename: &str) -> String {
    let standalone = adoc_to_html5(adoc_content, title, Some(notes_dir));
    let clean_filename = filename.strip_suffix(".adoc").unwrap_or(filename);

    let top_bar = format!(
        r#"<div class="web-page-topbar">
            <div class="topbar-left">
                <a class="nav-btn" href="/">&larr; Notes Plus Web Editor</a>
                <span class="page-current_title">{}</span>
            </div>
            <div class="topbar-right">
                <a class="action-btn" href="/raw/{}" target="_blank">Raw AsciiDoc</a>
                <a class="action-btn" href="/export/{}.pdf">Download PDF</a>
                <a class="action-btn primary" href="/export/{}">Download HTML5</a>
            </div>
        </div>"#,
        escape_html(title),
        filename,
        clean_filename,
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
    match req.method.as_str() {
        "GET" => match ctx.repository.list_pages() {
            Ok(pages) => {
                let json_items: Vec<serde_json::Value> = pages.iter().map(|p| p.to_json_value()).collect();
                send_response(stream, 200, "OK", MIME_JSON, serde_json::to_string(&json_items).unwrap_or_default().as_bytes(), cors_origin);
            }
            Err(e) => {
                send_json_error(stream, 500, "Internal Server Error", &e.to_string(), cors_origin);
            }
        },
        "POST" => {
            let parsed = req.json_body();
            let name = parsed.get("name").or_else(|| parsed.get("title")).and_then(|v| v.as_str()).unwrap_or("").trim();
            let is_journal = parsed.get("is_journal").and_then(|v| v.as_bool()).unwrap_or(false);
            let color = parsed.get("color").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());

            if name.is_empty() {
                send_json_error(stream, 400, "Bad Request", "Page name cannot be empty", cors_origin);
                return;
            }

            match ctx.repository.create_page(name, is_journal, color) {
                Ok(info) => {
                    send_response(stream, 201, "Created", MIME_JSON, info.to_json_value().to_string().as_bytes(), cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                }
            }
        }
        _ => { send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin); }
    }
}

pub fn handle_page_detail_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    filename: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if filename.ends_with("/pdf") || req.query.as_deref().map(|q| q.contains("export=pdf")).unwrap_or(false) {
        let note_clean = filename.strip_suffix("/pdf").unwrap_or(filename);
        handle_pdf_export_response(stream, ctx, note_clean, cors_origin);
        return;
    }

    if filename.ends_with("/color") {
        let base_name = filename.strip_suffix("/color").unwrap();
        match req.method.as_str() {
            "GET" => {
                if let Ok(Some(p)) = ctx.repository.get_page(base_name) {
                    let resp = json!({
                        "filename": p.filename,
                        "title": p.title,
                        "color": p.effective_color(),
                        "custom_color": p.color
                    });
                    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                } else {
                    send_json_error(stream, 404, "Not Found", &format!("Page '{}' not found", base_name), cors_origin);
                }
                return;
            }
            "PUT" | "POST" => {
                let parsed = req.json_body();
                let color = parsed.get("color").and_then(|v| v.as_str()).map(|s| s.trim());
                match ctx.repository.set_page_color(base_name, color) {
                    Ok(info) => {
                        send_response(stream, 200, "OK", MIME_JSON, info.to_json_value().to_string().as_bytes(), cors_origin);
                    }
                    Err(e) => {
                        send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                    }
                }
                return;
            }
            _ => {
                send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin);
                return;
            }
        }
    }

    let (group_path, file_name) = page::sanitize_note_path(filename);
    let resolved_path = if group_path.is_empty() {
        file_name
    } else {
        format!("{}/{}", group_path, file_name)
    };
    let filename = if ctx.repository.note_exists(&resolved_path) {
        resolved_path
    } else if let Ok(Some(p)) = ctx.repository.get_page(filename) {
        p.full_path()
    } else {
        resolved_path
    };

    match req.method.as_str() {
        "GET" => {
            if !ctx.repository.note_exists(&filename) {
                send_json_error(stream, 404, "Not Found", &format!("Page '{}' not found", filename), cors_origin);
                return;
            }

            match ctx.repository.read_note_content(&filename) {
                Ok(content) => {
                    let title = page::extract_doc_title(&content, &filename);
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

                match ctx.repository.read_note_content(&filename) {
                    Ok(content) => {
                        let mut blocks = parser::parse_blocks(&content);
                        let idx = block_idx as usize;
                        if idx <= blocks.len() {
                            let end_idx = (idx + count).min(blocks.len());
                            let new_blocks = parser::parse_blocks(raw_block);
                            blocks.splice(idx..end_idx, new_blocks);
                            let new_content = parser::blocks_to_adoc(&blocks);
                            if let Err(e) = ctx.repository.save_note(&filename, &new_content) {
                                log::error!("Failed to write page: {}", e);
                                send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                                return;
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

            match ctx.repository.save_note(&filename, &new_content) {
                Ok(_) => {
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
            match ctx.repository.delete_page(&filename) {
                Ok(_) => {
                    let resp = json!({ "ok": true });
                    send_json_ok(stream, &resp, cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                }
            }
        }
        _ => { send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin); }
    }
}

pub fn handle_search_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let query_val = req.query.as_deref()
        .and_then(|q| {
            q.split('&').find_map(|p| p.strip_prefix("q=").or_else(|| p.strip_prefix("search=")))
        })
        .unwrap_or("")
        .trim();

    match ctx.repository.search_pages(query_val) {
        Ok(results) => {
            let json_items: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    json!({
                        "id": r.page.id,
                        "title": r.page.title,
                        "name": r.page.title,
                        "filename": r.page.filename,
                        "group_path": r.page.group_path,
                        "full_path": r.page.full_path(),
                        "color": page::compute_note_color(&r.page.title),
                        "is_journal": r.page.is_journal,
                        "created_at": r.page.created_at,
                        "updated_at": r.page.updated_at,
                        "block_count": r.page.block_count,
                        "snippet": r.snippet,
                    })
                })
                .collect();
            send_response(stream, 200, "OK", MIME_JSON, serde_json::to_string(&json_items).unwrap_or_default().as_bytes(), cors_origin);
        }
        Err(e) => {
            send_json_error(stream, 500, "Internal Server Error", &e.to_string(), cors_origin);
        }
    }
}

pub fn handle_tree_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if req.method.as_str() != "GET" {
        send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin);
        return;
    }

    let max_depth: i32 = req.query.as_deref()
        .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("depth=")))
        .and_then(|d| d.parse().ok())
        .unwrap_or(0);

    let pages = match ctx.repository.list_pages() {
        Ok(p) => p,
        Err(e) => {
            send_json_error(stream, 500, "Internal Server Error", &e.to_string(), cors_origin);
            return;
        }
    };

    let groups = match ctx.repository.list_groups(None, None) {
        Ok(g) => g,
        Err(e) => {
            send_json_error(stream, 500, "Internal Server Error", &e.to_string(), cors_origin);
            return;
        }
    };

    let tree_json = crate::tree::build_group_tree(
        &pages,
        &groups,
        max_depth,
        Some(&ctx.notes_dir),
        true,
        None,
        None,
    );

    send_response(stream, 200, "OK", MIME_JSON, tree_json.as_bytes(), cors_origin);
}

pub fn handle_notes_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    if clean_path == API_ROUTE_NOTES {
        match req.method.as_str() {
            "GET" => {
                let q_param = req.query.as_deref().and_then(|q| {
                    q.split('&').find_map(|p| p.strip_prefix("q=").or_else(|| p.strip_prefix("search=")))
                });
                let q_trimmed = q_param.unwrap_or("").trim();
                let json_items: Vec<serde_json::Value> = if !q_trimmed.is_empty() {
                    ctx.repository.search_pages(q_trimmed).map(|results| {
                        results.into_iter().map(|r| {
                            json!({
                                "id": r.page.id,
                                "title": r.page.title,
                                "name": r.page.title,
                                "filename": r.page.filename,
                                "group_path": r.page.group_path,
                                "full_path": r.page.full_path(),
                                "color": r.page.effective_color(),
                                "custom_color": r.page.color,
                                "snippet": r.snippet
                            })
                        }).collect()
                    }).unwrap_or_default()
                } else {
                    ctx.repository.list_pages().map(|pages| {
                        pages.into_iter().map(|p| {
                            json!({
                                "id": p.id,
                                "title": p.title,
                                "name": p.title,
                                "filename": p.filename,
                                "group_path": p.group_path,
                                "full_path": p.full_path(),
                                "color": p.effective_color(),
                                "custom_color": p.color,
                                "snippet": ""
                            })
                        }).collect()
                    }).unwrap_or_default()
                };
                let json_str = serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string());
                send_response(stream, 200, "OK", MIME_JSON, json_str.as_bytes(), cors_origin);
                return;
            }
            "POST" => {
                let json_val = req.json_body();
                let title = json_val.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Note");
                let content = json_val.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let color = json_val.get("color").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());

                let filename = sanitize_note_filename(title);

                let initial_content = if content.is_empty() {
                    format!("= {}\n\n", title)
                } else {
                    content.to_string()
                };

                match ctx.repository.save_note(&filename, &initial_content) {
                    Ok(mut info) => {
                        if let Some(c) = color {
                            if let Ok(updated) = ctx.repository.set_page_color(&filename, Some(c)) {
                                info = updated;
                            }
                        }
                        let resp = json!({
                            "title": info.title,
                            "filename": info.filename,
                            "color": info.effective_color(),
                            "custom_color": info.color,
                            "content": initial_content
                        });
                        send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                    }
                    Err(e) => {
                        log::error!("Failed to create note: {}", e);
                        send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                    }
                }
                return;
            }
            _ => {}
        }
    }

    if clean_path.starts_with(API_ROUTE_NOTES_PREFIX) && clean_path.ends_with("/color") {
        let raw_name = clean_path
            .strip_prefix(API_ROUTE_NOTES_PREFIX)
            .unwrap()
            .strip_suffix("/color")
            .unwrap();
        match req.method.as_str() {
            "GET" => {
                if let Ok(Some(p)) = ctx.repository.get_page(raw_name) {
                    let resp = json!({
                        "filename": p.filename,
                        "title": p.title,
                        "color": p.effective_color(),
                        "custom_color": p.color
                    });
                    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
                } else {
                    send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", raw_name), cors_origin);
                }
                return;
            }
            "PUT" | "POST" => {
                let json_body = req.json_body();
                let color = json_body.get("color").and_then(|v| v.as_str()).map(|s| s.trim());
                match ctx.repository.set_page_color(raw_name, color) {
                    Ok(info) => {
                        send_response(stream, 200, "OK", MIME_JSON, info.to_json_value().to_string().as_bytes(), cors_origin);
                    }
                    Err(e) => {
                        send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                    }
                }
                return;
            }
            _ => {
                send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin);
                return;
            }
        }
    }

    if req.method == "POST" && clean_path.starts_with(API_ROUTE_NOTES_PREFIX) && clean_path.ends_with("/toggle") {
        let raw_name = clean_path
            .strip_prefix(API_ROUTE_NOTES_PREFIX)
            .unwrap()
            .strip_suffix("/toggle")
            .unwrap();
        if ctx.repository.note_exists(raw_name) {
            if let Ok(content) = ctx.repository.read_note_content(raw_name) {
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
                    if let Ok(_) = ctx.repository.save_note(raw_name, &new_adoc) {
                        let resp = json!({ "ok": true });
                        send_json_ok(stream, &resp, cors_origin);
                        return;
                    }
                }
            }
        }
        send_json_error(stream, 400, "Bad Request", "Failed to toggle checklist item", cors_origin);
        return;
    }

    if clean_path.starts_with(API_ROUTE_NOTES_PREFIX) {
        let note_name = clean_path.strip_prefix(API_ROUTE_NOTES_PREFIX).unwrap_or("");

        match req.method.as_str() {
            "GET" => {
                if let Ok(content) = ctx.repository.read_note_content(note_name) {
                    send_response(stream, 200, "OK", MIME_TEXT_PLAIN, content.as_bytes(), cors_origin);
                    return;
                }
                send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", note_name), cors_origin);
                return;
            }
            "PUT" => {
                let body_str = String::from_utf8_lossy(&req.body);
                let content = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    val.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or(body_str.to_string())
                } else {
                    body_str.to_string()
                };

                if let Err(e) = ctx.repository.save_note(note_name, &content) {
                    log::error!("Failed to write note: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
                    return;
                }

                let resp = json!({ "ok": true, "filename": note_name });
                send_json_ok(stream, &resp, cors_origin);
                return;
            }
            "DELETE" => {
                match ctx.repository.delete_page(note_name) {
                    Ok(_) => {
                        let resp = json!({ "ok": true });
                        send_json_ok(stream, &resp, cors_origin);
                        return;
                    }
                    Err(_) => {
                        send_json_error(stream, 404, "Not Found", "File not found", cors_origin);
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn handle_groups_api<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    clean_path: &str,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    match req.method.as_str() {
        "GET" => {
            match ctx.repository.list_groups(None, None) {
                Ok(groups) => {
                    let json_body = serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string());
                    send_response(stream, 200, "OK", MIME_JSON, json_body.as_bytes(), cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 500, "Internal Server Error", &e.to_string(), cors_origin);
                }
            }
        }
        "POST" => {
            let json_val = req.json_body();
            let name = json_val.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let parent = json_val.get("parent").and_then(|v| v.as_str()).unwrap_or("").trim();
            let note_sort = json_val.get("note_sort").and_then(|v| v.as_str()).map(|s| s.trim());

            if name.is_empty() {
                send_json_error(stream, 400, "Bad Request", "Group name is required", cors_origin);
                return;
            }

            match ctx.repository.create_group(parent, name) {
                Ok(mut group) => {
                    if let Some(sort_str) = note_sort {
                        let sort_order = sort_str.parse::<crate::group::NoteSortOrder>().unwrap_or_default();
                        if let Ok(updated_sort) = ctx.repository.set_group_note_sort(&group.path, sort_order) {
                            group.note_sort = updated_sort;
                        }
                    }
                    let resp = json!({ "ok": true, "group": group });
                    send_json_ok(stream, &resp, cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                }
            }
        }
        "DELETE" => {
            let path = clean_path.strip_prefix(API_ROUTE_GROUPS_PREFIX).unwrap_or("").trim();
            if path.is_empty() {
                send_json_error(stream, 400, "Bad Request", "Group path required", cors_origin);
                return;
            }
            let recursive = req.query.as_deref().map(|q| q.contains("recursive=true") || q.contains("recursive=1")).unwrap_or(false);
            match ctx.repository.delete_group(path, recursive) {
                Ok(_) => {
                    let resp = json!({ "ok": true });
                    send_json_ok(stream, &resp, cors_origin);
                }
                Err(e) => {
                    send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                }
            }
        }
        "PUT" => {
            let old_path = clean_path.strip_prefix(API_ROUTE_GROUPS_PREFIX).unwrap_or("").trim();
            let json_val = req.json_body();
            let new_name = json_val.get("new_name").or_else(|| json_val.get("name")).and_then(|v| v.as_str()).unwrap_or("").trim();
            let note_sort = json_val.get("note_sort").and_then(|v| v.as_str()).map(|s| s.trim());

            if old_path.is_empty() || (new_name.is_empty() && note_sort.is_none()) {
                send_json_error(stream, 400, "Bad Request", "old_path and new_name or note_sort required", cors_origin);
                return;
            }

            let mut current_path = old_path.to_string();
            let current_base = old_path.rsplit('/').next().unwrap_or(old_path);
            if !new_name.is_empty() && new_name != current_base {
                match ctx.repository.rename_group(old_path, new_name) {
                    Ok(new_path) => {
                        current_path = new_path;
                    }
                    Err(e) => {
                        send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                        return;
                    }
                }
            }

            if let Some(sort_str) = note_sort {
                let sort_order = sort_str.parse::<crate::group::NoteSortOrder>().unwrap_or_default();
                if let Err(e) = ctx.repository.set_group_note_sort(&current_path, sort_order) {
                    send_json_error(stream, 400, "Bad Request", &e.to_string(), cors_origin);
                    return;
                }
            }

            let resp = json!({ "ok": true, "new_path": current_path, "path": current_path });
            send_json_ok(stream, &resp, cors_origin);
        }
        _ => {
            send_json_error(stream, 405, "Method Not Allowed", "Method not allowed", cors_origin);
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

    let adoc_filename = sanitize_note_filename(&filename);
    if !ctx.repository.note_exists(&adoc_filename) {
        send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", adoc_filename), cors_origin);
        return;
    }

    match ctx.repository.read_note_content(&adoc_filename) {
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
    match crate::html::export_all_pages_to_html5(&ctx.notes_dir, &ctx.notes_dir, &export_dir) {
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

pub fn handle_pdf_export_response<W: Write>(
    stream: &mut W,
    ctx: &ServerContext,
    note_name: &str,
    cors_origin: &str,
) {
    let adoc_filename = sanitize_note_filename(note_name);
    let resolved_path = if ctx.repository.note_exists(&adoc_filename) {
        adoc_filename
    } else if let Ok(Some(p)) = ctx.repository.get_page(note_name) {
        p.full_path()
    } else {
        adoc_filename
    };

    if !ctx.repository.note_exists(&resolved_path) {
        send_json_error(stream, 404, "Not Found", &format!("Note '{}' not found", note_name), cors_origin);
        return;
    }

    if !ctx.has_pdf_exporter() {
        send_json_error(stream, 501, "Not Implemented", "PDF exporter is not available", cors_origin);
        return;
    }

    let temp_file = match tempfile::Builder::new().prefix("export_").suffix(".pdf").tempfile() {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create temporary file for PDF export: {}", e);
            send_json_error(stream, 500, "Internal Server Error", "Internal server error", cors_origin);
            return;
        }
    };
    let temp_path = temp_file.path().to_path_buf();

    match ctx.export_pdf(&resolved_path, &temp_path) {
        Ok(()) => {
            match std::fs::read(&temp_path) {
                Ok(bytes) => {
                    let clean_name = resolved_path.rsplit('/').next().unwrap_or(&resolved_path);
                    let title = clean_name.strip_suffix(".adoc").unwrap_or(clean_name);
                    send_attachment_response(
                        stream,
                        200,
                        "OK",
                        crate::constants::MIME_PDF,
                        &bytes,
                        &format!("{}.pdf", title),
                        cors_origin,
                    );
                }
                Err(e) => {
                    log::error!("Failed to read generated PDF: {}", e);
                    send_json_error(stream, 500, "Internal Server Error", "Failed to read generated PDF", cors_origin);
                }
            }
        }
        Err(e) => {
            log::error!("PDF exporter failed: {}", e);
            send_json_error(stream, 500, "Internal Server Error", &format!("PDF export failed: {}", e), cors_origin);
        }
    }
}

pub fn handle_export_pdf_api<W: Write>(
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

    handle_pdf_export_response(stream, ctx, &filename, cors_origin);
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

    if let Some(q) = req.query.as_deref() {
        if q.contains("export=pdf") || q.contains("download=pdf") {
            handle_pdf_export_response(stream, ctx, note_name, cors_origin);
            return;
        } else if q.contains("export=1") || q.contains("download=1") {
            if let Ok(content) = ctx.repository.read_note_content(note_name) {
                let title = note_name.rsplit('/').next().unwrap_or(note_name).strip_suffix(".adoc").unwrap_or(note_name);
                let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
                send_attachment_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), &format!("{}.html", title), cors_origin);
                return;
            }
        } else if q.contains("view=rendered") {
            if let Ok(content) = ctx.repository.read_note_content(note_name) {
                let title = note_name.rsplit('/').next().unwrap_or(note_name).strip_suffix(".adoc").unwrap_or(note_name);
                let html = render_web_page_html(&content, title, &ctx.notes_dir, note_name);
                send_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), cors_origin);
                return;
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

    if let Ok(content) = ctx.repository.read_note_content(note_name) {
        send_response(stream, 200, "OK", MIME_TEXT_PLAIN, content.as_bytes(), cors_origin);
        return;
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
    if note_name.ends_with(".pdf") {
        let stripped = note_name.strip_suffix(".pdf").unwrap_or(note_name);
        handle_pdf_export_response(stream, ctx, stripped, cors_origin);
        return;
    }

    let stripped = note_name.strip_suffix(".html").unwrap_or(note_name);

    if let Ok(content) = ctx.repository.read_note_content(stripped) {
        let title = stripped.rsplit('/').next().unwrap_or(stripped).strip_suffix(".adoc").unwrap_or(stripped);
        let html = adoc_to_html5(&content, title, Some(&ctx.notes_dir));
        send_attachment_response(stream, 200, "OK", MIME_HTML, html.as_bytes(), &format!("{}.html", title), cors_origin);
        return;
    }
    send_json_error(stream, 404, "Not Found", "File not found", cors_origin);
}

pub fn handle_events_sse<W: Write>(
    mut stream: W,
    _req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    use crate::server::http::{send_sse_done, send_sse_event, send_sse_header};
    send_sse_header(&mut stream, cors_origin);

    let is_busy = ctx.session.lock().map(|s| s.is_busy()).unwrap_or(false);
    let connect_event = json!({
        "type": "connected",
        "app": "Notes Plus",
        "agent_busy": is_busy
    });
    send_sse_event(&mut stream, &connect_event);
    send_sse_done(&mut stream);
}
