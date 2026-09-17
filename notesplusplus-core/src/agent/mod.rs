//! AI Assistant subsystem with Ollama and Xiaomi MiMoCode / OpenAI compatible tool calling.

pub mod client;
pub mod prompt;
pub mod tools;
pub mod diff;
pub mod backup;
pub mod permissions;
pub mod session;
pub mod section_editor;

pub use backup::{BackupManager, Snapshot};
pub use client::{AssistantResponse, ChatMessage, LlmClient, LlmConfig, LlmProvider, ModelInfo, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_MODEL};
pub use diff::{compute_line_diff, DiffLine, DiffLineType, DiffSummary};
pub use permissions::{PendingConfirmation, PermissionConfig, PermissionDecision, PermissionManager};
pub use prompt::{
    build_custom_instruction, build_import_instruction, build_system_prompt,
    build_system_prompt_layered, build_system_prompt_with_custom,
    build_template_instruction, build_template_instruction_ex,
    EnvironmentContext, ASSISTANT_IDENTITY, DEFAULT_SYSTEM_PROMPT,
};
pub use section_editor::{
    append_to_note, edit_section, find_section_span, insert_section, list_sections,
    InsertPosition, SectionEditorError, SectionInfo, SectionSpan,
};
pub use session::{AgentSession, AgentStepResult};
pub use tools::{fetch_url, get_available_tools, FunctionCall, FunctionDefinition, ToolCall, ToolDefinition};
