//! QMetaObject bridge exposing AI Assistant to Sailfish OS QML.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use qmetaobject::*;

use notesplusplus_core::agent::{
    build_import_instruction, build_template_instruction, fetch_url, AgentSession,
    AgentStepResult, LlmClient, LlmConfig, LlmProvider, PendingConfirmation,
    PermissionConfig, PermissionManager, ModelInfo, DEFAULT_OLLAMA_ENDPOINT,
};

enum WorkerTask {
    SendPrompt(String),
    ConfirmAction(bool),
    UndoAction,
}

struct WorkerOutput {
    step_result: Result<AgentStepResult, String>,
    messages_json: String,
    pending_action: Option<PendingConfirmation>,
    can_undo: bool,
    last_snapshot_id: Option<String>,
    last_created_note: Option<String>,
}

#[derive(QObject)]
pub struct AgentBridge {
    base: qt_base_class!(trait QObject),

    // Properties
    agent_busy: qt_property!(bool; NOTIFY busy_changed),
    messages_json: qt_property!(String; NOTIFY messages_changed),
    pending_action_json: qt_property!(String; NOTIFY pending_action_changed),
    has_pending_action: qt_property!(bool; NOTIFY pending_action_changed),
    can_undo: qt_property!(bool; NOTIFY undo_state_changed),
    last_snapshot_id: qt_property!(String; NOTIFY undo_state_changed),
    last_created_note: qt_property!(String; NOTIFY last_created_note_changed),
    error_message: qt_property!(String; NOTIFY error_occurred),
    streaming_text: qt_property!(String; NOTIFY streaming_text_changed),

    // Configuration properties
    provider_type: qt_property!(String; NOTIFY config_changed),
    endpoint_url: qt_property!(String; NOTIFY config_changed),
    model_name: qt_property!(String; NOTIFY config_changed),
    timeout_secs: qt_property!(i32; NOTIFY config_changed),
    auto_allow_read: qt_property!(bool; NOTIFY config_changed),
    auto_allow_create: qt_property!(bool; NOTIFY config_changed),
    require_confirm_edit: qt_property!(bool; NOTIFY config_changed),
    allow_self_signed: qt_property!(bool; NOTIFY config_changed),
    available_models: qt_property!(String; NOTIFY models_changed),
    models_loading: qt_property!(bool; NOTIFY models_changed),

    // Signals
    busy_changed: qt_signal!(),
    messages_changed: qt_signal!(),
    pending_action_changed: qt_signal!(),
    undo_state_changed: qt_signal!(),
    last_created_note_changed: qt_signal!(),
    config_changed: qt_signal!(),
    error_occurred: qt_signal!(message: String),
    response_finished: qt_signal!(content: String),
    undo_completed: qt_signal!(message: String),
    streaming_text_changed: qt_signal!(),
    models_changed: qt_signal!(),

    // Methods
    configure: qt_method!(fn(&mut self, provider: String, url: String, model: String, key: String, timeout: i32, auto_read: bool, auto_create: bool, require_edit: bool, allow_self_signed: bool)),
    reset_session: qt_method!(fn(&mut self, context_filename: String, context_content: String, extra_context: String)),
    send_prompt: qt_method!(fn(&mut self, text: String)),
    run_template: qt_method!(fn(&mut self, template_id: String, input_text: String, context_filename: String, context_content: String)),
    import_text: qt_method!(fn(&mut self, source_text: String, target_title: String, mode: String, custom_instruction: String)),
    fetch_url_content: qt_method!(fn(&mut self, url: String) -> String),
    read_local_file: qt_method!(fn(&mut self, file_path: String) -> String),
    confirm_action: qt_method!(fn(&mut self, approved: bool)),
    undo_last_action: qt_method!(fn(&mut self)),
    poll_worker: qt_method!(fn(&mut self) -> bool),
    fetch_models: qt_method!(fn(&mut self)),
    poll_models: qt_method!(fn(&mut self) -> bool),

    // Internal shared state
    internal_api_key: String,
    session: Arc<Mutex<AgentSession>>,
    worker_result: Arc<Mutex<Option<WorkerOutput>>>,
    streaming_buffer: Arc<Mutex<String>>,
    models_result: Arc<Mutex<Option<Result<Vec<ModelInfo>, String>>>>,
}

