//! AI Assistant subsystem with Ollama and Xiaomi MiMoCode / OpenAI compatible tool calling.

pub mod client;
pub mod prompt;
pub mod tools;
pub mod diff;
pub mod backup;
pub mod permissions;
pub mod session;

pub use backup::{BackupManager, Snapshot};
pub use client::{AssistantResponse, ChatMessage, LlmClient, LlmConfig, LlmError, LlmProvider};
pub use diff::{compute_line_diff, DiffLine, DiffLineType, DiffSummary};
pub use permissions::{PendingConfirmation, PermissionConfig, PermissionDecision, PermissionManager};
pub use prompt::{build_import_instruction, build_system_prompt, build_template_instruction};
pub use session::{AgentSession, AgentStepResult};
pub use tools::{fetch_url, get_available_tools, FunctionCall, FunctionDefinition, ToolCall, ToolDefinition};
