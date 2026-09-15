//! Agent chat session and multi-turn tool execution loop.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use serde_json::json;
use crate::agent::backup::BackupManager;
use crate::agent::client::{ChatMessage, LlmClient};
use crate::agent::permissions::{PendingConfirmation, PermissionDecision, PermissionManager};
use crate::agent::prompt::build_system_prompt_with_custom;
use crate::agent::tools::{ToolCall, TOOL_READ_NOTE, TOOL_LIST_NOTES, TOOL_SEARCH_NOTES, TOOL_CREATE_NOTE, TOOL_EDIT_NOTE, TOOL_FETCH_URL};
use crate::page;
use crate::repository::NoteRepository;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStepResult {
    Finished {
        content: String,
        last_snapshot_id: Option<String>,
    },
    RequiresConfirmation(PendingConfirmation),
    Error(String),
}

#[derive(Default, Clone, Debug)]
pub struct SessionState {
    pub messages: Vec<ChatMessage>,
    pub pending_action: Option<PendingConfirmation>,
    pub last_snapshot_id: Option<String>,
    pub last_created_note: Option<String>,
}

pub struct AgentSession {
    repository: Arc<dyn NoteRepository>,
    backup_mgr: Arc<Mutex<BackupManager>>,
    permission_mgr: Arc<Mutex<PermissionManager>>,
    client: Arc<Mutex<LlmClient>>,
    state: Arc<Mutex<SessionState>>,
    is_busy: Arc<AtomicBool>,
    max_tool_turns: usize,
}

impl AgentSession {
    pub fn new(
        _notes_dir: impl AsRef<Path>,
        repository: Arc<dyn NoteRepository>,
        backup_dir: impl AsRef<Path>,
        permission_mgr: PermissionManager,
        client: LlmClient,
    ) -> Self {
        Self {
            repository,
            backup_mgr: Arc::new(Mutex::new(BackupManager::new(backup_dir))),
            permission_mgr: Arc::new(Mutex::new(permission_mgr)),
            client: Arc::new(Mutex::new(client)),
            state: Arc::new(Mutex::new(SessionState::default())),
            is_busy: Arc::new(AtomicBool::new(false)),
            max_tool_turns: 8,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::SeqCst)
    }

    /// Returns a snapshot of the current session state in a single lock acquisition.
    pub fn session_state_snapshot(&self) -> SessionState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn messages(&self) -> Vec<ChatMessage> {
        self.session_state_snapshot().messages
    }

    pub fn pending_action(&self) -> Option<PendingConfirmation> {
        self.session_state_snapshot().pending_action
    }

    pub fn last_snapshot_id(&self) -> Option<String> {
        self.session_state_snapshot().last_snapshot_id
    }

    pub fn last_created_note(&self) -> Option<String> {
        self.session_state_snapshot().last_created_note
    }

    pub fn can_undo(&self) -> bool {
        self.session_state_snapshot().last_snapshot_id.is_some()
    }

    /// Dynamically updates LLM client and permission manager without resetting active chat state.
    pub fn update_config(&mut self, permission_mgr: PermissionManager, client: LlmClient) {
        *self.permission_mgr.lock().unwrap_or_else(|e| e.into_inner()) = permission_mgr;
        *self.client.lock().unwrap_or_else(|e| e.into_inner()) = client;
    }

    /// Resets conversation with fresh system prompt including optional active note content.
    pub fn reset_session(
        &mut self,
        active_note: Option<(&str, &str)>,
        extra_context: Option<&str>,
    ) {
        let custom_sys = self.client.lock().unwrap_or_else(|e| e.into_inner()).config().system_prompt.clone();
        let sys_prompt = build_system_prompt_with_custom(
            custom_sys.as_deref(),
            active_note,
            extra_context,
        );

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.messages.clear();
        state.pending_action = None;
        state.last_created_note = None;
        state.messages.push(ChatMessage::system(sys_prompt));
    }

    /// Appends a user prompt to the conversation history.
    pub fn push_user_message(&mut self, user_prompt: &str) {
        let needs_reset = self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.is_empty();
        if needs_reset {
            self.reset_session(None, None);
        }
        self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.push(ChatMessage::user(user_prompt));
    }

