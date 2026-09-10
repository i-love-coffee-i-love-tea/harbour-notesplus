//! Agent chat session and multi-turn tool execution loop.

use std::fs;
use std::path::{Path, PathBuf};
use rusqlite::Connection;
use serde_json::json;
use crate::agent::backup::BackupManager;
use crate::agent::client::{ChatMessage, LlmClient};
use crate::agent::permissions::{PendingConfirmation, PermissionDecision, PermissionManager};
use crate::agent::prompt::build_system_prompt_with_custom;
use crate::agent::tools::{is_blocked_host, ToolCall};
use crate::db;
use crate::page;
use crate::search;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStepResult {
    Finished {
        content: String,
        last_snapshot_id: Option<String>,
    },
    RequiresConfirmation(PendingConfirmation),
    Error(String),
}

pub struct AgentSession {
    notes_dir: PathBuf,
    db_path: PathBuf,
    backup_mgr: BackupManager,
    permission_mgr: PermissionManager,
    client: LlmClient,
    messages: Vec<ChatMessage>,
    pending_action: Option<PendingConfirmation>,
    last_snapshot_id: Option<String>,
    last_created_note: Option<String>,
    max_tool_turns: usize,
}

impl AgentSession {
    pub fn new(
        notes_dir: impl AsRef<Path>,
        db_path: impl AsRef<Path>,
        backup_dir: impl AsRef<Path>,
        permission_mgr: PermissionManager,
        client: LlmClient,
    ) -> Self {
        Self {
            notes_dir: notes_dir.as_ref().to_path_buf(),
            db_path: db_path.as_ref().to_path_buf(),
            backup_mgr: BackupManager::new(backup_dir),
            permission_mgr,
            client,
            messages: Vec::new(),
            pending_action: None,
            last_snapshot_id: None,
            last_created_note: None,
            max_tool_turns: 8,
        }
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn pending_action(&self) -> Option<&PendingConfirmation> {
        self.pending_action.as_ref()
    }

    pub fn last_snapshot_id(&self) -> Option<&str> {
        self.last_snapshot_id.as_deref()
    }

    pub fn last_created_note(&self) -> Option<&str> {
        self.last_created_note.as_deref()
    }

    pub fn can_undo(&self) -> bool {
        self.last_snapshot_id.is_some()
    }

    /// Dynamically updates LLM client and permission manager without resetting active chat state.
    pub fn update_config(&mut self, permission_mgr: PermissionManager, client: LlmClient) {
        self.permission_mgr = permission_mgr;
        self.client = client;
    }

    /// Resets conversation with fresh system prompt including optional active note content.
    pub fn reset_session(
        &mut self,
        active_note: Option<(&str, &str)>,
        extra_context: Option<&str>,
    ) {
        self.messages.clear();
        self.pending_action = None;
        self.last_created_note = None;
        let sys_prompt = build_system_prompt_with_custom(
            self.client.config().system_prompt.as_deref(),
            active_note,
            extra_context,
        );
        self.messages.push(ChatMessage::system(sys_prompt));
    }

    /// Appends a user prompt to the conversation history.
    pub fn push_user_message(&mut self, user_prompt: &str) {
        if self.messages.is_empty() {
            self.reset_session(None, None);
        }
        self.messages.push(ChatMessage::user(user_prompt));
    }

    /// Appends a user prompt and drives the conversation loop with a streaming token callback.
    pub fn send_prompt_streaming<F: FnMut(&str)>(&mut self, user_prompt: &str, on_token: F) -> AgentStepResult {
        if self.messages.is_empty() {
            self.reset_session(None, None);
        }

        let already_pushed = self.messages.last().map(|m| m.role.as_str() == "user" && m.content.as_deref() == Some(user_prompt)).unwrap_or(false);
        if !already_pushed {
            self.messages.push(ChatMessage::user(user_prompt));
        }
        self.run_loop_streaming(on_token)
    }

    /// Resolves pending confirmation (approving or rejecting) and resumes the loop with a streaming token callback.
    pub fn confirm_pending_action_streaming<F: FnMut(&str)>(&mut self, approved: bool, on_token: F) -> AgentStepResult {
        let pending = match self.pending_action.take() {
            Some(p) => p,
            None => return AgentStepResult::Error("No pending action to confirm".to_string()),
        };

        if approved {
            let result_str = self.apply_note_edit(&pending.filename, &pending.new_content, &pending.reason);
            self.messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                result_str,
            ));
        } else {
            self.messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                format!("User rejected the proposed changes to '{}'.", pending.filename),
            ));
        }

        self.run_loop_streaming(on_token)
    }

    /// Runs the LLM tool execution loop with token streaming until an answer or confirmation gate is reached.
    fn run_loop_streaming<F: FnMut(&str)>(&mut self, mut on_token: F) -> AgentStepResult {
        let mut turns = 0;

        while turns < self.max_tool_turns {
            turns += 1;

            let response = match self.client.send_chat_streaming(&self.messages, &mut on_token) {
                Ok(resp) => resp,
                Err(err) => return AgentStepResult::Error(format!("LLM Request Failed: {}", err)),
            };

            // If assistant responded with tool calls
            if !response.tool_calls.is_empty() {
                // Record assistant tool call message with optional textual thoughts
                let mut asst_msg = ChatMessage::assistant_tool_calls(response.tool_calls.clone());
                asst_msg.content = response.content.clone();
                self.messages.push(asst_msg);

                // Execute tool calls in order, pausing for confirmation if needed
                for tool_call in &response.tool_calls {
                    let file_content = if tool_call.function.name == "edit_note" {
                        let filename = tool_call.function.arguments.get("filename")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let sanitized = page::sanitize_filename(filename);
                        let file_path = self.notes_dir.join(&sanitized);
                        fs::read_to_string(&file_path).ok()
                    } else {
                        None
                    };

                    match self.permission_mgr.evaluate(tool_call, file_content.as_deref()) {
                        PermissionDecision::Allowed => {
                            let output = self.execute_tool(tool_call);
                            self.messages.push(ChatMessage::tool_result(
                                tool_call.id.clone(),
                                tool_call.function.name.clone(),
                                output,
                            ));
                        }
                        PermissionDecision::RequiresConfirmation(pending) => {
                            // Record any remaining tool calls as skipped so the LLM knows
                            for remaining in response.tool_calls.iter().skip_while(|tc| tc.id != tool_call.id).skip(1) {
                                self.messages.push(ChatMessage::tool_result(
                                    remaining.id.clone(),
                                    remaining.function.name.clone(),
                                    "Skipped: waiting for user confirmation of prior edit".to_string(),
                                ));
                            }
                            self.pending_action = Some(pending.clone());
                            return AgentStepResult::RequiresConfirmation(pending);
                        }
                        PermissionDecision::Denied(reason) => {
                            self.messages.push(ChatMessage::tool_result(
                                tool_call.id.clone(),
                                tool_call.function.name.clone(),
                                format!("Tool execution denied: {}", reason),
                            ));
                        }
                    }
                }
            } else {
                // Final textual answer
                let final_content = response.content.unwrap_or_default();
                self.messages.push(ChatMessage::assistant(final_content.clone()));
                return AgentStepResult::Finished {
                    content: final_content,
                    last_snapshot_id: self.last_snapshot_id.clone(),
                };
            }
        }

        AgentStepResult::Error("Max tool turns exceeded without reaching a final response".to_string())
    }

    /// Executes an auto-allowed tool.
    fn execute_tool(&mut self, tool_call: &ToolCall) -> String {
        let name = tool_call.function.name.as_str();
        let args = &tool_call.function.arguments;

        match name {
            "read_note" => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let sanitized = page::sanitize_filename(filename);
                let path = self.notes_dir.join(&sanitized);
                match fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(e) => format!("Error reading note '{}': {}", sanitized, e),
                }
            }
            "list_notes" => {
                match self.open_db() {
                    Ok(conn) => match page::list_pages(&conn) {
                        Ok(pages) => {
                            let mut list = Vec::new();
                            for p in pages {
                                list.push(json!({
                                    "filename": p.filename,
                                    "title": p.title,
                                    "updated_at": p.updated_at
                                }));
                            }
                            serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".to_string())
                        }
                        Err(e) => format!("Error listing pages: {}", e),
                    },
                    Err(e) => format!("Database error: {}", e),
                }
            }
            "search_notes" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                match self.open_db() {
                    Ok(conn) => match search::search_pages(&conn, query) {
                        Ok(results) => {
                            let mut out = Vec::new();
                            for r in results {
                                out.push(json!({
                                    "filename": r.page.filename,
                                    "title": r.page.title,
                                    "snippet": r.snippet
                                }));
                            }
                            serde_json::to_string_pretty(&out).unwrap_or_else(|_| "[]".to_string())
                        }
                        Err(e) => format!("Search error: {}", e),
                    },
                    Err(e) => format!("Database error: {}", e),
                }
            }
            "create_note" => {
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                match self.open_db() {
                    Ok(conn) => match page::create_page(&conn, &self.notes_dir, &page::sanitize_filename(title), false) {
                        Ok(created) => {
                            if !content.trim().is_empty() {
                                let path = self.notes_dir.join(&created.filename);
                                if let Err(e) = fs::write(&path, content) {
                                    return format!("Error writing note content: {}", e);
                                }
                                let _ = db::update_fts_content(&conn, created.id, content);
                            }
                            self.last_created_note = Some(created.title.clone());
                            format!("Successfully created note '{}' ({})", created.title, created.filename)
                        }
                        Err(e) => format!("Error creating note: {}", e),
                    },
                    Err(e) => format!("Database error: {}", e),
                }
            }
            "edit_note" => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Updated note");
                self.apply_note_edit(&page::sanitize_filename(filename), content, reason)
            }
            "fetch_url" => {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if is_blocked_host(url) {
                    format!("URL '{}' blocked: fetching localhost/private addresses is not allowed", url)
                } else {
                    match ureq::get(url).timeout(std::time::Duration::from_secs(15)).call() {
                        Ok(resp) => {
                            let text = resp.into_string().unwrap_or_default();
                            if text.len() > 6000 {
                                let mut end = 6000;
                                while end > 0 && !text.is_char_boundary(end) {
                                    end -= 1;
                                }
                                format!("{}... [truncated]", &text[..end])
                            } else {
                                text
                            }
                        }
                        Err(e) => format!("Failed to fetch URL '{}': {}", url, e),
                    }
                }
            }
            _ => format!("Unknown tool '{}'", name),
        }
    }

    /// Applies note update with pre-edit backup snapshot and SQLite FTS index update.
    pub fn apply_note_edit(&mut self, filename: &str, new_content: &str, reason: &str) -> String {
        let file_path = self.notes_dir.join(filename);
        let current_content = fs::read_to_string(&file_path).unwrap_or_default();

        // 1. Create pre-edit snapshot
        let snapshot = match self.backup_mgr.create_snapshot(filename, &current_content, reason) {
            Ok(s) => {
                self.last_snapshot_id = Some(s.id.clone());
                Some(s)
            }
            Err(e) => {
                log::warn!("Failed to create backup snapshot: {}", e);
                None
            }
        };

        // 2. Write new content to file
        if let Err(e) = fs::write(&file_path, new_content) {
            return format!("Failed to write to file '{}': {}", filename, e);
        }

        // 3. Update SQLite FTS index
        if let Ok(conn) = self.open_db() {
            let title = extract_doc_title(new_content, filename);
            if let Ok(Some(existing_page)) = page::get_page(&conn, filename) {
                let _ = db::update_fts_content(&conn, existing_page.id, new_content);
            } else {
                let now = chrono::Utc::now().to_rfc3339();
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO pages (filename, title, is_journal, created_at, updated_at, block_count)
                     VALUES (?1, ?2, 0, ?3, ?3, 0)",
                    rusqlite::params![filename, title, now],
                );
                if let Ok(page_id) = conn.query_row(
                    "SELECT id FROM pages WHERE filename = ?1",
                    rusqlite::params![filename],
                    |row| row.get::<_, i64>(0),
                ) {
                    let _ = db::update_fts_content(&conn, page_id, new_content);
                }
            }
        }

        let snap_msg = snapshot
            .map(|s| format!(" (Snapshot archived: {})", s.id))
            .unwrap_or_default();

        format!("Successfully updated note '{}' with reason: {}{}", filename, reason, snap_msg)
    }

    /// Undoes the last recorded edit action by rolling back to its pre-edit snapshot.
    pub fn undo_last_action(&mut self) -> Result<String, String> {
        let snapshot_id = self.last_snapshot_id.clone()
            .ok_or_else(|| "No previous action available to undo".to_string())?;
        self.rollback_snapshot(&snapshot_id)
    }

    /// Rolls back a note to an arbitrary snapshot by ID.
    pub fn rollback_snapshot(&mut self, snapshot_id: &str) -> Result<String, String> {
        let snapshot = self.backup_mgr.get_snapshot(snapshot_id)
            .map_err(|e| format!("Failed to read snapshot: {}", e))?
            .ok_or_else(|| format!("Snapshot '{}' not found", snapshot_id))?;

        let file_path = self.notes_dir.join(&snapshot.filename);
        fs::write(&file_path, &snapshot.content)
            .map_err(|e| format!("Failed to restore file: {}", e))?;

        // Update database index
        if let Ok(conn) = self.open_db() {
            if let Ok(Some(existing_page)) = page::get_page(&conn, &snapshot.filename) {
                let _ = db::update_fts_content(&conn, existing_page.id, &snapshot.content);
            }
        }

        let _ = self.backup_mgr.delete_snapshot(snapshot_id);
        if self.last_snapshot_id.as_deref() == Some(snapshot_id) {
            self.last_snapshot_id = None;
        }

        Ok(format!("Successfully rolled back '{}' to pre-edit state ({})", snapshot.filename, snapshot_id))
    }

    fn open_db(&self) -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open(&self.db_path)?;
        db::init_schema(&conn)?;
        Ok(conn)
    }
}

