//! Permission policy and interactive confirmation evaluation.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::agent::diff::{compute_line_diff, DiffSummary};
use crate::agent::tools::ToolCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub auto_allow_read: bool,
    pub auto_allow_create: bool,
    pub require_confirm_edit: bool,
    pub allow_fetch_url: bool,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            auto_allow_read: true,
            auto_allow_create: true,
            require_confirm_edit: true,
            allow_fetch_url: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub action_id: String,
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub filename: String,
    pub reason: String,
    pub new_content: String,
    pub diff: DiffSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Allowed,
    RequiresConfirmation(PendingConfirmation),
    Denied(String),
}

pub struct PermissionManager {
    config: PermissionConfig,
}

impl PermissionManager {
    pub fn new(config: PermissionConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PermissionConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: PermissionConfig) {
        self.config = config;
    }

    /// Evaluates whether a tool call can proceed automatically or requires user confirmation.
    pub fn evaluate(
        &self,
        tool_call: &ToolCall,
        existing_file_content: Option<&str>,
    ) -> PermissionDecision {
        let name = tool_call.function.name.as_str();
        match name {
            "read_note" | "list_notes" | "search_notes" => {
                if self.config.auto_allow_read {
                    PermissionDecision::Allowed
                } else {
                    PermissionDecision::Denied("Read operations are currently disabled by user configuration.".to_string())
                }
            }
            "fetch_url" => {
                if self.config.allow_fetch_url {
                    PermissionDecision::Allowed
                } else {
                    PermissionDecision::Denied("Web requests are currently disabled by user configuration.".to_string())
                }
            }
            "create_note" => {
                if self.config.auto_allow_create {
                    PermissionDecision::Allowed
                } else {
                    let title = tool_call.function.arguments.get("title")
                        .and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let content = tool_call.function.arguments.get("content")
                        .and_then(|v| v.as_str()).unwrap_or("");
                    let diff = compute_line_diff("", content);
                    let action_id = format!("act_create_{}", Utc::now().timestamp_millis());

                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: "create_note".to_string(),
                        filename: format!("{}.adoc", title.to_lowercase().replace(' ', "_")),
                        reason: format!("Create new note '{}'", title),
                        new_content: content.to_string(),
                        diff,
                    })
                }
            }
            "edit_note" => {
                let filename = tool_call.function.arguments.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("unknown.adoc");
                let new_content = tool_call.function.arguments.get("content")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let reason = tool_call.function.arguments.get("reason")
                    .and_then(|v| v.as_str()).unwrap_or("Update note content");

                let old_content = existing_file_content.unwrap_or("");
                let diff = compute_line_diff(old_content, new_content);

                if !self.config.require_confirm_edit {
                    PermissionDecision::Allowed
                } else {
                    let action_id = format!("act_edit_{}", Utc::now().timestamp_millis());
                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: "edit_note".to_string(),
                        filename: filename.to_string(),
                        reason: reason.to_string(),
                        new_content: new_content.to_string(),
                        diff,
                    })
                }
            }
            _ => PermissionDecision::Denied(format!("Unknown tool: {}", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::agent::tools::FunctionCall;

    #[test]
    fn test_auto_allow_read_and_create() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let read_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "read_note".to_string(),
                arguments: json!({ "filename": "test.adoc" }),
            },
        };
        assert_eq!(pm.evaluate(&read_call, None), PermissionDecision::Allowed);

        let create_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "create_note".to_string(),
                arguments: json!({ "title": "New Doc", "content": "= New Doc\nText" }),
            },
        };
        assert_eq!(pm.evaluate(&create_call, None), PermissionDecision::Allowed);
    }

    #[test]
    fn test_require_confirmation_for_edit_with_diff() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let edit_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "edit_note".to_string(),
                arguments: json!({
                    "filename": "meeting.adoc",
                    "content": "= Meeting\nUpdated summary",
                    "reason": "Added notes"
                }),
            },
        };

        let decision = pm.evaluate(&edit_call, Some("= Meeting\nOld draft"));
        match decision {
            PermissionDecision::RequiresConfirmation(pending) => {
                assert_eq!(pending.tool_name, "edit_note");
                assert_eq!(pending.filename, "meeting.adoc");
                assert_eq!(pending.reason, "Added notes");
                assert_eq!(pending.diff.additions, 1);
                assert_eq!(pending.diff.deletions, 1);
            }
            other => panic!("Expected RequiresConfirmation, got {:?}", other),
        }
    }
}
