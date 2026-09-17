//! Permission policy and interactive confirmation evaluation.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::agent::diff::{compute_line_diff, DiffSummary};
use crate::agent::section_editor::{append_to_note, edit_section, insert_section, InsertPosition};
use crate::agent::tools::{
    ToolCall, TOOL_APPEND_TO_NOTE, TOOL_CREATE_NOTE, TOOL_EDIT_NOTE, TOOL_EDIT_SECTION,
    TOOL_FETCH_URL, TOOL_INSERT_SECTION, TOOL_LIST_GROUPS, TOOL_LIST_NOTES, TOOL_MOVE_NOTE,
    TOOL_READ_NOTE, TOOL_RETRIEVE_CONTEXT, TOOL_SEARCH_NOTES,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
            TOOL_READ_NOTE | TOOL_LIST_NOTES | TOOL_SEARCH_NOTES | TOOL_RETRIEVE_CONTEXT | TOOL_LIST_GROUPS => {
                if self.config.auto_allow_read {
                    PermissionDecision::Allowed
                } else {
                    PermissionDecision::Denied("Read operations are currently disabled by user configuration.".to_string())
                }
            }
            TOOL_FETCH_URL => {
                if self.config.allow_fetch_url {
                    PermissionDecision::Allowed
                } else {
                    PermissionDecision::Denied("Web requests are currently disabled by user configuration.".to_string())
                }
            }
            TOOL_MOVE_NOTE => {
                let filename = tool_call.function.arguments.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("unknown.adoc");
                let target_group = tool_call.function.arguments.get("target_group")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let reason = tool_call.function.arguments.get("reason")
                    .and_then(|v| v.as_str())
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| {
                        if target_group.is_empty() {
                            format!("Move note '{}' to root library", filename)
                        } else {
                            format!("Move note '{}' to group '{}'", filename, target_group)
                        }
                    });

                if !self.config.require_confirm_edit {
                    PermissionDecision::Allowed
                } else {
                    let target_desc = if target_group.is_empty() { "Root library".to_string() } else { format!("Group '{}'", target_group) };
                    let diff = compute_line_diff("Location: Current", &format!("Location: {}", target_desc));
                    let action_id = format!("act_move_{}", Utc::now().timestamp_millis());
                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: TOOL_MOVE_NOTE.to_string(),
                        filename: filename.to_string(),
                        reason,
                        new_content: target_group.to_string(),
                        diff,
                    })
                }
            }
            TOOL_CREATE_NOTE => {
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
                        tool_name: TOOL_CREATE_NOTE.to_string(),
                        filename: format!("{}.adoc", title.to_lowercase().replace(' ', "_")),
                        reason: format!("Create new note '{}'", title),
                        new_content: content.to_string(),
                        diff,
                    })
                }
            }
            TOOL_EDIT_NOTE => {
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
                        tool_name: TOOL_EDIT_NOTE.to_string(),
                        filename: filename.to_string(),
                        reason: reason.to_string(),
                        new_content: new_content.to_string(),
                        diff,
                    })
                }
            }
            TOOL_EDIT_SECTION => {
                let filename = tool_call.function.arguments.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("unknown.adoc");
                let heading = tool_call.function.arguments.get("heading")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let content = tool_call.function.arguments.get("content")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let new_heading = tool_call.function.arguments.get("new_heading")
                    .and_then(|v| v.as_str());
                let reason = tool_call.function.arguments.get("reason")
                    .and_then(|v| v.as_str())
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| format!("Edit section '{}'", heading));

                let old_content = existing_file_content.unwrap_or("");
                let new_content = match edit_section(old_content, heading, content, new_heading) {
                    Ok(c) => c,
                    Err(e) => return PermissionDecision::Denied(e.to_string()),
                };

                let diff = compute_line_diff(old_content, &new_content);

                if !self.config.require_confirm_edit {
                    PermissionDecision::Allowed
                } else {
                    let action_id = format!("act_edit_section_{}", Utc::now().timestamp_millis());
                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: TOOL_EDIT_SECTION.to_string(),
                        filename: filename.to_string(),
                        reason,
                        new_content,
                        diff,
                    })
                }
            }
            TOOL_APPEND_TO_NOTE => {
                let filename = tool_call.function.arguments.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("unknown.adoc");
                let content = tool_call.function.arguments.get("content")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let heading = tool_call.function.arguments.get("heading")
                    .and_then(|v| v.as_str());
                let reason = tool_call.function.arguments.get("reason")
                    .and_then(|v| v.as_str())
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| {
                        if let Some(h) = heading {
                            format!("Append to section '{}'", h)
                        } else {
                            "Append to note".to_string()
                        }
                    });

                let old_content = existing_file_content.unwrap_or("");
                let new_content = match append_to_note(old_content, content, heading) {
                    Ok(c) => c,
                    Err(e) => return PermissionDecision::Denied(e.to_string()),
                };

                let diff = compute_line_diff(old_content, &new_content);

                if !self.config.require_confirm_edit {
                    PermissionDecision::Allowed
                } else {
                    let action_id = format!("act_append_{}", Utc::now().timestamp_millis());
                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: TOOL_APPEND_TO_NOTE.to_string(),
                        filename: filename.to_string(),
                        reason,
                        new_content,
                        diff,
                    })
                }
            }
            TOOL_INSERT_SECTION => {
                let filename = tool_call.function.arguments.get("filename")
                    .and_then(|v| v.as_str()).unwrap_or("unknown.adoc");
                let title = tool_call.function.arguments.get("title")
                    .and_then(|v| v.as_str()).unwrap_or("New Section");
                let level = tool_call.function.arguments.get("level")
                    .and_then(|v| v.as_u64()).map(|l| l as usize).unwrap_or(2);
                let content = tool_call.function.arguments.get("content")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let pos_str = tool_call.function.arguments.get("position")
                    .and_then(|v| v.as_str()).unwrap_or("after_heading");
                let target_heading = tool_call.function.arguments.get("target_heading")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let reason = tool_call.function.arguments.get("reason")
                    .and_then(|v| v.as_str())
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| format!("Insert section '{}'", title));

                let pos = match pos_str {
                    "before_heading" => InsertPosition::BeforeHeading(target_heading),
                    "top" => InsertPosition::Top,
                    "bottom" => InsertPosition::Bottom,
                    _ => {
                        if target_heading.is_empty() {
                            InsertPosition::Bottom
                        } else {
                            InsertPosition::AfterHeading(target_heading)
                        }
                    }
                };

                let old_content = existing_file_content.unwrap_or("");
                let new_content = match insert_section(old_content, title, level, content, pos) {
                    Ok(c) => c,
                    Err(e) => return PermissionDecision::Denied(e.to_string()),
                };

                let diff = compute_line_diff(old_content, &new_content);

                if !self.config.require_confirm_edit {
                    PermissionDecision::Allowed
                } else {
                    let action_id = format!("act_insert_{}", Utc::now().timestamp_millis());
                    PermissionDecision::RequiresConfirmation(PendingConfirmation {
                        action_id,
                        tool_call_id: tool_call.id.clone(),
                        tool_name: TOOL_INSERT_SECTION.to_string(),
                        filename: filename.to_string(),
                        reason,
                        new_content,
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

    #[test]
    fn test_require_confirmation_for_edit_section() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let edit_section_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "edit_section".to_string(),
                arguments: json!({
                    "filename": "doc.adoc",
                    "heading": "Overview",
                    "content": "Fresh overview body."
                }),
            },
        };

        let sample = "= Title\n\n== Overview\nOld text.\n\n== Next\nOther text.";
        let decision = pm.evaluate(&edit_section_call, Some(sample));
        match decision {
            PermissionDecision::RequiresConfirmation(pending) => {
                assert_eq!(pending.tool_name, "edit_section");
                assert_eq!(pending.filename, "doc.adoc");
                assert!(pending.new_content.contains("== Overview\nFresh overview body."));
                assert!(pending.new_content.contains("== Next\nOther text."));
                assert!(pending.diff.additions > 0);
            }
            other => panic!("Expected RequiresConfirmation, got {:?}", other),
        }
    }

    #[test]
    fn test_require_confirmation_for_append_to_note() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let append_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "append_to_note".to_string(),
                arguments: json!({
                    "filename": "doc.adoc",
                    "content": "* [ ] New task",
                    "heading": "Tasks"
                }),
            },
        };

        let sample = "= Title\n\n== Tasks\n* [x] Done task";
        let decision = pm.evaluate(&append_call, Some(sample));
        match decision {
            PermissionDecision::RequiresConfirmation(pending) => {
                assert_eq!(pending.tool_name, "append_to_note");
                assert_eq!(pending.filename, "doc.adoc");
                assert!(pending.new_content.contains("* [x] Done task\n* [ ] New task"));
            }
            other => panic!("Expected RequiresConfirmation, got {:?}", other),
        }
    }

    #[test]
    fn test_require_confirmation_for_insert_section() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let insert_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "insert_section".to_string(),
                arguments: json!({
                    "filename": "doc.adoc",
                    "title": "Methodology",
                    "level": 2,
                    "content": "Step 1, Step 2.",
                    "position": "before_heading",
                    "target_heading": "Results"
                }),
            },
        };

        let sample = "= Paper\n\n== Introduction\nIntro.\n\n== Results\nNumbers.";
        let decision = pm.evaluate(&insert_call, Some(sample));
        match decision {
            PermissionDecision::RequiresConfirmation(pending) => {
                assert_eq!(pending.tool_name, "insert_section");
                assert_eq!(pending.filename, "doc.adoc");
                assert!(pending.new_content.contains("== Methodology\nStep 1, Step 2."));
                assert!(pending.new_content.find("== Methodology").unwrap() < pending.new_content.find("== Results").unwrap());
            }
            other => panic!("Expected RequiresConfirmation, got {:?}", other),
        }
    }

    #[test]
    fn test_auto_allow_retrieve_context() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "retrieve_context".to_string(),
                arguments: json!({ "query": "sailfish rust" }),
            },
        };
        assert_eq!(pm.evaluate(&call, None), PermissionDecision::Allowed);
    }

    #[test]
    fn test_auto_allow_list_groups_and_confirm_move_note() {
        let pm = PermissionManager::new(PermissionConfig::default());
        let list_groups_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "list_groups".to_string(),
                arguments: json!({}),
            },
        };
        assert_eq!(pm.evaluate(&list_groups_call, None), PermissionDecision::Allowed);

        let move_call = ToolCall {
            id: None,
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "move_note".to_string(),
                arguments: json!({
                    "filename": "meeting.adoc",
                    "target_group": "Work/Projects",
                    "reason": "Organize into Work/Projects"
                }),
            },
        };
        let decision = pm.evaluate(&move_call, None);
        match decision {
            PermissionDecision::RequiresConfirmation(pending) => {
                assert_eq!(pending.tool_name, "move_note");
                assert_eq!(pending.filename, "meeting.adoc");
                assert_eq!(pending.new_content, "Work/Projects");
                assert_eq!(pending.reason, "Organize into Work/Projects");
            }
            other => panic!("Expected RequiresConfirmation for move_note, got {:?}", other),
        }
    }
}
