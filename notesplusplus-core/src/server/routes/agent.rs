use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use serde_json::json;

use crate::agent::{
    build_template_instruction, AgentStepResult, LlmClient, LlmProvider, PermissionManager,
};
use crate::constants::MIME_JSON;
use crate::server::http::{
    send_response, send_sse_done, send_sse_event, send_sse_header, ParsedHttpRequest,
};
use crate::server::ServerContext;

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
            let resp = json!({
                "provider": match cfg.provider {
                    LlmProvider::Ollama => "ollama",
                    LlmProvider::OpenAiCompatible => "openai",
                },
                "endpoint": cfg.endpoint_url,
                "model": cfg.model,
                "timeout": cfg.timeout_secs,
                "has_key": cfg.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
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
                if let Some(ep) = json_body.get("endpoint").and_then(|v| v.as_str()) {
                    cfg_guard.endpoint_url = ep.to_string();
                }
                if let Some(m) = json_body.get("model").and_then(|v| v.as_str()) {
                    cfg_guard.model = m.to_string();
                }
                if let Some(k) = json_body.get("api_key").and_then(|v| v.as_str()) {
                    cfg_guard.api_key = if k.is_empty() { None } else { Some(k.to_string()) };
                }
                if let Some(t) = json_body.get("timeout").and_then(|v| v.as_u64()) {
                    cfg_guard.timeout_secs = t;
                }
                if let Some(a) = json_body.get("allow_self_signed").and_then(|v| v.as_bool()) {
                    cfg_guard.allow_self_signed = a;
                }

                let new_client = LlmClient::new(cfg_guard.clone());
                let perm_mgr = PermissionManager::new(ctx.perm_config.lock().unwrap_or_else(|e| e.into_inner()).clone());
                ctx.session.lock().unwrap_or_else(|e| e.into_inner()).update_config(perm_mgr, new_client);

                let resp = json!({ "ok": true });
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
            let resp = json!({ "error": format!("{}", e) });
            send_response(stream, 502, "Bad Gateway", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}

pub fn handle_agent_chat<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let body_str = String::from_utf8_lossy(&req.body);
    let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
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
        match step_result {
            AgentStepResult::Finished { content, last_snapshot_id } => {
                send_sse_event(&mut *s, &json!({
                    "type": "finished",
                    "content": content,
                    "last_snapshot_id": last_snapshot_id,
                    "can_undo": session_guard.can_undo(),
                    "last_created_note": session_guard.last_created_note()
                }));
            }
            AgentStepResult::RequiresConfirmation(pending) => {
                send_sse_event(&mut *s, &json!({
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
                send_sse_event(&mut *s, &json!({
                    "type": "error",
                    "error": err
                }));
            }
        }
        send_sse_done(&mut *s);
    };
}

pub fn handle_agent_template<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let body_str = String::from_utf8_lossy(&req.body);
    let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
    let template_id = json_body.get("template_id").and_then(|v| v.as_str()).unwrap_or("summarize");
    let content = json_body.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let context_filename = json_body.get("context_filename").and_then(|v| v.as_str()).unwrap_or("note.adoc");

    let instruction = build_template_instruction(template_id, "", Some(context_filename), Some(content));

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
        match step_result {
            AgentStepResult::Finished { content, last_snapshot_id } => {
                send_sse_event(&mut *s, &json!({
                    "type": "finished",
                    "content": content,
                    "last_snapshot_id": last_snapshot_id,
                    "can_undo": session_guard.can_undo()
                }));
            }
            AgentStepResult::RequiresConfirmation(pending) => {
                send_sse_event(&mut *s, &json!({
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
                send_sse_event(&mut *s, &json!({
                    "type": "error",
                    "error": err
                }));
            }
        }
        send_sse_done(&mut *s);
    };
}

pub fn handle_agent_confirm<W: Write + Send + 'static>(
    mut stream: W,
    req: &ParsedHttpRequest,
    ctx: &ServerContext,
    cors_origin: &str,
) {
    let body_str = String::from_utf8_lossy(&req.body);
    let json_body: serde_json::Value = serde_json::from_str(&body_str).unwrap_or(json!({}));
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
        match step_result {
            AgentStepResult::Finished { content, last_snapshot_id } => {
                send_sse_event(&mut *s, &json!({
                    "type": "finished",
                    "content": content,
                    "last_snapshot_id": last_snapshot_id,
                    "can_undo": session_guard.can_undo()
                }));
            }
            AgentStepResult::RequiresConfirmation(pending) => {
                send_sse_event(&mut *s, &json!({
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
                send_sse_event(&mut *s, &json!({
                    "type": "error",
                    "error": err
                }));
            }
        }
        send_sse_done(&mut *s);
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
            send_response(stream, 200, "OK", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
        Err(err) => {
            let resp = json!({ "ok": false, "error": err, "can_undo": session_guard.can_undo() });
            send_response(stream, 400, "Bad Request", MIME_JSON, resp.to_string().as_bytes(), cors_origin);
        }
    }
}