fn extract_doc_title(content: &str, fallback_filename: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("= ") {
            return trimmed.trim_start_matches("= ").trim().to_string();
        }
    }
    fallback_filename.trim_end_matches(".adoc").replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::agent::client::LlmConfig;
    use crate::agent::permissions::PermissionConfig;

    #[test]
    fn test_apply_note_edit_and_undo() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let note_file = notes_dir.join("meeting.adoc");
        fs::write(&note_file, "= Meeting\nInitial agenda").unwrap();

        let mut session = AgentSession::new(
            &notes_dir,
            &db_path,
            &backup_dir,
            PermissionManager::new(PermissionConfig::default()),
            LlmClient::new(LlmConfig::default()),
        );

        // Apply edit
        let res = session.apply_note_edit("meeting.adoc", "= Meeting\nUpdated agenda with action items", "Added items");
        assert!(res.contains("Successfully updated"));
        assert!(session.can_undo());

        // Check file content updated
        let modified = fs::read_to_string(&note_file).unwrap();
        assert_eq!(modified, "= Meeting\nUpdated agenda with action items");

        // Rollback
        let undo_res = session.undo_last_action().unwrap();
        assert!(undo_res.contains("Successfully rolled back"));
        assert!(!session.can_undo());

        // Check original content restored
        let reverted = fs::read_to_string(&note_file).unwrap();
        assert_eq!(reverted, "= Meeting\nInitial agenda");
    }

    #[test]
    fn test_execute_create_and_search_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let db_path = tmp.path().join("test.db");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let mut session = AgentSession::new(
            &notes_dir,
            &db_path,
            &backup_dir,
            PermissionManager::new(PermissionConfig::default()),
            LlmClient::new(LlmConfig::default()),
        );

        let create_call = ToolCall {
            id: Some("call_1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({
                    "title": "Shopping List",
                    "content": "= Shopping List\n* [ ] Apples\n* [ ] Milk"
                }),
            },
        };

        let create_out = session.execute_tool(&create_call);
        assert!(create_out.contains("Successfully created note"));

        let search_call = ToolCall {
            id: Some("call_2".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "search_notes".to_string(),
                arguments: json!({ "query": "Apples" }),
            },
        };

        let search_out = session.execute_tool(&search_call);
        assert!(search_out.contains("Shopping List") || search_out.contains("shopping_list.adoc"));
    }
}