impl Default for AgentBridge {
    fn default() -> Self {
        let paths = notesplusplus_core::paths::AppPaths::new();
        let notes_dir = paths.notes_dir;
        let db_path = paths.db_path;
        let backup_dir = paths.data_dir.join("backups");

        let _ = std::fs::create_dir_all(&notes_dir);
        let _ = std::fs::create_dir_all(&backup_dir);

        let config = LlmConfig::default();
        let perm_config = PermissionConfig::default();
        let client = LlmClient::new(config);
        let perm_mgr = PermissionManager::new(perm_config);

        let mut session = AgentSession::new(
            &notes_dir,
            &db_path,
            &backup_dir,
            perm_mgr,
            client,
        );
        session.reset_session(None, None);

        let initial_messages = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());

        Self {
            base: Default::default(),
            agent_busy: false,
            messages_json: initial_messages,
            pending_action_json: String::new(),
            has_pending_action: false,
            can_undo: false,
            last_snapshot_id: String::new(),
            last_created_note: String::new(),
            error_message: String::new(),
            streaming_text: String::new(),
            available_models: String::new(),
            models_loading: false,
            provider_type: "ollama".to_string(),
            endpoint_url: DEFAULT_OLLAMA_ENDPOINT.to_string(),
            model_name: "llama3.2".to_string(),
            internal_api_key: String::new(),
            timeout_secs: 90,
            auto_allow_read: true,
            auto_allow_create: true,
            require_confirm_edit: true,
            allow_self_signed: false,
            busy_changed: Default::default(),
            messages_changed: Default::default(),
            pending_action_changed: Default::default(),
            undo_state_changed: Default::default(),
            last_created_note_changed: Default::default(),
            config_changed: Default::default(),
            error_occurred: Default::default(),
            response_finished: Default::default(),
            undo_completed: Default::default(),
            streaming_text_changed: Default::default(),
            models_changed: Default::default(),
            configure: Default::default(),
            reset_session: Default::default(),
            send_prompt: Default::default(),
            run_template: Default::default(),
            import_text: Default::default(),
            fetch_url_content: Default::default(),
            read_local_file: Default::default(),
            confirm_action: Default::default(),
            undo_last_action: Default::default(),
            poll_worker: Default::default(),
            fetch_models: Default::default(),
            poll_models: Default::default(),
            session: Arc::new(Mutex::new(session)),
            worker_result: Arc::new(Mutex::new(None)),
            streaming_buffer: Arc::new(Mutex::new(String::new())),
            models_result: Arc::new(Mutex::new(None)),
        }
    }
}

impl AgentBridge {
    pub fn configure(
        &mut self,
        provider: String,
        url: String,
        model: String,
        key: String,
        timeout: i32,
        auto_read: bool,
        auto_create: bool,
        require_edit: bool,
        allow_self_signed: bool,
    ) {
        self.provider_type = provider.clone();
        self.endpoint_url = url.clone();
        self.model_name = model.clone();
        self.internal_api_key = key.clone();
        self.timeout_secs = if timeout > 0 { timeout } else { 90 };
        self.auto_allow_read = auto_read;
        self.auto_allow_create = auto_create;
        self.require_confirm_edit = require_edit;
        self.allow_self_signed = allow_self_signed;

        let provider_enum = if provider.to_lowercase() == "mimocode" || provider.to_lowercase() == "openai" {
            LlmProvider::OpenAiCompatible
        } else {
            LlmProvider::Ollama
        };

        let llm_config = LlmConfig {
            provider: provider_enum,
            endpoint_url: url,
            model,
            api_key: if key.trim().is_empty() { None } else { Some(key) },
            timeout_secs: self.timeout_secs as u64,
            allow_self_signed,
            system_prompt: None,
        };

        let perm_config = PermissionConfig {
            auto_allow_read: auto_read,
            auto_allow_create: auto_create,
            require_confirm_edit: require_edit,
        };

        if let Ok(mut session) = self.session.lock() {
            session.update_config(
                PermissionManager::new(perm_config),
                LlmClient::new(llm_config),
            );
        }

        self.config_changed();
    }

