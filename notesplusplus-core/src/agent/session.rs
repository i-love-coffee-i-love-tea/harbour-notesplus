//! Agent chat session and multi-turn tool execution loop.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use serde_json::json;
use crate::agent::backup::BackupManager;
use crate::agent::client::{ChatMessage, LlmClient};
use crate::agent::permissions::{PendingConfirmation, PermissionDecision, PermissionManager};
use crate::agent::prompt::{build_system_prompt_layered, EnvironmentContext};
use crate::agent::tools::{
    ToolCall, TOOL_APPEND_TO_NOTE, TOOL_CREATE_NOTE, TOOL_EDIT_NOTE, TOOL_EDIT_SECTION,
    TOOL_FETCH_URL, TOOL_INSERT_SECTION, TOOL_LIST_GROUPS, TOOL_LIST_NOTES, TOOL_MOVE_NOTE,
    TOOL_READ_NOTE, TOOL_RETRIEVE_CONTEXT, TOOL_SEARCH_NOTES,
};
use crate::error::NotesError;
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
    pub remaining_tool_calls: Vec<ToolCall>,
    pub last_snapshot_id: Option<String>,
    pub last_created_note: Option<String>,
}

const MAX_TOOL_OUTPUT_CHARS: usize = 16_000;
const MAX_CONVERSATION_MESSAGES: usize = 28;

fn truncate_tool_output(output: String) -> String {
    if output.len() > MAX_TOOL_OUTPUT_CHARS {
        let mut truncated: String = output.chars().take(MAX_TOOL_OUTPUT_CHARS).collect();
        truncated.push_str("\n... [Output truncated to 16,000 characters]");
        truncated
    } else {
        output
    }
}

