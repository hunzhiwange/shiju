//! 纯命令参数解析，路径约束复用网关运行库。

use super::{
    COMMANDS,
    CommandRoute,
    CommandSpec,
    IntegrationKind,
    PermissionMode,
    ReasoningEffort,
    SettingsTarget,
    TaskControlAction,
    normalize_workspace_relative_path,
};

pub(super) fn route_command_id(item_id: &str) -> Result<CommandRoute, String> {
    if let Some(id) = item_id.strip_prefix("integration:skill:") {
        return stable_integration_id(IntegrationKind::Skill, id, "").map(
            |(kind, id, arguments)| CommandRoute::ExpandIntegrationPrompt { kind, id, arguments },
        );
    }
    COMMANDS
        .iter()
        .copied()
        .find(|command| command.id == item_id)
        .map(CommandSpec::route)
        .ok_or_else(|| format!("unknown TUI command: /{item_id}"))
}

pub(super) fn route_slash_command(command_id: &str, arguments: &str) -> Result<CommandRoute, String> {
    if let Some(id) = command_id.strip_prefix("skill:") {
        return stable_integration_id(IntegrationKind::Skill, id, arguments).map(
            |(kind, id, arguments)| CommandRoute::ExpandIntegrationPrompt { kind, id, arguments },
        );
    }
    if let Some(uuid) = command_id.strip_prefix("workflow:") {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err("TUI Workflow command requires a non-empty UUID".to_string());
        }
        if arguments.is_empty() {
            return Err("TUI Workflow command requires a non-empty query".to_string());
        }
        return Ok(CommandRoute::StartWorkflowRun {
            uuid: uuid.to_string(),
            query: arguments.to_string(),
        });
    }
    match command_id {
        "memory" => route_memory(arguments),
        "files" => Ok(CommandRoute::OpenWorkspaceFiles {
            path: normalize_workspace_relative_path(arguments)?,
        }),
        "hooks" => match arguments {
            "" => Ok(CommandRoute::OpenHookCatalog),
            "audit" => Ok(CommandRoute::ShowHookAudit),
            _ => {
                let mut parts = arguments.split_whitespace();
                let action = parts.next().unwrap_or_default();
                let plugin_id = parts.next().unwrap_or_default();
                let hook_hash = parts.next().unwrap_or_default();
                let trusted = match action {
                    "trust" => true,
                    "revoke" => false,
                    _ => {
                        return Err(
                            "/hooks expects trust|revoke <plugin-id> <hook-hash>".to_string()
                        );
                    }
                };
                if plugin_id.is_empty() || hook_hash.is_empty() || parts.next().is_some() {
                    return Err("/hooks expects trust|revoke <plugin-id> <hook-hash>".to_string());
                }
                Ok(CommandRoute::ConfirmHookMutation {
                    plugin_id: plugin_id.to_string(),
                    hook_hash: hook_hash.to_string(),
                    trusted,
                })
            }
        },
        "cd" => {
            if arguments.is_empty() {
                return Err("TUI command /cd requires a non-empty path".to_string());
            }
            Ok(CommandRoute::ChangeDirectory { path: arguments.to_string() })
        }
        "tasks" if !arguments.is_empty() => {
            let mut parts = arguments.split_whitespace();
            let action = parts.next().unwrap_or_default();
            let id = parts.next().unwrap_or_default();
            if id.is_empty() || parts.next().is_some() {
                return Err(
                    "/tasks expects <start|retry|pause|resume|cancel|archive> <owned-task-id>"
                        .to_string(),
                );
            }
            let action = match action {
                "start" => TaskControlAction::Start,
                "retry" => TaskControlAction::Retry,
                "pause" => TaskControlAction::Pause,
                "resume" => TaskControlAction::Resume,
                "cancel" => TaskControlAction::Cancel,
                "archive" => TaskControlAction::Archive,
                _ => {
                    return Err("/tasks expects start, retry, pause, resume, cancel, or archive"
                        .to_string());
                }
            };
            Ok(CommandRoute::ConfirmTaskControl { id: id.to_string(), action })
        }
        "schedule" if !arguments.is_empty() => route_schedule_management(arguments),
        "skills" | "plugins" if !arguments.is_empty() => {
            route_integration_management(command_id, arguments)
        }
        "model" if !arguments.is_empty() => {
            Ok(CommandRoute::ApplyModel(required_setting_value("model", arguments)?))
        }
        "effort" if !arguments.is_empty() => parse_effort(arguments).map(CommandRoute::ApplyEffort),
        "plan" if !arguments.is_empty() => match arguments {
            "on" | "plan" => Ok(CommandRoute::ApplyPermissionMode(PermissionMode::Plan)),
            "off" | "default" => Ok(CommandRoute::ApplyPermissionMode(PermissionMode::Default)),
            query => Ok(CommandRoute::StartPlanRun { query: query.to_string() }),
        },
        "goal" if !arguments.is_empty() => {
            Ok(CommandRoute::StartGoalRun { query: arguments.to_string() })
        }
        "permissions" if !arguments.is_empty() => {
            parse_permission_mode(arguments).map(CommandRoute::ApplyPermissionMode)
        }
        "rename" => {
            if arguments.is_empty() {
                return Err("TUI command /rename requires a non-empty title".to_string());
            }
            Ok(CommandRoute::RenameSession { title: Some(arguments.to_string()) })
        }
        "fork" => Ok(CommandRoute::ForkSession {
            message_id: optional_message_id_argument(command_id, arguments)?,
        }),
        "reset" | "rewind" => Ok(CommandRoute::ResetSession {
            message_id: Some(required_message_id_argument(command_id, arguments)?),
        }),
        "compact" | "summarize" => Ok(CommandRoute::CompactSession {
            message_id: Some(required_message_id_argument(command_id, arguments)?),
        }),
        "archive" => {
            reject_arguments(command_id, arguments)?;
            Ok(CommandRoute::ArchiveSession)
        }
        _ => {
            reject_arguments(command_id, arguments)?;
            route_command_id(command_id)
        }
    }
}

