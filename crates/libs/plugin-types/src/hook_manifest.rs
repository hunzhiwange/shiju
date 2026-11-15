//! Codex hook declarations. Parsing never grants trust or executes a handler.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CodexHookEvent {
    PreModelRequest,
    PostModelRequest,
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    SubagentStart,
    SubagentStop,
    Stop,
    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexHookMatcherGroup {
    #[serde(default)]
    pub matcher: Option<String>,
    #[serde(default)]
    pub hooks: Vec<CodexHookHandler>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CodexHookHandler {
    #[serde(rename = "command")]
    Command {
        command: String,
        #[serde(default, rename = "commandWindows", alias = "command_windows")]
        command_windows: Option<String>,
        #[serde(default)]
        timeout: Option<u64>,
        #[serde(default)]
        r#async: bool,
        #[serde(default, rename = "statusMessage")]
        status_message: Option<String>,
        #[serde(default, rename = "additionalContextLimit")]
        additional_context_limit: Option<usize>,
    },
    #[serde(rename = "mcp_tool")]
    McpTool {
        server: String,
        tool: String,
        #[serde(default, deserialize_with = "deserialize_mcp_input")]
        input: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        timeout: Option<u64>,
        #[serde(default, rename = "statusMessage")]
        status_message: Option<String>,
    },
    #[serde(rename = "prompt")]
    Prompt {},
    #[serde(rename = "agent")]
    Agent {},
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexHooksManifest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hooks: BTreeMap<CodexHookEvent, Vec<CodexHookMatcherGroup>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHookDiagnostic {
    pub event: CodexHookEvent,
    pub group_index: usize,
    pub handler_index: usize,
    pub message: String,
}

impl CodexHooksManifest {
    /// Keep load failures alongside declarations rather than silently losing handlers.
    pub fn diagnostics(&self) -> Vec<CodexHookDiagnostic> {
        let mut diagnostics = Vec::new();
        for (&event, groups) in &self.hooks {
            for (group_index, group) in groups.iter().enumerate() {
                for (handler_index, handler) in group.hooks.iter().enumerate() {
                    let message = match handler {
                        CodexHookHandler::Prompt {} => {
                            Some("prompt hook handlers are not supported")
                        }
                        CodexHookHandler::Agent {} => Some("agent hook handlers are not supported"),
                        CodexHookHandler::McpTool { .. } if event == CodexHookEvent::SessionEnd => {
                            Some("SessionEnd MCP hooks are not supported")
                        }
                        CodexHookHandler::McpTool { server, tool, .. }
                            if server.trim().is_empty() || tool.trim().is_empty() =>
                        {
                            Some("MCP hook server and tool must not be empty")
                        }
                        CodexHookHandler::Command { command, command_windows, .. } => {
                            let command = if cfg!(windows) {
                                command_windows.as_deref().unwrap_or(command)
                            } else {
                                command
                            };
                            command.trim().is_empty().then_some("hook command must not be empty")
                        }
                        _ => None,
                    };
                    if let Some(message) = message {
                        diagnostics.push(CodexHookDiagnostic {
                            event,
                            group_index,
                            handler_index,
                            message: message.to_owned(),
                        });
                    }
                }
            }
        }
        diagnostics
    }
}

fn deserialize_mcp_input<'de, D>(
    deserializer: D,
) -> Result<serde_json::Map<String, serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    fn representable(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Null => false,
            serde_json::Value::Number(n) => !n.is_u64() || n.as_i64().is_some(),
            serde_json::Value::Array(items) => items.iter().all(representable),
            serde_json::Value::Object(items) => items.values().all(representable),
            serde_json::Value::Bool(_) | serde_json::Value::String(_) => true,
        }
    }
    let input = serde_json::Map::deserialize(deserializer)?;
    if !input.values().all(representable) {
        return Err(serde::de::Error::custom("MCP hook input must be representable as TOML"));
    }
    Ok(input)
}