fn prune_history_if_needed(messages: &mut Vec<ChatMessage>) {
    if messages.len() <= MAX_CONVERSATION_MESSAGES {
        return;
    }

    let sys_msg = messages.first().cloned();
    let target_keep = 18;
    let mut drop_idx = messages.len().saturating_sub(target_keep);

    // Ensure we don't start slicing on a tool result message (which requires preceding assistant tool_calls)
    while drop_idx < messages.len() && (messages[drop_idx].role == "tool" || messages[drop_idx].tool_call_id.is_some()) {
        drop_idx += 1;
    }

    let mut compacted = Vec::with_capacity(messages.len() - drop_idx + 1);
    if let Some(sys) = sys_msg {
        compacted.push(sys);
    }
    compacted.extend(messages.drain(drop_idx..));
    *messages = compacted;
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
        let total_notes = self.repository.list_pages().ok().map(|l| l.len());
        let groups_list = self.repository.list_groups(None, None).ok().map(|groups| {
            groups.into_iter().map(|g| g.path).collect::<Vec<_>>()
        });
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        let (filename, content) = match active_note {
            Some((f, c)) => (Some(f), Some(c)),
            None => (None, None),
        };

        let env = EnvironmentContext {
            current_date_time: Some(&now_str),
            active_note_filename: filename,
            active_note_title: None,
            active_note_content: content,
            extra_context,
            total_notes_count: total_notes,
            existing_groups: groups_list.as_deref(),
        };

        let sys_prompt = build_system_prompt_layered(
            custom_sys.as_deref(),
            &env,
        );

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.messages.clear();
        state.pending_action = None;
        state.remaining_tool_calls.clear();
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

    /// Appends a user prompt and drives the conversation loop with streaming token and status callbacks.
    pub fn send_prompt_streaming_with_status<F: FnMut(&str), S: FnMut(&str, &str)>(
        &mut self,
        user_prompt: &str,
        on_token: F,
        on_status: S,
    ) -> AgentStepResult {
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
        self.run_loop_streaming_with_status(on_token, on_status)
    }

    /// Appends a user prompt and drives the conversation loop with a streaming token callback.
    pub fn send_prompt_streaming<F: FnMut(&str)>(&mut self, user_prompt: &str, on_token: F) -> AgentStepResult {
        self.send_prompt_streaming_with_status(user_prompt, on_token, |_, _| {})
    }

    /// Resolves pending confirmation and resumes the loop with streaming token and status callbacks.
    pub fn confirm_pending_action_streaming_with_status<F: FnMut(&str), S: FnMut(&str, &str)>(
        &mut self,
        approved: bool,
        on_token: F,
        on_status: S,
    ) -> AgentStepResult {
        let (pending, remaining) = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let p = match state.pending_action.take() {
                Some(p) => p,
                None => return AgentStepResult::Error("No pending action to confirm".to_string()),
            };
            let rem = std::mem::take(&mut state.remaining_tool_calls);
            (p, rem)
        };

        if approved {
            let result_str = if pending.tool_name == TOOL_MOVE_NOTE {
                match self.repository.move_page(&pending.filename, &pending.new_content) {
                    Ok(p) => format!("Successfully moved note '{}' to group '{}' (full path: '{}')", pending.filename, p.group_path, p.full_path()),
                    Err(e) => format!("Error moving note '{}': {}", pending.filename, e),
                }
            } else {
                self.apply_note_edit(&pending.filename, &pending.new_content, &pending.reason)
            };
            let result_str = truncate_tool_output(result_str);
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                result_str,
            ));
        } else {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.messages.push(ChatMessage::tool_result(
                pending.tool_call_id,
                pending.tool_name,
                format!("User rejected the proposed changes to '{}'.", pending.filename),
            ));
        }

        // Add tool results for remaining tool calls in the same turn
        if !remaining.is_empty() {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            for tc in remaining {
                state.messages.push(ChatMessage::tool_result(
                    tc.id,
                    tc.function.name,
                    "Skipped: previous tool edit required user confirmation and halted subsequent actions in the batch.".to_string(),
                ));
            }
        }

        self.run_loop_streaming_with_status(on_token, on_status)
    }

    /// Resolves pending confirmation (approving or rejecting) and resumes the loop with a streaming token callback.
    pub fn confirm_pending_action_streaming<F: FnMut(&str)>(&mut self, approved: bool, on_token: F) -> AgentStepResult {
        self.confirm_pending_action_streaming_with_status(approved, on_token, |_, _| {})
    }

    /// Runs the LLM tool execution loop with token and status streaming until an answer or confirmation gate is reached.
    fn run_loop_streaming_with_status<F: FnMut(&str), S: FnMut(&str, &str)>(&self, mut on_token: F, mut on_status: S) -> AgentStepResult {
        self.is_busy.store(true, Ordering::SeqCst);
        let result = self.do_run_loop_streaming(&mut on_token, &mut on_status);
        self.is_busy.store(false, Ordering::SeqCst);
        result
    }

    fn do_run_loop_streaming<F: FnMut(&str), S: FnMut(&str, &str)>(
        &self,
        on_token: &mut F,
        on_status: &mut S,
    ) -> AgentStepResult {
        let mut turns = 0;

        while turns < self.max_tool_turns {
            turns += 1;

            on_status("thinking", "Thinking...");

            {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                prune_history_if_needed(&mut state.messages);
            }

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
                    let tool_name = tool_call.function.name.as_str();
                    let args = &tool_call.function.arguments;

                    // Emit sub-step status for the active tool invocation
                    match tool_name {
                        TOOL_SEARCH_NOTES => {
                            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("searching", &format!("Searching notes for '{}'...", q));
                        }
                        TOOL_RETRIEVE_CONTEXT => {
                            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("searching", &format!("Retrieving context for '{}'...", q));
                        }
                        TOOL_READ_NOTE => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("reading", &format!("Reading '{}'...", f));
                        }
                        TOOL_FETCH_URL => {
                            let u = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("reading", &format!("Fetching URL '{}'...", u));
                        }
                        TOOL_LIST_NOTES => {
                            on_status("reading", "Listing notes library...");
                        }
                        TOOL_LIST_GROUPS => {
                            on_status("reading", "Listing note groups and categories...");
                        }
                        TOOL_MOVE_NOTE => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            let g = args.get("target_group").and_then(|v| v.as_str()).unwrap_or("root");
                            let g_label = if g.is_empty() { "root" } else { g };
                            on_status("editing", &format!("Moving note '{}' to '{}'...", f, g_label));
                        }
                        TOOL_EDIT_NOTE => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("editing", &format!("Updating '{}'...", f));
                        }
                        TOOL_EDIT_SECTION => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            let h = args.get("heading").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("editing", &format!("Editing section '{}' in '{}'...", h, f));
                        }
                        TOOL_APPEND_TO_NOTE => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            on_status("editing", &format!("Appending to '{}'...", f));
                        }
                        TOOL_INSERT_SECTION => {
                            let f = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                            let t = args.get("title").and_then(|v| v.as_str()).unwrap_or("Section");
                            on_status("editing", &format!("Inserting section '{}' into '{}'...", t, f));
                        }
                        TOOL_CREATE_NOTE => {
                            let t = args.get("title").and_then(|v| v.as_str()).unwrap_or("Note");
                            on_status("editing", &format!("Creating note '{}'...", t));
                        }
                        _ => {
                            on_status("thinking", &format!("Executing {}...", tool_name));
                        }
                    }

                    let file_content = match tool_name {
                        TOOL_EDIT_NOTE | TOOL_EDIT_SECTION | TOOL_APPEND_TO_NOTE | TOOL_INSERT_SECTION => {
                            let filename = args.get("filename")
                                .and_then(|v| v.as_str()).unwrap_or("");
                            self.repository.read_note_content(filename).ok()
                        }
                        _ => None,
                    };

                    let decision = {
                        let perm = self.permission_mgr.lock().unwrap_or_else(|e| e.into_inner());
                        perm.evaluate(tool_call, file_content.as_deref())
                    };

                    match decision {
                        PermissionDecision::Allowed => {
                            let output = self.execute_tool(tool_call);
                            let output = truncate_tool_output(output);
                            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                            state.messages.push(ChatMessage::tool_result(
                                tool_call.id.clone(),
                                tool_call.function.name.clone(),
                                output,
                            ));
                        }
                        PermissionDecision::RequiresConfirmation(pending) => {
                            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                            let remaining: Vec<ToolCall> = response.tool_calls.iter()
                                .skip_while(|tc| tc.id != tool_call.id)
                                .skip(1)
                                .cloned()
                                .collect();
                            state.remaining_tool_calls = remaining;
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
                on_status("drafting", "Synthesizing response...");
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
            TOOL_RETRIEVE_CONTEXT => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                match self.repository.search_pages(query) {
                    Ok(results) => {
                        if results.is_empty() {
                            format!("No relevant note context found in index for query '{}'.", query)
                        } else {
                            let limit = max_results.max(1).min(10);
                            let mut retrieved = Vec::new();
                            for hit in results.into_iter().take(limit) {
                                let filename = hit.page.filename.clone();
                                let title = hit.page.title.clone();
                                let snippet = hit.snippet.clone();
                                let full_content = self.repository.read_note_content(&filename).unwrap_or_default();
                                let excerpt = if full_content.len() > 1200 {
                                    let mut head: String = full_content.chars().take(1200).collect();
                                    head.push_str("\n... [Excerpt truncated]");
                                    head
                                } else {
                                    full_content
                                };
                                retrieved.push(json!({
                                    "filename": filename,
                                    "title": title,
                                    "search_snippet": snippet,
                                    "content_excerpt": excerpt,
                                }));
                            }
                            serde_json::to_string_pretty(&retrieved).unwrap_or_else(|_| "[]".to_string())
                        }
                    }
                    Err(e) => format!("Context retrieval error: {}", e),
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
            TOOL_LIST_GROUPS => {
                match self.repository.list_groups(None, None) {
                    Ok(groups) => {
                        let group_list: Vec<serde_json::Value> = groups.iter().map(|g| {
                            json!({
                                "path": g.path,
                                "display_name": g.display_name,
                                "note_count": g.note_count,
                                "child_group_count": g.child_group_count,
                            })
                        }).collect();
                        serde_json::to_string_pretty(&group_list).unwrap_or_else(|_| "[]".to_string())
                    }
                    Err(e) => format!("List groups error: {}", e),
                }
            }
            TOOL_MOVE_NOTE => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let target_group = args.get("target_group").and_then(|v| v.as_str()).unwrap_or("");
                match self.repository.move_page(filename, target_group) {
                    Ok(p) => format!("Successfully moved note '{}' to group '{}' (full path: '{}')", filename, p.group_path, p.full_path()),
                    Err(e) => format!("Error moving note '{}': {}", filename, e),
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
            TOOL_EDIT_SECTION => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let heading = args.get("heading").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let new_heading = args.get("new_heading").and_then(|v| v.as_str());
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Updated note section");

                let old_content = match self.repository.read_note_content(filename) {
                    Ok(c) => c,
                    Err(e) => return format!("Error reading note '{}': {}", filename, e),
                };

                match crate::agent::section_editor::edit_section(&old_content, heading, content, new_heading) {
                    Ok(new_content) => self.apply_note_edit(filename, &new_content, reason),
                    Err(e) => format!("Error editing section: {}", e),
                }
            }
            TOOL_APPEND_TO_NOTE => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let heading = args.get("heading").and_then(|v| v.as_str());
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Appended content to note");

                let old_content = match self.repository.read_note_content(filename) {
                    Ok(c) => c,
                    Err(_) => String::new(),
                };

                match crate::agent::section_editor::append_to_note(&old_content, content, heading) {
                    Ok(new_content) => self.apply_note_edit(filename, &new_content, reason),
                    Err(e) => format!("Error appending to note: {}", e),
                }
            }
            TOOL_INSERT_SECTION => {
                let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("New Section");
                let level = args.get("level").and_then(|v| v.as_u64()).map(|l| l as usize).unwrap_or(2);
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let pos_str = args.get("position").and_then(|v| v.as_str()).unwrap_or("after_heading");
                let target_heading = args.get("target_heading").and_then(|v| v.as_str()).unwrap_or("");
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("Inserted section into note");

                let pos = match pos_str {
                    "before_heading" => crate::agent::section_editor::InsertPosition::BeforeHeading(target_heading),
                    "top" => crate::agent::section_editor::InsertPosition::Top,
                    "bottom" => crate::agent::section_editor::InsertPosition::Bottom,
                    _ => {
                        if target_heading.is_empty() {
                            crate::agent::section_editor::InsertPosition::Bottom
                        } else {
                            crate::agent::section_editor::InsertPosition::AfterHeading(target_heading)
                        }
                    }
                };

                let old_content = match self.repository.read_note_content(filename) {
                    Ok(c) => c,
                    Err(_) => String::new(),
                };

                match crate::agent::section_editor::insert_section(&old_content, title, level, content, pos) {
                    Ok(new_content) => self.apply_note_edit(filename, &new_content, reason),
                    Err(e) => format!("Error inserting section: {}", e),
                }
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
    pub fn undo_last_action(&mut self) -> Result<String, NotesError> {
        let snapshot_id = self.state.lock().unwrap_or_else(|e| e.into_inner()).last_snapshot_id.clone()
            .ok_or_else(|| NotesError::Msg("No previous action available to undo".to_string()))?;
        self.rollback_snapshot(&snapshot_id)
    }

    /// Rolls back a note to an arbitrary snapshot by ID.
    pub fn rollback_snapshot(&mut self, snapshot_id: &str) -> Result<String, NotesError> {
        let snapshot = {
            let backup = self.backup_mgr.lock().unwrap_or_else(|e| e.into_inner());
            let snap = backup.get_snapshot(snapshot_id)
                .map_err(|e| NotesError::Msg(format!("Failed to read snapshot: {}", e)))?
                .ok_or_else(|| NotesError::Msg(format!("Snapshot '{}' not found", snapshot_id)))?;
            let _ = backup.delete_snapshot(snapshot_id);
            snap
        };

        self.repository.save_note(&snapshot.filename, &snapshot.content)
            .map_err(|e| NotesError::Msg(format!("Failed to restore note: {}", e)))?;

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

    #[test]
    fn test_execute_edit_section_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);
        let note_path = notes_dir.join("roadmap.adoc");
        fs::write(&note_path, "= Roadmap\n\n== Milestones\n* [ ] Alpha\n\n== Team\nAlice & Bob").unwrap();

        let edit_sec_call = ToolCall {
            id: Some("es1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "edit_section".to_string(),
                arguments: json!({
                    "filename": "roadmap.adoc",
                    "heading": "Milestones",
                    "content": "* [x] Alpha\n* [ ] Beta"
                }),
            },
        };
        let res = session.execute_tool(&edit_sec_call);
        assert!(res.contains("Successfully updated note"), "{}", res);

        let read_call = ToolCall {
            id: Some("r1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "roadmap.adoc" }),
            },
        };
        let content = session.execute_tool(&read_call);
        assert!(content.contains("== Milestones\n* [x] Alpha\n* [ ] Beta"));
        assert!(content.contains("== Team\nAlice & Bob"));
    }

    #[test]
    fn test_execute_append_to_note_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);
        let note_path = notes_dir.join("journal.adoc");
        fs::write(&note_path, "= Daily Journal\n\n== Morning\nStarted work.").unwrap();

        let append_call = ToolCall {
            id: Some("ap1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "append_to_note".to_string(),
                arguments: json!({
                    "filename": "journal.adoc",
                    "content": "== Evening\nFinished release."
                }),
            },
        };
        let res = session.execute_tool(&append_call);
        assert!(res.contains("Successfully updated note"), "{}", res);

        let read_call = ToolCall {
            id: Some("r1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "journal.adoc" }),
            },
        };
        let content = session.execute_tool(&read_call);
        assert!(content.contains("== Morning\nStarted work."));
        assert!(content.contains("== Evening\nFinished release."));
    }

    #[test]
    fn test_execute_insert_section_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);
        let note_path = notes_dir.join("doc.adoc");
        fs::write(&note_path, "= Doc Title\n\n== Intro\nHello.\n\n== Conclusion\nBye.").unwrap();

        let insert_call = ToolCall {
            id: Some("ins1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "insert_section".to_string(),
                arguments: json!({
                    "filename": "doc.adoc",
                    "title": "Body",
                    "level": 2,
                    "content": "Core analysis.",
                    "position": "before_heading",
                    "target_heading": "Conclusion"
                }),
            },
        };
        let res = session.execute_tool(&insert_call);
        assert!(res.contains("Successfully updated note"), "{}", res);

        let read_call = ToolCall {
            id: Some("r1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "doc.adoc" }),
            },
        };
        let content = session.execute_tool(&read_call);
        assert!(content.contains("== Body\nCore analysis."));
        assert!(content.find("== Body").unwrap() < content.find("== Conclusion").unwrap());
    }

    #[test]
    fn test_execute_retrieve_context_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        let create_call = ToolCall {
            id: Some("c1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({
                    "title": "Project Nebula",
                    "content": "= Project Nebula\nQuantum computing research notes."
                }),
            },
        };
        session.execute_tool(&create_call);

        let retrieve_call = ToolCall {
            id: Some("rc1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "retrieve_context".to_string(),
                arguments: json!({
                    "query": "Quantum",
                    "max_results": 3
                }),
            },
        };
        let result = session.execute_tool(&retrieve_call);
        assert!(result.contains("Project Nebula"), "{}", result);
        assert!(result.contains("Quantum computing"), "{}", result);
    }

    #[test]
    fn test_react_loop_status_emission() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let mut session = make_session(&notes_dir, &backup_dir);

        let mut statuses = Vec::new();
        let _ = session.send_prompt_streaming_with_status(
            "Hello test",
            |_| {},
            |status, detail| {
                statuses.push((status.to_string(), detail.to_string()));
            },
        );

        // At least the initial thinking status must have been emitted
        assert!(!statuses.is_empty());
        assert_eq!(statuses[0].0, "thinking");
        assert_eq!(statuses[0].1, "Thinking...");
    }

    #[test]
    fn test_execute_list_groups_and_move_note_tool() {
        let tmp = tempdir().unwrap();
        let notes_dir = tmp.path().join("notes");
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&notes_dir).unwrap();

        let session = make_session(&notes_dir, &backup_dir);

        // Create a note in root and a note in Work
        let create_call1 = ToolCall {
            id: Some("c1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({
                    "title": "Meeting",
                    "content": "= Meeting\nNotes here.",
                    "group": "Work"
                }),
            },
        };
        session.execute_tool(&create_call1);

        let create_call2 = ToolCall {
            id: Some("c2".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({
                    "title": "Journal",
                    "content": "= Journal\nMy thoughts."
                }),
            },
        };
        session.execute_tool(&create_call2);

        // 1. Test list_groups
        let list_groups_call = ToolCall {
            id: Some("lg1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "list_groups".to_string(),
                arguments: json!({}),
            },
        };
        let groups_json = session.execute_tool(&list_groups_call);
        assert!(groups_json.contains("\"path\": \"Work\""), "{}", groups_json);

        // 2. Test move_note
        let move_call = ToolCall {
            id: Some("m1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "move_note".to_string(),
                arguments: json!({
                    "filename": "Journal",
                    "target_group": "Personal/Daily",
                    "reason": "Organize journal into Personal/Daily"
                }),
            },
        };
        let move_result = session.execute_tool(&move_call);
        assert!(move_result.contains("Successfully moved note"), "{}", move_result);

        // Verify moved note location
        let read_call = ToolCall {
            id: Some("r1".to_string()),
            tool_type: "function".to_string(),
            function: crate::agent::tools::FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "Personal/Daily/Journal" }),
            },
        };
        let content = session.execute_tool(&read_call);
        assert!(content.contains("= Journal\nMy thoughts."));
    }
}