pub(super) fn route_memory(arguments: &str) -> Result<CommandRoute, String> {
    if arguments.is_empty() {
        return Ok(CommandRoute::QueryMemory { query: None });
    }
    let action_end = arguments.find(char::is_whitespace).unwrap_or(arguments.len());
    let action = &arguments[..action_end];
    let remainder = arguments[action_end..].trim();
    match action {
        "remember" => {
            let key_end = remainder.find(char::is_whitespace).unwrap_or(remainder.len());
            let key = &remainder[..key_end];
            let content = remainder[key_end..].trim();
            if key.is_empty() || content.is_empty() {
                return Err("/memory remember expects <key> <content>".to_string());
            }
            Ok(CommandRoute::ConfirmMemoryStore {
                key: key.to_string(),
                content: content.to_string(),
            })
        }
        "forget" => {
            let mut keys = remainder.split_whitespace();
            let key = keys.next().unwrap_or_default();
            if key.is_empty() || keys.next().is_some() {
                return Err("/memory forget expects <key>".to_string());
            }
            Ok(CommandRoute::ConfirmMemoryDelete { key: key.to_string() })
        }
        _ => Ok(CommandRoute::QueryMemory { query: Some(arguments.to_string()) }),
    }
}

pub(super) fn route_schedule_management(arguments: &str) -> Result<CommandRoute, String> {
    let mut parts = arguments.split_whitespace();
    let action = parts.next().unwrap_or_default();
    let job_id = parts.next().unwrap_or_default();
    if job_id.is_empty() || parts.next().is_some() {
        return Err("/schedule expects <enable|disable|delete> <owned-job-id>".to_string());
    }
    match action {
        "enable" => {
            Ok(CommandRoute::ConfirmCronToggle { job_id: job_id.to_string(), enabled: true })
        }
        "disable" => {
            Ok(CommandRoute::ConfirmCronToggle { job_id: job_id.to_string(), enabled: false })
        }
        "delete" => Ok(CommandRoute::ConfirmCronDelete { job_id: job_id.to_string() }),
        _ => Err("/schedule expects enable, disable, or delete".to_string()),
    }
}

pub(super) fn route_workspace_file_selection(item_id: &str) -> Result<CommandRoute, String> {
    let (kind, path) = item_id
        .split_once(':')
        .ok_or_else(|| "workspace file selection requires a stable item id".to_string())?;
    let path = normalize_workspace_relative_path(path)?;
    match kind {
        "directory" => Ok(CommandRoute::OpenWorkspaceFiles { path }),
        "file" if path != "." => Ok(CommandRoute::ReadWorkspaceFile { path }),
        "file" => Err("workspace file selection requires a file path".to_string()),
        _ => Err(format!("unknown workspace file selection kind: {kind}")),
    }
}

