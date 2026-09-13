use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde_json::json;

use crate::agent::{
    build_template_instruction_ex, fetch_url, AgentStepResult, LlmClient, LlmProvider, PermissionManager,
};
use crate::constants::MIME_JSON;
use crate::server::http::{
    send_json_error, send_json_ok, send_response, send_sse_done, send_sse_event, send_sse_header,
    ParsedHttpRequest,
};
use crate::server::ServerContext;

fn send_step_result_sse<W: Write>(
    stream: &mut W,
    step_result: &AgentStepResult,
    session: &crate::agent::AgentSession,
    include_created_note: bool,
) {
    match step_result {
        AgentStepResult::Finished { content, last_snapshot_id } => {
            let mut event = json!({
                "type": "finished",
                "content": content,
                "last_snapshot_id": last_snapshot_id,
                "can_undo": session.can_undo()
            });
            if include_created_note {
                event["last_created_note"] = json!(session.last_created_note());
            }
            send_sse_event(stream, &event);
        }
        AgentStepResult::RequiresConfirmation(pending) => {
            send_sse_event(stream, &json!({
                "type": "pending_confirmation",
                "action": {
                    "tool_name": pending.tool_name,
                    "filename": pending.filename,
                    "reason": pending.reason,
                    "diff": pending.diff.lines.iter().map(|l| json!({
                        "diff_type": format!("{:?}", l.line_type).to_lowercase(),
                        "text": l.content
                    })).collect::<Vec<_>>()
                }
            }));
        }
        AgentStepResult::Error(err) => {
            send_sse_event(stream, &json!({
                "type": "error",
                "error": err
            }));
        }
    }
    send_sse_done(stream);
}

pub fn handle_agent_status<W: Write>(
    stream: &mut W,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
    let is_busy = session_guard.is_busy();
    let can_undo = session_guard.can_undo();
    let has_pending = session_guard.pending_action().is_some();
    let last_snap = session_guard.last_snapshot_id();
    let last_created = session_guard.last_created_note();

    let resp = json!({
        "busy": is_busy,
        "can_undo": can_undo,
        "has_pending": has_pending,
        "last_snapshot_id": last_snap,
        "last_created_note": last_created,
    });
    send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
}

pub fn handle_agent_config<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    match req.method.as_str() {
        "GET" => {
            let cfg = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
            let is_undo_available = ctx.session.lock().unwrap_or_else(|e| e.into_inner()).can_undo();
            let has_pending = ctx.session.lock().unwrap_or_else(|e| e.into_inner()).pending_action().is_some();
            // Server address and tokens are configured on phone app and kept safe from leaking to the web UI
            let resp = json!({
                "provider": match cfg.provider {
                    LlmProvider::Ollama => "ollama",
                    LlmProvider::OpenAiCompatible => "openai",
                },
                "model": cfg.model,
                "system_prompt": cfg.system_prompt.clone().unwrap_or_default(),
                "can_undo": is_undo_available,
                "has_pending": has_pending
            });
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        "POST" => {
            let body_str = String::from_utf8_lossy(&req.body);
            if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
                let mut cfg_guard = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(p) = json_body.get("provider").and_then(|v| v.as_str()) {
                    cfg_guard.provider = match p.to_lowercase().as_str() {
                        "openai" | "mimocode" | "compatible" => LlmProvider::OpenAiCompatible,
                        _ => LlmProvider::Ollama,
                    };
                }
                if let Some(m) = json_body.get("model").and_then(|v| v.as_str()) {
                    cfg_guard.model = m.to_string();
                }
                if let Some(sp) = json_body.get("system_prompt").and_then(|v| v.as_str()) {
                    cfg_guard.system_prompt = if sp.trim().is_empty() {
                        None
                    } else {
                        Some(sp.to_string())
                    };
                }

                let new_client = LlmClient::new(cfg_guard.clone());
                let perm_mgr = PermissionManager::new(ctx.perm_config.lock().unwrap_or_else(|e| e.into_inner()).clone());
                ctx.session.lock().unwrap_or_else(|e| e.into_inner()).update_config(perm_mgr, new_client);

                let resp = json!({
                    "ok": true,
                    "provider": match cfg_guard.provider {
                        LlmProvider::Ollama => "ollama",
                        LlmProvider::OpenAiCompatible => "openai",
                    },
                    "model": cfg_guard.model,
                    "system_prompt": cfg_guard.system_prompt.clone().unwrap_or_default()
                });
                send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            }
        }
        _ => {}
    }
}

pub fn handle_agent_models<W: Write>(
    stream: &mut W,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let cfg = ctx.llm_config.lock().unwrap_or_else(|e| e.into_inner());
    let client = LlmClient::new(cfg.clone());
    drop(cfg);

    match client.list_models() {
        Ok(models) => {
            let resp = json!({ "models": models });
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        Err(e) => {
            log::error!("LLM model listing failed: {}", e);
            send_json_error(stream, 502, "Bad Gateway", "Failed to retrieve models from AI provider", cors_origin);
        }
    }
}

pub fn handle_agent_chat<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let prompt = json_body.get("prompt").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let context_filename = json_body.get("context_filename").and_then(|v| v.as_str()).map(|s| s.to_string());
    let context_content = json_body.get("context_content").and_then(|v| v.as_str()).map(|s| s.to_string());

    send_sse_header(&mut stream, cors_origin);

    let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref fname) = context_filename {
        let content = context_content.unwrap_or_else(|| {
            fs::read_to_string(ctx.notes_dir.join(fname)).unwrap_or_default()
        });
        session_guard.reset_session(Some((fname.as_str(), &content)), None);
    }

    let stream_mutex = Arc::new(Mutex::new(stream));
    let stream_for_tokens = stream_mutex.clone();

    let step_result = session_guard.send_prompt_streaming(&prompt, move |token| {
        if let Ok(mut s) = stream_for_tokens.lock() {
            send_sse_event(&mut *s, &json!({
                "type": "token",
                "text": token
            }));
        }
    });

    if let Ok(mut s) = stream_mutex.lock() {
        send_step_result_sse(&mut *s, &step_result, &session_guard, true);
    };
}