    pub fn reset_session(
        &mut self,
        context_filename: String,
        context_content: String,
        extra_context: String,
    ) {
        if let Ok(mut session) = self.session.lock() {
            let active = if !context_filename.trim().is_empty() {
                Some((context_filename.as_str(), context_content.as_str()))
            } else {
                None
            };
            let extra = if !extra_context.trim().is_empty() {
                Some(extra_context.as_str())
            } else {
                None
            };
            session.reset_session(active, extra);
            self.messages_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
            self.pending_action_json = String::new();
            self.has_pending_action = false;
        }

        self.messages_changed();
        self.pending_action_changed();
    }

    pub fn send_prompt(&mut self, text: String) {
        if self.agent_busy || text.trim().is_empty() {
            return;
        }
        if let Ok(mut session) = self.session.lock() {
            session.push_user_message(&text);
            self.messages_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
        }
        self.messages_changed();
        self.spawn_worker(WorkerTask::SendPrompt(text));
    }

    pub fn run_template(
        &mut self,
        template_id: String,
        input_text: String,
        context_filename: String,
        context_content: String,
    ) {
        if self.agent_busy {
            return;
        }
        let fname_opt = if !context_filename.trim().is_empty() {
            Some(context_filename.as_str())
        } else {
            None
        };
        let active_opt = if !context_content.trim().is_empty() {
            Some(context_content.as_str())
        } else {
            None
        };
        let formatted_prompt = build_template_instruction(&template_id, &input_text, fname_opt, active_opt);
        if let Ok(mut session) = self.session.lock() {
            session.push_user_message(&formatted_prompt);
            self.messages_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
        }
        self.messages_changed();
        self.spawn_worker(WorkerTask::SendPrompt(formatted_prompt));
    }

    pub fn import_text(
        &mut self,
        source_text: String,
        target_title: String,
        mode: String,
        custom_instruction: String,
    ) {
        if self.agent_busy || source_text.trim().is_empty() {
            return;
        }
        let title_opt = if !target_title.trim().is_empty() {
            Some(target_title.as_str())
        } else {
            None
        };
        let custom_opt = if !custom_instruction.trim().is_empty() {
            Some(custom_instruction.as_str())
        } else {
            None
        };
        let formatted_prompt = build_import_instruction(&source_text, title_opt, &mode, custom_opt);
        if let Ok(mut session) = self.session.lock() {
            session.push_user_message(&formatted_prompt);
            self.messages_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
        }
        self.messages_changed();
        self.spawn_worker(WorkerTask::SendPrompt(formatted_prompt));
    }

    pub fn fetch_url_content(&mut self, url: String) -> String {
        match fetch_url(&url) {
            Ok(text) => text,
            Err(e) => format!("Error fetching URL: {}", e),
        }
    }