pub(super) fn route_integration_management(command_id: &str, arguments: &str) -> Result<CommandRoute, String> {
    let mut parts = arguments.split_whitespace();
    let action = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default();
    if parts.next().is_some() || id.is_empty() {
        return Err(format!("/{command_id} management expects <enable|disable|delete> <id>"));
    }
    let kind =
        if command_id == "skills" { IntegrationKind::Skill } else { IntegrationKind::Plugin };
    match action {
        "enable" => {
            Ok(CommandRoute::ConfirmIntegrationToggle { kind, id: id.to_string(), enabled: true })
        }
        "disable" => {
            Ok(CommandRoute::ConfirmIntegrationToggle { kind, id: id.to_string(), enabled: false })
        }
        "delete" => Ok(CommandRoute::ConfirmIntegrationDelete { kind, id: id.to_string() }),
        _ => Err(format!("/{command_id} management expects enable, disable, or delete")),
    }
}

pub(super) fn stable_integration_id(
    kind: IntegrationKind,
    id: &str,
    arguments: &str,
) -> Result<(IntegrationKind, String, String), String> {
    let id = id.trim();
    if id.is_empty() || id.contains(char::is_whitespace) {
        return Err("integration command requires a non-empty stable id".to_string());
    }
    Ok((kind, id.to_string(), arguments.trim().to_string()))
}

pub(super) fn route_settings_target(item_id: &str) -> Result<CommandRoute, String> {
    let target = match item_id {
        "model" => SettingsTarget::Model,
        "effort" => SettingsTarget::Effort,
        "permissions" => SettingsTarget::Permissions,
        "acp" => SettingsTarget::AcpBackend,
        "delegate" => SettingsTarget::DelegateAgent,
        "full_access" => SettingsTarget::FullAccess,
        "subagents" => SettingsTarget::SubagentMode,
        _ => return Err(format!("unknown session setting: {item_id}")),
    };
    Ok(CommandRoute::OpenSettingsTarget(target))
}

pub(super) fn parse_effort(value: &str) -> Result<ReasoningEffort, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(ReasoningEffort::Auto),
        "none" => Ok(ReasoningEffort::None),
        "minimal" => Ok(ReasoningEffort::Minimal),
        "max" => Ok(ReasoningEffort::Max),
        "low" => Ok(ReasoningEffort::Low),
        "medium" => Ok(ReasoningEffort::Medium),
        "high" => Ok(ReasoningEffort::High),
        "xhigh" => Ok(ReasoningEffort::XHigh),
        _ => Err(format!("unknown reasoning effort: {value}")),
    }
}

pub(super) fn parse_permission_mode(value: &str) -> Result<PermissionMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "default" => Ok(PermissionMode::Default),
        "plan" => Ok(PermissionMode::Plan),
        "bypass" => Ok(PermissionMode::Bypass),
        _ => Err(format!("unknown permission mode: {value}")),
    }
}

pub(super) fn required_setting_value(name: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{name} setting requires a non-empty stable id"))
    } else {
        Ok(value.to_string())
    }
}

pub(super) fn optional_setting_value(value: &str, default_id: &str) -> Result<Option<String>, String> {
    let value = required_setting_value("session", value)?;
    Ok((value != default_id).then_some(value))
}

pub(super) fn stable_message_route(
    item_id: &str,
    route: impl FnOnce(String) -> CommandRoute,
) -> Result<CommandRoute, String> {
    let message_id = item_id.trim();
    if message_id.is_empty() {
        Err("session mutation selection requires a non-empty stable message id".to_string())
    } else {
        Ok(route(message_id.to_string()))
    }
}

pub(super) fn required_message_id_argument(command_id: &str, arguments: &str) -> Result<String, String> {
    optional_message_id_argument(command_id, arguments)?
        .ok_or_else(|| format!("TUI command /{command_id} requires one persisted message id"))
}

pub(super) fn optional_message_id_argument(
    command_id: &str,
    arguments: &str,
) -> Result<Option<String>, String> {
    if arguments.is_empty() {
        return Ok(None);
    }
    let mut parts = arguments.split_whitespace();
    let message_id = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        return Err(format!("TUI command /{command_id} accepts at most one message id"));
    }
    Ok(Some(message_id.to_string()))
}

pub(super) fn reject_arguments(command_id: &str, arguments: &str) -> Result<(), String> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(format!("TUI command /{command_id} does not accept arguments"))
    }
}