pub fn handle_agent_template<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let template_id = json_body.get("template_id").and_then(|v| v.as_str()).unwrap_or("summarize");
    let content = json_body.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let context_filename = json_body.get("context_filename").and_then(|v| v.as_str()).unwrap_or("note.adoc");

    // Import-specific optional parameters
    let import_title = json_body.get("target_title").and_then(|v| v.as_str());
    let import_mode = json_body.get("mode").and_then(|v| v.as_str());
    let import_custom = json_body.get("custom_instruction").and_then(|v| v.as_str());

    let instruction = build_template_instruction_ex(template_id, "", Some(context_filename), Some(content), import_title, import_mode, import_custom);

    send_sse_header(&mut stream, cors_origin);
    let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
    session_guard.reset_session(Some((context_filename, content)), None);

    let stream_mutex = Arc::new(Mutex::new(stream));
    let stream_for_tokens = stream_mutex.clone();

    let step_result = session_guard.send_prompt_streaming(&instruction, move |token| {
        if let Ok(mut s) = stream_for_tokens.lock() {
            send_sse_event(&mut *s, &json!({
                "type": "token",
                "text": token
            }));
        }
    });

    if let Ok(mut s) = stream_mutex.lock() {
        send_step_result_sse(&mut *s, &step_result, &session_guard, false);
    };
}

pub fn handle_agent_confirm<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let approved = json_body.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);

    send_sse_header(&mut stream, cors_origin);
    let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());

    let stream_mutex = Arc::new(Mutex::new(stream));
    let stream_for_tokens = stream_mutex.clone();

    let step_result = session_guard.confirm_pending_action_streaming(approved, move |token| {
        if let Ok(mut s) = stream_for_tokens.lock() {
            send_sse_event(&mut *s, &json!({
                "type": "token",
                "text": token
            }));
        }
    });

    if let Ok(mut s) = stream_mutex.lock() {
        send_step_result_sse(&mut *s, &step_result, &session_guard, false);
    };
}

pub fn handle_agent_undo<W: Write>(
    stream: &mut W,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let mut session_guard = ctx.session.lock().unwrap_or_else(|e| e.into_inner());
    match session_guard.undo_last_action() {
        Ok(msg) => {
            let resp = json!({ "ok": true, "message": msg, "can_undo": session_guard.can_undo() });
            send_json_ok(stream, &resp, cors_origin);
        }
        Err(err) => {
            let resp = json!({ "ok": false, "error": err, "can_undo": session_guard.can_undo() });
            send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}

pub fn handle_fetch_url<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let url = json_body.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
    if url.is_empty() {
        let resp = json!({ "ok": false, "error": "URL parameter is required" });
        send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        return;
    }

    match fetch_url(url) {
        Ok(content) => {
            let resp = json!({ "ok": true, "content": content });
            send_json_ok(stream, &resp, cors_origin);
        }
        Err(err) => {
            let resp = json!({ "ok": false, "error": err });
            send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}

pub fn handle_preprocess_html<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let html = json_body.get("html").and_then(|v| v.as_str()).unwrap_or("");
    let processed = crate::html::preprocess_html(html);
    let resp = json!({ "ok": true, "content": processed });
    send_json_ok(stream, &resp, cors_origin);
}

pub fn handle_read_file<W: Write>(
    stream: &mut W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let json_body = req.json_body();
    let file_path = json_body.get("file_path").and_then(|v| v.as_str()).unwrap_or("").trim();
    if file_path.is_empty() {
        let resp = json!({ "ok": false, "error": "file_path parameter is required" });
        send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        return;
    }
    if file_path.contains("..") {
        let resp = json!({ "ok": false, "error": "Path traversal ('..') is not allowed" });
        send_response(stream, 403, "Forbidden", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        return;
    }
    let expanded = if file_path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(&file_path[2..])
        } else {
            PathBuf::from(file_path)
        }
    } else {
        PathBuf::from(file_path)
    };
    let canonical_notes_dir = match ctx.notes_dir.canonicalize() {
        Ok(c) => c,
        Err(_) => ctx.notes_dir.clone(),
    };
    let canonical = match expanded.canonicalize() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Error resolving file path: {}", e);
            let resp = json!({ "ok": false, "error": "Error resolving file path" });
            send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
            return;
        }
    };
    if !canonical.starts_with(&canonical_notes_dir) {
        let resp = json!({ "ok": false, "error": "Access denied: file is outside the notes directory" });
        send_response(stream, 403, "Forbidden", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        return;
    }
    match fs::read_to_string(&canonical) {
        Ok(content) => {
            let is_html = canonical.extension().and_then(|e| e.to_str()).map(|ext| ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")).unwrap_or(false) || crate::html::preprocess::looks_like_html(&content);
            let processed = if is_html {
                crate::html::preprocess_html(&content)
            } else {
                content
            };
            let resp = json!({ "ok": true, "content": processed });
            send_json_ok(stream, &resp, cors_origin);
        }
        Err(e) => {
            log::error!("Error reading file: {}", e);
            let resp = json!({ "ok": false, "error": "Error reading file" });
            send_response(stream, 500, "Internal Server Error", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}