    /// Appends a user prompt and drives the conversation loop with a streaming token callback.
    pub fn send_prompt_streaming<F: FnMut(&str)>(&mut self, user_prompt: &str, on_token: F) -> AgentStepResult {
        let needs_reset = self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.is_empty();
        if needs_reset {
            self.reset_session(None, None);
        }

        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let already_pushed = state.messages.last().map(|m| m.role.as_str() == "user" && m.content.as_deref() == Some(user_prompt)).unwrap_or(false);
            if !already_pushed {
                state.messages.push(ChatMessage::user(user_prompt));
            }
        }
        self.run_loop_streaming(on_token)
    }

    /// Resolves pending confirmation (approving or rejecting) and resumes the loop with a streaming token callback.
    pub fn confirm_pending_action_streaming<F: FnMut(&str)>(&mut self, approved: bool, on_token: F) -> AgentStepResult {
        let pending = match self.state.lock().unwrap_or_else(|e| e.into_inner()).pending_action.take() {
            Some(p) => p,
            None => return AgentStepResult::Error("No pending action to confirm".to_string()),
        };

        if approved {
            let result_str = self.apply_note_edit(&pending.filename, &pending.new_content, &pending.reason);
            self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                result_str,
            ));
        } else {
            self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                format!("User rejected the proposed changes to '{}'.", pending.filename),
            ));
        }

        self.run_loop_streaming(on_token)
    }

    /// Runs the LLM tool execution loop with token streaming until an answer or confirmation gate is reached.
    fn run_loop_streaming<F: FnMut(&str)>(&self, mut on_token: F) -> AgentStepResult {
        self.is_busy.store(true, Ordering::SeqCst);
        let result = self.do_run_loop_streaming(&mut on_token);
        self.is_busy.store(false, Ordering::SeqCst);
        result
    }

    fn do_run_loop_streaming<F: FnMut(&str)>(&self, on_token: &mut F) -> AgentStepResult {
        let mut turns = 0;

        while turns < self.max_tool_turns {
            turns += 1;

            let messages = self.state.lock().unwrap_or_else(|e| e.into_inner()).messages.clone();
            let client = self.client.lock().unwrap_or_else(|e| e.into_inner()).clone();

            let response = match client.send_chat_streaming(&messages, &mut *on_token) {
                Ok(resp) => resp,
                Err(err) => return AgentStepResult::Error(format!("LLM Request Failed: {}", err)),
            };

            // If assistant responded with tool calls
            if !response.tool_calls.is_empty() {
                {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    let mut asst_msg = ChatMessage::assistant_tool_calls(response.tool_calls.clone());
                    asst_msg.content = response.content.clone();
                    state.messages.push(asst_msg);
                }

                // Execute tool calls in order, pausing for confirmation if needed
                for tool_call in &response.tool_calls {
                    let file_content = if tool_call.function.name == TOOL_EDIT_NOTE {
                        let filename = tool_call.function.arguments.get("filename")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        self.repository.read_note_content(filename).ok()
                    } else {
                        None
                    };

                    let decision = {
                        let perm = self.permission_mgr.lock().unwrap_or_else(|e| e.into_inner());
                        perm.evaluate(tool_call, file_content.as_deref())
                    };

                    match decision {
                        PermissionDecision::Allowed => {
                            let output = self.execute_tool(tool_call);
                            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                            state.messages.push(ChatMessage::tool_result(
                                tool_call.id.clone(),
                                tool_call.function.name.clone(),
                                output,
                            ));
                        }
                        PermissionDecision::RequiresConfirmation(pending) => {
                            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                            for remaining in response.tool_calls.iter().skip_while(|tc| tc.id != tool_call.id).skip(1) {
                                state.messages.push(ChatMessage::tool_result(
                                    remaining.id.clone(),
                                    remaining.function.name.clone(),
                                    "Skipped: waiting for user confirmation of prior edit".to_string(),
                                ));
                            }
                            state.pending_action = Some(pending.clone());
                            return AgentStepResult::RequiresConfirmation(pending);
                        }
                        PermissionDecision::Denied(reason) => {
                            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                            state.messages.push(ChatMessage::tool_result(
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
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                state.messages.push(ChatMessage::assistant(final_content.clone()));
                return AgentStepResult::Finished {
                    content: final_content,
                    last_snapshot_id: state.last_snapshot_id.clone(),
                };
            }
        }

        AgentStepResult::Error("Max tool turns exceeded without reaching a final response".to_string())
    }

    /// Executes an auto-allowed tool.
    fn execute_tool(&self, tool_call: &ToolCall) -> String {
        let name = tool_call.function.name.as_str();
        let args = &tool_call.function.arguments;

        match name {
            TOOL_READ_NOTE => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                match self.repository.read_note_content(filename) {
                    Ok(content) => content,
                    Err(e) => format!("Error reading note '{}': {}", filename, e),
                }
            }
            TOOL_SEARCH_NOTES => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                match self.repository.search_pages(query) {
                    Ok(results) => {
                        if results.is_empty() {
                            format!("No notes found matching query '{}'", query)
                        } else {
                            let hits: Vec<serde_json::Value> = results.iter().map(|hit| {
                                json!({
                                    "filename": hit.page.filename,
                                    "group_path": hit.page.group_path,
                                    "full_path": hit.page.full_path(),
                                    "title": hit.page.title,
                                    "snippet": hit.snippet,
                                })
                            }).collect();
                            serde_json::to_string_pretty(&hits).unwrap_or_else(|_| "[]".to_string())
                        }
                    }
                    Err(e) => format!("Search error: {}", e),
                }
            }
            TOOL_LIST_NOTES => {
                match self.repository.list_pages() {
                    Ok(pages) => {
                        let page_list: Vec<serde_json::Value> = pages.iter().map(|p| {
                            json!({
                                "filename": p.filename,
                                "group_path": p.group_path,
                                "full_path": p.full_path(),
                                "title": p.title,
                                "is_journal": p.is_journal,
                                "updated_at": p.updated_at,
                            })
                        }).collect();
                        serde_json::to_string_pretty(&page_list).unwrap_or_else(|_| "[]".to_string())
                    }
                    Err(e) => format!("List error: {}", e),
                }
            }
            TOOL_CREATE_NOTE => {
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                let group = args.get("group").and_then(|v| v.as_str()).unwrap_or("");
                let initial_content = args.get("content").or_else(|| args.get("initial_content")).and_then(|v| v.as_str());

                let note_path = if group.is_empty() {
                    title.to_string()
                } else {
                    format!("{}/{}", group.trim_matches('/'), title)
                };

                match self.repository.create_page(&note_path, false) {
                    Ok(created) => {
                        if let Some(content) = initial_content {
                            if let Err(e) = self.repository.save_note(&created.full_path(), content) {
                                return format!("Created note '{}' but failed to write content: {}", created.title, e);
                            }
                        }
                        self.state.lock().unwrap_or_else(|e| e.into_inner()).last_created_note = Some(created.title.clone());
                        format!("Successfully created note '{}' ({})", created.title, created.full_path())
                    }
                    Err(e) => format!("Error creating note: {}", e),
                }
            }
            TOOL_EDIT_NOTE => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Updated note");
                self.apply_note_edit(filename, content, reason)
            }
            TOOL_FETCH_URL => {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                match crate::agent::tools::fetch_url(url) {
                    Ok(text) => text,
                    Err(e) => format!("Error fetching URL: {}", e),
                }
            }
            _ => format!("Unknown tool '{}'", name),
        }
    }

    /// Applies note update with pre-edit backup snapshot and SQLite FTS index update.
    pub fn apply_note_edit(&self, filename: &str, new_content: &str, reason: &str) -> String {
        let (group_path, safe_stem) = page::sanitize_note_path(filename);
        let full_path = if group_path.is_empty() { safe_stem.clone() } else { format!("{}/{}", group_path, safe_stem) };
        let current_content = match self.repository.read_note_content(filename) {
            Ok(c) => c,
            Err(_) => {
                // File may not exist yet (new note) — empty is correct for backup
                String::new()
            }
        };

        // 1. Create pre-edit snapshot
        let snapshot = {
            let backup = self.backup_mgr.lock().unwrap_or_else(|e| e.into_inner());
            match backup.create_snapshot(&full_path, &current_content, reason) {
                Ok(s) => {
                    self.state.lock().unwrap_or_else(|e| e.into_inner()).last_snapshot_id = Some(s.id.clone());
                    Some(s)
                }
                Err(e) => {
                    log::warn!("Failed to create backup snapshot: {}", e);
                    None
                }
            }
        };

        // 2. Save note and update DB atomically via repository
        if let Err(e) = self.repository.save_note(filename, new_content) {
            return format!("Failed to save note '{}': {}", full_path, e);
        }

        let snap_msg = snapshot
            .map(|s| format!(" (Snapshot archived: {})", s.id))
            .unwrap_or_default();

        format!("Successfully updated note '{}' with reason: {}{}", full_path, reason, snap_msg)
    }

    /// Undoes the last recorded edit action by rolling back to its pre-edit snapshot.
    pub fn undo_last_action(&mut self) -> Result<String, String> {
        let snapshot_id = self.state.lock().unwrap_or_else(|e| e.into_inner()).last_snapshot_id.clone()
            .ok_or_else(|| "No previous action available to undo".to_string())?;
        self.rollback_snapshot(&snapshot_id)
    }

    /// Rolls back a note to an arbitrary snapshot by ID.
    pub fn rollback_snapshot(&mut self, snapshot_id: &str) -> Result<String, String> {
        let snapshot = {
            let backup = self.backup_mgr.lock().unwrap_or_else(|e| e.into_inner());
            let snap = backup.get_snapshot(snapshot_id)
                .map_err(|e| format!("Failed to read snapshot: {}", e))?
                .ok_or_else(|| format!("Snapshot '{}' not found", snapshot_id))?;
            let _ = backup.delete_snapshot(snapshot_id);
            snap
        };

        self.repository.save_note(&snapshot.filename, &snapshot.content)
            .map_err(|e| format!("Failed to restore note: {}", e))?;

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.last_snapshot_id.as_deref() == Some(snapshot_id) {
            state.last_snapshot_id = None;
        }

        Ok(format!("Successfully rolled back '{}' to pre-edit state ({})", snapshot.filename, snapshot_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::agent::client::LlmConfig;
    use crate::agent::permissions::PermissionConfig;
    use crate::repository::FsSqliteNoteRepository;

    fn make_session(notes_dir: &Path, backup_dir: &Path) -> AgentSession {
        let db_path = notes_dir.parent().unwrap().join("test.db");
        let conn = crate::db::open_db(&db_path).unwrap();
        let repo: Arc<dyn NoteRepository> = Arc::new(FsSqliteNoteRepository::new(notes_dir, Arc::new(Mutex::new(conn))));
        AgentSession::new(
            notes_dir,
            repo,
            backup_dir,
            PermissionManager::new(PermissionConfig::default()),
            LlmClient::new(LlmConfig::default()),
        )
    }

    #[test]
    fn test_apply_note_edit_and_undo() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let note_file = notes_dir.join("meeting.adoc");
        fs::write(&note_file, "= Meeting\nInitial agenda").unwrap();

        let mut session = make_session(&notes_dir, &backup_dir);

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
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

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

    #[test]
    fn test_read_and_edit_note_with_adoc_extension() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        // Create a note with .adoc extension
        let note_path = notes_dir.join("test_note.adoc");
        fs::write(&note_path, "= Test Note\nOriginal text.").unwrap();

        // Reading with filename containing .adoc
        let read_call = ToolCall {
            id: Some("read_1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "test_note.adoc" }),
            },
        };
        let read_out = session.execute_tool(&read_call);
        assert_eq!(read_out, "= Test Note\nOriginal text.");

        // Editing with filename containing .adoc
        let edit_call = ToolCall {
            id: Some("edit_1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: TOOL_EDIT_NOTE.to_string(),
                arguments: json!({
                    "filename": "test_note.adoc",
                    "content": "= Test Note\nModified text.",
                    "reason": "Test edit"
                }),
            },
        };
        let edit_out = session.execute_tool(&edit_call);
        assert!(edit_out.contains("Successfully updated note 'test_note.adoc'"));

        // Verify the file was updated on disk at test_note.adoc, not test_note_adoc
        assert!(note_path.exists());
        let updated = fs::read_to_string(&note_path).unwrap();
        assert_eq!(updated, "= Test Note\nModified text.");
        assert!(!notes_dir.join("test_note_adoc").exists());
    }

    #[test]
    fn test_non_blocking_status_inspection() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        assert!(!session.is_busy());
        assert!(!session.can_undo());
        assert!(session.pending_action().is_none());
        assert_eq!(session.messages().len(), 0);
    }

    #[test]
    fn test_read_note_via_repository_not_fs() {
        // Verify the agent reads through the repository, not direct filesystem access.
        // The repository's safe_note_path prevents path traversal.
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        // Write a secret outside notes dir
        fs::write(tmp.path().join("secret.txt"), "classified data").unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        // Attempt traversal via the read_note tool
        let read_call = ToolCall {
            id: Some("traversal_1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "../secret.txt" }),
            },
        };
        let output = session.execute_tool(&read_call);
        assert!(output.starts_with("Error reading note"), "traversal must fail: {}", output);
        assert!(!output.contains("classified"), "must not leak secret content");
    }

    #[test]
    fn test_search_uses_repository() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        // Create a note via the tool
        let create_call = ToolCall {
            id: Some("c1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({ "title": "Searchable", "content": "= Searchable\nUnique keyword: xyzzy" }),
            },
        };
        session.execute_tool(&create_call);

        // Search via the tool — verifies the repository's FTS integration works
        let search_call = ToolCall {
            id: Some("s1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "search_notes".to_string(),
                arguments: json!({ "query": "xyzzy" }),
            },
        };
        let result = session.execute_tool(&search_call);
        assert!(result.contains("Searchable"), "search should find the note: {}", result);
    }

    #[test]
    fn test_list_uses_repository() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        // Create notes
        for title in &["Alpha", "Beta", "Gamma"] {
            let call = ToolCall {
                id: Some(format!("c_{}", title)),
                tool_type: "function".to_string(),
                function: crate::agent::tools::FunctionCall {
                    name: "create_note".to_string(),
                    arguments: json!({ "title": title, "content": format!("= {}\n", title) }),
                },
            };
            session.execute_tool(&call);
        }

        let list_call = ToolCall {
            id: Some("list1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "list_notes".to_string(),
                arguments: json!({}),
            },
        };
        let result = session.execute_tool(&list_call);
        assert!(result.contains("Alpha"), "should list Alpha: {}", result);
        assert!(result.contains("Beta"), "should list Beta: {}", result);
        assert!(result.contains("Gamma"), "should list Gamma: {}", result);
    }
}