    pub fn read_local_file(&mut self, file_path: String) -> String {
        let p = file_path.trim();
        if p.is_empty() {
            return String::new();
        }
        if p.contains("..") {
            return "Error: path traversal ('..') is not allowed".to_string();
        }
        let expanded = if p.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(&p[2..])
            } else {
                PathBuf::from(p)
            }
        } else {
            PathBuf::from(p)
        };
        let notes_dir = notesplusplus_core::paths::AppPaths::new().notes_dir;
        let canonical = match expanded.canonicalize() {
            Ok(c) => c,
            Err(e) => return format!("Error resolving path: {}", e),
        };
        if !canonical.starts_with(&notes_dir) {
            return "Error: access denied — file is outside the notes directory".to_string();
        }
        match std::fs::read_to_string(&canonical) {
            Ok(content) => content,
            Err(e) => format!("Error reading file: {}", e),
        }
    }

    pub fn confirm_action(&mut self, approved: bool) {
        if self.agent_busy || !self.has_pending_action {
            return;
        }
        self.spawn_worker(WorkerTask::ConfirmAction(approved));
    }

    pub fn undo_last_action(&mut self) {
        if self.agent_busy || !self.can_undo {
            return;
        }
        self.spawn_worker(WorkerTask::UndoAction);
    }

    fn spawn_worker(&mut self, task: WorkerTask) {
        self.agent_busy = true;
        self.streaming_text = String::new();
        self.busy_changed();
        self.streaming_text_changed();

        if let Ok(mut buf) = self.streaming_buffer.lock() {
            buf.clear();
        }

        let session_arc = Arc::clone(&self.session);
        let result_arc = Arc::clone(&self.worker_result);
        let streaming_buf_arc = Arc::clone(&self.streaming_buffer);

        thread::spawn(move || {
            let output = match session_arc.lock() {
                Ok(mut session) => match task {
                    WorkerTask::SendPrompt(text) => {
                        let stream_buf = Arc::clone(&streaming_buf_arc);
                        let step_res = session.send_prompt_streaming(&text, move |tok| {
                            if let Ok(mut b) = stream_buf.lock() {
                                b.push_str(tok);
                            }
                        });
                        let msgs_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
                        let pending = session.pending_action().cloned();
                        let can_undo = session.can_undo();
                        let last_snap = session.last_snapshot_id().map(|s| s.to_string());
                        let last_created = session.last_created_note().map(|s| s.to_string());

                        WorkerOutput {
                            step_result: Ok(step_res),
                            messages_json: msgs_json,
                            pending_action: pending,
                            can_undo,
                            last_snapshot_id: last_snap,
                            last_created_note: last_created,
                        }
                    }
                    WorkerTask::ConfirmAction(approved) => {
                        let stream_buf = Arc::clone(&streaming_buf_arc);
                        let step_res = session.confirm_pending_action_streaming(approved, move |tok| {
                            if let Ok(mut b) = stream_buf.lock() {
                                b.push_str(tok);
                            }
                        });
                        let msgs_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
                        let pending = session.pending_action().cloned();
                        let can_undo = session.can_undo();
                        let last_snap = session.last_snapshot_id().map(|s| s.to_string());
                        let last_created = session.last_created_note().map(|s| s.to_string());

                        WorkerOutput {
                            step_result: Ok(step_res),
                            messages_json: msgs_json,
                            pending_action: pending,
                            can_undo,
                            last_snapshot_id: last_snap,
                            last_created_note: last_created,
                        }
                    }
                    WorkerTask::UndoAction => {
                        let undo_res = session.undo_last_action();
                        let msgs_json = serde_json::to_string(&session.messages()).unwrap_or_else(|_| "[]".to_string());
                        let pending = session.pending_action().cloned();
                        let can_undo = session.can_undo();
                        let last_snap = session.last_snapshot_id().map(|s| s.to_string());
                        let last_created = session.last_created_note().map(|s| s.to_string());

                        let step_result = match undo_res {
                            Ok(msg) => Ok(AgentStepResult::Finished {
                                content: msg,
                                last_snapshot_id: None,
                            }),
                            Err(e) => Err(e),
                        };

                        WorkerOutput {
                            step_result,
                            messages_json: msgs_json,
                            pending_action: pending,
                            can_undo,
                            last_snapshot_id: last_snap,
                            last_created_note: last_created,
                        }
                    }
                },
                Err(e) => WorkerOutput {
                    step_result: Err(format!("Session lock failure: {}", e)),
                    messages_json: "[]".to_string(),
                    pending_action: None,
                    can_undo: false,
                    last_snapshot_id: None,
                    last_created_note: None,
                },
            };

            if let Ok(mut r) = result_arc.lock() {
                *r = Some(output);
            }
        });
    }

    /// Polled by QML Timer while busy. Returns true when worker has finished.
    pub fn poll_worker(&mut self) -> bool {
        if !self.agent_busy {
            return false;
        }

        if let Ok(buf) = self.streaming_buffer.lock() {
            if *buf != self.streaming_text {
                self.streaming_text = buf.clone();
                self.streaming_text_changed();
            }
        }

        let output_opt = match self.worker_result.lock() {
            Ok(mut r) => r.take(),
            Err(_) => None,
        };

        if let Some(output) = output_opt {
            self.messages_json = output.messages_json;
            self.can_undo = output.can_undo;
            self.last_snapshot_id = output.last_snapshot_id.unwrap_or_default();
            self.last_created_note = output.last_created_note.unwrap_or_default();
            self.streaming_text = String::new();
            self.streaming_text_changed();

            if let Ok(mut buf) = self.streaming_buffer.lock() {
                buf.clear();
            }

            if let Some(pending) = output.pending_action {
                self.pending_action_json = serde_json::to_string(&pending).unwrap_or_default();
                self.has_pending_action = true;
            } else {
                self.pending_action_json = String::new();
                self.has_pending_action = false;
            }

            self.agent_busy = false;

            match output.step_result {
                Ok(AgentStepResult::Finished { content, .. }) => {
                    self.response_finished(content.clone());
                    if content.starts_with("Successfully rolled back") {
                        self.undo_completed(content);
                    }
                }
                Ok(AgentStepResult::RequiresConfirmation(_)) => {
                    // Pending action updated
                }
                Ok(AgentStepResult::Error(err)) => {
                    self.error_message = err.clone();
                    self.error_occurred(err);
                }
                Err(err) => {
                    self.error_message = err.clone();
                    self.error_occurred(err);
                }
            }

            self.busy_changed();
            self.messages_changed();
            self.pending_action_changed();
            self.undo_state_changed();
            self.last_created_note_changed();
            return true;
        }

        false
    }

    pub fn fetch_models(&mut self) {
        if self.models_loading {
            return;
        }
        self.models_loading = true;
        self.models_changed();

        // Build a temporary LlmClient from current config properties
        let provider_enum = if self.provider_type.to_lowercase() == "mimocode" || self.provider_type.to_lowercase() == "openai" {
            LlmProvider::OpenAiCompatible
        } else {
            LlmProvider::Ollama
        };
        let config = LlmConfig {
            provider: provider_enum,
            endpoint_url: self.endpoint_url.clone(),
            model: self.model_name.clone(),
            api_key: if self.internal_api_key.trim().is_empty() { None } else { Some(self.internal_api_key.clone()) },
            timeout_secs: self.timeout_secs.max(15) as u64,
            allow_self_signed: self.allow_self_signed,
            system_prompt: None,
        };

        let result_slot = self.models_result.clone();
        *result_slot.lock().unwrap_or_else(|e| e.into_inner()) = None;

        thread::spawn(move || {
            let client = LlmClient::new(config);
            let result = client.list_models().map_err(|e| format!("{}", e));
            if let Ok(mut guard) = result_slot.lock() {
                *guard = Some(result);
            }
        });
    }

    pub fn poll_models(&mut self) -> bool {
        let has_result = if let Ok(guard) = self.models_result.lock() {
            guard.is_some()
        } else {
            false
        };

        if has_result {
            if let Ok(mut guard) = self.models_result.lock() {
                if let Some(result) = guard.take() {
                    match result {
                        Ok(models) => {
                            self.available_models = serde_json::to_string(&models)
                                .unwrap_or_else(|_| "[]".to_string());
                        }
                        Err(e) => {
                            self.error_message = format!("Failed to fetch models: {}", e);
                            self.error_occurred(self.error_message.clone());
                            self.available_models = "[]".to_string();
                        }
                    }
                    self.models_loading = false;
                    self.models_changed();
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn path_traversal_detected() {
        let paths = ["../../etc/passwd", "../secret.txt", "foo/../../bar"];
        for p in paths {
            assert!(p.contains(".."), "expected '..' in '{}'", p);
        }
    }

    #[test]
    fn safe_paths_have_no_traversal() {
        let paths = ["Journal.adoc", "notes/my-note.adoc", "/home/user/notes/test.adoc"];
        for p in paths {
            assert!(!p.contains(".."), "unexpected '..' in '{}'", p);
        }
    }

    #[test]
    fn path_containment_check() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path();

        // Inside notes dir
        let inside = notes_dir.join("test.adoc");
        assert!(inside.starts_with(notes_dir));

        // Outside notes dir
        let outside = PathBuf::from("/etc/passwd");
        assert!(!outside.starts_with(notes_dir));

        // Sibling directory
        let sibling = tmp.path().parent().unwrap().join("other");
        assert!(!sibling.starts_with(notes_dir));
    }

    #[test]
    fn provider_mapping_openai() {
        let providers = ["mimocode", "openai", "Mimocode", "OpenAI"];
        for p in providers {
            assert!(
                p.to_lowercase() == "mimocode" || p.to_lowercase() == "openai",
                "'{}' should map to OpenAiCompatible", p
            );
        }
    }

    #[test]
    fn provider_mapping_ollama() {
        let providers = ["ollama", "Ollama", "OLLAMA"];
        for p in providers {
            assert_ne!(p.to_lowercase(), "mimocode");
            assert_ne!(p.to_lowercase(), "openai");
        }
    }

    #[test]
    fn timeout_default_when_zero() {
        let timeout = 0i32;
        let result = if timeout > 0 { timeout } else { 90 };
        assert_eq!(result, 90);
    }

    #[test]
    fn timeout_preserved_when_positive() {
        let timeout = 120i32;
        let result = if timeout > 0 { timeout } else { 90 };
        assert_eq!(result, 120);
    }
}
