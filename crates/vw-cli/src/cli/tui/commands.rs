//! 活动 TUI 的稳定命令目录与纯路由结果。
//!
//! 本模块只描述命令及其当前可用性，不持有 Gateway client，也不执行副作用。

use vw_shared::session::ui_types::ChatSessionMeta;
use vw_tui::V3SelectionItem;

use super::model::integration::{IntegrationItem, IntegrationKind};
use super::model::background_task::TaskControlAction;
use super::model::{PermissionMode, ReasoningEffort};
use super::runtime::workspace_files::normalize_workspace_relative_path;

pub(crate) const COMMAND_PALETTE_SURFACE: &str = "command_palette";
pub(crate) const SESSION_RESUME_SURFACE: &str = "session_resume";
pub(crate) const SESSION_RESET_SURFACE: &str = "session_reset";
pub(crate) const SESSION_COMPACT_SURFACE: &str = "session_compact";
pub(crate) const SETTINGS_SURFACE: &str = "session_settings";
pub(crate) const SETTINGS_MODEL_SURFACE: &str = "session_settings_model";
pub(crate) const SETTINGS_EFFORT_SURFACE: &str = "session_settings_effort";
pub(crate) const SETTINGS_PERMISSION_SURFACE: &str = "session_settings_permissions";
pub(crate) const SETTINGS_PLAN_SURFACE: &str = "session_settings_plan";
pub(crate) const SETTINGS_ACP_SURFACE: &str = "session_settings_acp";
pub(crate) const SETTINGS_DELEGATE_SURFACE: &str = "session_settings_delegate";
pub(crate) const SETTINGS_ACCESS_SURFACE: &str = "session_settings_access";
pub(crate) const SETTINGS_SUBAGENT_SURFACE: &str = "session_settings_subagent";
pub(crate) const WORKFLOW_SELECTION_SURFACE: &str = "orchestration_workflows";
pub(crate) const SCHEDULE_SELECTION_SURFACE: &str = "orchestration_schedules";
pub(crate) const TASK_SELECTION_SURFACE: &str = "orchestration_tasks";
pub(crate) const WORKSPACE_FILES_SURFACE: &str = "workspace_files";
pub(crate) const HOOK_CATALOG_SURFACE: &str = "hook_catalog";

const LATER_BATCH_REASON: &str = "该能力将在后续桌面能力对齐批次接通";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CommandRoute {
    Help,
    Status,
    RunDoctor,
    ShowDiff,
    OpenWorkspaceFiles { path: String },
    ReadWorkspaceFile { path: String },
    OpenHookCatalog,
    ShowHookAudit,
    ConfirmHookMutation {
        plugin_id: String,
        hook_hash: String,
        trusted: bool,
    },
    ChangeDirectory { path: String },
    Exit,
    OpenSessionPicker,
    ContinueSession,
    NewSession,
    ResumeSession { session_id: String },
    RenameSession { title: Option<String> },
    ForkSession { message_id: Option<String> },
    ResetSession { message_id: Option<String> },
    CompactSession { message_id: Option<String> },
    ArchiveSession,
    OpenSettings,
    OpenModelPicker,
    OpenEffortPicker,
    OpenPermissionPicker { plan_only: bool },
    OpenSettingsTarget(SettingsTarget),
    ApplyModel(String),
    ApplyEffort(ReasoningEffort),
    ApplyPermissionMode(PermissionMode),
    ApplyAcpBackend(Option<String>),
    ApplyDelegateAgent(Option<String>),
    ApplyFullAccess(bool),
    ApplySubagentMode(bool),
    QueryMemory { query: Option<String> },
    ConfirmMemoryStore { key: String, content: String },
    ConfirmMemoryDelete { key: String },
    OpenIntegrations(IntegrationKind),
    ExpandIntegrationPrompt { kind: IntegrationKind, id: String, arguments: String },
    ConfirmIntegrationToggle { kind: IntegrationKind, id: String, enabled: bool },
    ConfirmIntegrationDelete { kind: IntegrationKind, id: String },
    OpenOrchestration(OrchestrationPanelKind),
    OpenWorkflowDetail { uuid: String },
    OpenCronDetail { job_id: String },
    OpenTaskDetail { task_id: String },
    ConfirmCronToggle { job_id: String, enabled: bool },
    ConfirmCronDelete { job_id: String },
    StartWorkflowRun { uuid: String, query: String },
    ConfirmTaskControl { id: String, action: TaskControlAction },
    StartPlanRun { query: String },
    StartGoalRun { query: String },
    Unavailable { command_id: String, reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SettingsTarget {
    Model,
    Effort,
    Permissions,
    AcpBackend,
    DelegateAgent,
    FullAccess,
    SubagentMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OrchestrationPanelKind {
    Workflows,
    Schedules,
    Agents,
    Tasks,
}

#[derive(Clone, Copy)]
struct CommandSpec {
    id: &'static str,
    description: &'static str,
    route: Option<fn() -> CommandRoute>,
}

impl CommandSpec {
    const fn local(
        id: &'static str,
        description: &'static str,
        route: fn() -> CommandRoute,
    ) -> Self {
        Self { id, description, route: Some(route) }
    }

    fn selection_item(self) -> V3SelectionItem {
        let disabled_reason = self.route.is_none().then(|| LATER_BATCH_REASON.to_string());
        V3SelectionItem {
            id: self.id.to_string(),
            label: format!("/{}", self.id),
            description: Some(self.description.to_string()),
            search_value: Some(format!("{} {}", self.id, self.description)),
            disabled: disabled_reason.is_some(),
            disabled_reason,
            ..V3SelectionItem::default()
        }
    }

    fn route(self) -> CommandRoute {
        self.route.map_or_else(
            || CommandRoute::Unavailable {
                command_id: self.id.to_string(),
                reason: LATER_BATCH_REASON.to_string(),
            },
            |route| route(),
        )
    }
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec::local("help", "显示当前命令及可用状态", || CommandRoute::Help),
    CommandSpec::local("status", "显示当前会话与 Gateway 状态", || CommandRoute::Status),
    CommandSpec::local("doctor", "运行 Gateway 诊断", || CommandRoute::RunDoctor),
    CommandSpec::local("exit", "退出 TUI", || CommandRoute::Exit),
    CommandSpec::local("diff", "显示当前会话文件变化", || CommandRoute::ShowDiff),
    CommandSpec::local("files", "浏览当前 Gateway 目录中的文件", || {
        CommandRoute::OpenWorkspaceFiles { path: ".".to_string() }
    }),
    CommandSpec::local("hooks", "查看 Hook 声明与安全审计", || {
        CommandRoute::OpenHookCatalog
    }),
    CommandSpec::local("cd", "切换 Gateway 工作目录", || CommandRoute::ChangeDirectory {
        path: String::new(),
    }),
    CommandSpec::local("resume", "选择并恢复会话", || CommandRoute::OpenSessionPicker),
    CommandSpec::local("continue", "恢复最近的非归档会话", || {
        CommandRoute::ContinueSession
    }),
    CommandSpec::local("new", "创建新会话", || CommandRoute::NewSession),
    CommandSpec::local("rename", "重命名当前会话", || CommandRoute::RenameSession {
        title: None,
    }),
    CommandSpec::local("fork", "从当前会话创建分支", || CommandRoute::ForkSession {
        message_id: None,
    }),
    CommandSpec::local("reset", "将当前会话回退到持久化消息（别名 /rewind）", || {
        CommandRoute::ResetSession { message_id: None }
    }),
    CommandSpec::local("compact", "压缩当前会话上下文（别名 /summarize）", || {
        CommandRoute::CompactSession { message_id: None }
    }),
    CommandSpec::local("archive", "归档当前会话", || CommandRoute::ArchiveSession),
    CommandSpec::local("model", "选择当前会话模型", || CommandRoute::OpenModelPicker),
    CommandSpec::local("effort", "选择推理强度", || CommandRoute::OpenEffortPicker),
    CommandSpec::local("plan", "配置计划模式", || {
        CommandRoute::OpenPermissionPicker { plan_only: true }
    }),
    CommandSpec::local("permissions", "配置当前会话权限", || {
        CommandRoute::OpenPermissionPicker { plan_only: false }
    }),
    CommandSpec::local("settings", "打开会话设置", || CommandRoute::OpenSettings),
    CommandSpec::local("skills", "浏览可用技能", || {
        CommandRoute::OpenIntegrations(IntegrationKind::Skill)
    }),
    CommandSpec::local("plugins", "浏览插件", || {
        CommandRoute::OpenIntegrations(IntegrationKind::Plugin)
    }),
    CommandSpec::local("mcp", "查看 MCP 服务", || {
        CommandRoute::OpenIntegrations(IntegrationKind::Mcp)
    }),
    CommandSpec::local("memory", "查询或刷新记忆", || {
        CommandRoute::QueryMemory { query: None }
    }),
    CommandSpec::local("workflows", "查看工作流", || {
        CommandRoute::OpenOrchestration(OrchestrationPanelKind::Workflows)
    }),
    CommandSpec::local("schedule", "查看调度", || {
        CommandRoute::OpenOrchestration(OrchestrationPanelKind::Schedules)
    }),
    CommandSpec::local("agents", "查看代理", || {
        CommandRoute::OpenOrchestration(OrchestrationPanelKind::Agents)
    }),
    CommandSpec::local("tasks", "查看后台任务", || {
        CommandRoute::OpenOrchestration(OrchestrationPanelKind::Tasks)
    }),
    CommandSpec::local("goal", "运行持续目标", || CommandRoute::StartGoalRun {
        query: String::new(),
    }),
];

pub(crate) fn command_palette_items() -> Vec<V3SelectionItem> {
    COMMANDS.iter().copied().map(CommandSpec::selection_item).collect()
}

pub(crate) fn command_palette_items_with_integrations(
    integrations: &[IntegrationItem],
) -> Vec<V3SelectionItem> {
    let mut items = command_palette_items();
    items.extend(
        integrations
            .iter()
            .filter(|item| {
                item.kind == IntegrationKind::Skill && item.enabled && item.prompt_prefix.is_some()
            })
            .map(|item| V3SelectionItem {
                id: format!("integration:{}:{}", item.kind.command_name(), item.id),
                label: format!("/skill:{}", item.id),
                description: Some(format!("{} · 来源 {}", item.title, item.source)),
                search_value: Some(format!("{} {} {}", item.id, item.title, item.source)),
                ..V3SelectionItem::default()
            }),
    );
    items
}

pub(crate) fn route_selection(surface: &str, item_id: &str) -> Result<CommandRoute, String> {
    match surface {
        COMMAND_PALETTE_SURFACE => route_command_id(item_id),
        SESSION_RESUME_SURFACE => {
            let session_id = item_id.trim();
            if session_id.is_empty() {
                Err("session selection requires a non-empty stable id".to_string())
            } else {
                Ok(CommandRoute::ResumeSession { session_id: session_id.to_string() })
            }
        }
        SESSION_RESET_SURFACE => stable_message_route(item_id, |message_id| {
            CommandRoute::ResetSession { message_id: Some(message_id) }
        }),
        SESSION_COMPACT_SURFACE => stable_message_route(item_id, |message_id| {
            CommandRoute::CompactSession { message_id: Some(message_id) }
        }),
        SETTINGS_SURFACE => route_settings_target(item_id),
        SETTINGS_MODEL_SURFACE => required_setting_value("model", item_id)
            .map(CommandRoute::ApplyModel),
        SETTINGS_EFFORT_SURFACE => parse_effort(item_id).map(CommandRoute::ApplyEffort),
        SETTINGS_PERMISSION_SURFACE | SETTINGS_PLAN_SURFACE => {
            parse_permission_mode(item_id).map(CommandRoute::ApplyPermissionMode)
        }
        SETTINGS_ACP_SURFACE => Ok(CommandRoute::ApplyAcpBackend(optional_setting_value(
            item_id,
            "default",
        )?)),
        SETTINGS_DELEGATE_SURFACE => Ok(CommandRoute::ApplyDelegateAgent(optional_setting_value(
            item_id, "main",
        )?)),
        SETTINGS_ACCESS_SURFACE => match item_id {
            "standard" => Ok(CommandRoute::ApplyFullAccess(false)),
            "full" => Ok(CommandRoute::ApplyFullAccess(true)),
            _ => Err(format!("unknown full-access setting: {item_id}")),
        },
        SETTINGS_SUBAGENT_SURFACE => match item_id {
            "disabled" => Ok(CommandRoute::ApplySubagentMode(false)),
            "enabled" => Ok(CommandRoute::ApplySubagentMode(true)),
            _ => Err(format!("unknown subagent setting: {item_id}")),
        },
        WORKFLOW_SELECTION_SURFACE => {
            let uuid = item_id.trim();
            if uuid.is_empty() {
                Err("workflow selection requires a non-empty stable UUID".to_string())
            } else {
                Ok(CommandRoute::OpenWorkflowDetail { uuid: uuid.to_string() })
            }
        }
        SCHEDULE_SELECTION_SURFACE => {
            let job_id = item_id.trim();
            if job_id.is_empty() {
                Err("schedule selection requires a non-empty stable job ID".to_string())
            } else {
                Ok(CommandRoute::OpenCronDetail { job_id: job_id.to_string() })
            }
        }
        TASK_SELECTION_SURFACE => {
            let task_id = item_id.trim();
            if task_id.is_empty() {
                Err("task selection requires a non-empty stable task ID".to_string())
            } else {
                Ok(CommandRoute::OpenTaskDetail { task_id: task_id.to_string() })
            }
        }
        WORKSPACE_FILES_SURFACE => route_workspace_file_selection(item_id),
        _ => Err(format!("unsupported TUI selection surface: {surface}")),
    }
}

/// 将 Gateway 会话摘要转为可搜索、只携带稳定 ID 的选择项。
pub(crate) fn session_resume_items(previews: &[ChatSessionMeta]) -> Vec<V3SelectionItem> {
    previews
        .iter()
        .map(|preview| {
            let title = preview.title.trim();
            let label = if title.is_empty() { preview.id.clone() } else { title.to_string() };
            let last_content = preview.last_content.as_deref().unwrap_or_default().trim();
            let description = if last_content.is_empty() {
                format!("{} 条消息", preview.message_count)
            } else {
                format!("{} 条消息 · {last_content}", preview.message_count)
            };
            V3SelectionItem {
                id: preview.id.clone(),
                label,
                description: Some(description),
                search_value: Some(format!(
                    "{} {} {}",
                    preview.id,
                    preview.title,
                    preview.last_content.as_deref().unwrap_or_default()
                )),
                ..V3SelectionItem::default()
            }
        })
        .collect()
}

pub(crate) fn route_slash_submission(text: &str) -> Result<Option<CommandRoute>, String> {
    let Some(command_text) = text.strip_prefix('/') else {
        return Ok(None);
    };
    let command_end = command_text.find(char::is_whitespace).unwrap_or(command_text.len());
    let command_id = &command_text[..command_end];
    if command_id.is_empty() {
        return Err("empty TUI slash command".to_string());
    }
    let arguments = command_text[command_end..].trim();
    route_slash_command(command_id, arguments).map(Some)
}

pub(crate) fn help_text() -> String {
    COMMANDS
        .iter()
        .map(|command| {
            let suffix = if command.route.is_none() { " (暂不可用)" } else { "" };
            format!("/{} — {}{suffix}", command.id, command.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn route_command_id(item_id: &str) -> Result<CommandRoute, String> {
    if let Some(id) = item_id.strip_prefix("integration:skill:") {
        return stable_integration_id(IntegrationKind::Skill, id, "")
            .map(|(kind, id, arguments)| CommandRoute::ExpandIntegrationPrompt {
                kind,
                id,
                arguments,
            });
    }
    COMMANDS
        .iter()
        .copied()
        .find(|command| command.id == item_id)
        .map(CommandSpec::route)
        .ok_or_else(|| format!("unknown TUI command: /{item_id}"))
}

fn route_slash_command(command_id: &str, arguments: &str) -> Result<CommandRoute, String> {
    if let Some(id) = command_id.strip_prefix("skill:") {
        return stable_integration_id(IntegrationKind::Skill, id, arguments)
            .map(|(kind, id, arguments)| CommandRoute::ExpandIntegrationPrompt {
                kind,
                id,
                arguments,
            });
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
                    return Err(
                        "/hooks expects trust|revoke <plugin-id> <hook-hash>".to_string()
                    );
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
                    return Err(
                        "/tasks expects start, retry, pause, resume, cancel, or archive"
                            .to_string(),
                    );
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
        "effort" if !arguments.is_empty() => {
            parse_effort(arguments).map(CommandRoute::ApplyEffort)
        }
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

fn route_memory(arguments: &str) -> Result<CommandRoute, String> {
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

fn route_schedule_management(arguments: &str) -> Result<CommandRoute, String> {
    let mut parts = arguments.split_whitespace();
    let action = parts.next().unwrap_or_default();
    let job_id = parts.next().unwrap_or_default();
    if job_id.is_empty() || parts.next().is_some() {
        return Err("/schedule expects <enable|disable|delete> <owned-job-id>".to_string());
    }
    match action {
        "enable" => Ok(CommandRoute::ConfirmCronToggle {
            job_id: job_id.to_string(),
            enabled: true,
        }),
        "disable" => Ok(CommandRoute::ConfirmCronToggle {
            job_id: job_id.to_string(),
            enabled: false,
        }),
        "delete" => Ok(CommandRoute::ConfirmCronDelete { job_id: job_id.to_string() }),
        _ => Err("/schedule expects enable, disable, or delete".to_string()),
    }
}

fn route_workspace_file_selection(item_id: &str) -> Result<CommandRoute, String> {
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

fn route_integration_management(
    command_id: &str,
    arguments: &str,
) -> Result<CommandRoute, String> {
    let mut parts = arguments.split_whitespace();
    let action = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default();
    if parts.next().is_some() || id.is_empty() {
        return Err(format!("/{command_id} management expects <enable|disable|delete> <id>"));
    }
    let kind = if command_id == "skills" {
        IntegrationKind::Skill
    } else {
        IntegrationKind::Plugin
    };
    match action {
        "enable" => Ok(CommandRoute::ConfirmIntegrationToggle {
            kind,
            id: id.to_string(),
            enabled: true,
        }),
        "disable" => Ok(CommandRoute::ConfirmIntegrationToggle {
            kind,
            id: id.to_string(),
            enabled: false,
        }),
        "delete" => Ok(CommandRoute::ConfirmIntegrationDelete { kind, id: id.to_string() }),
        _ => Err(format!("/{command_id} management expects enable, disable, or delete")),
    }
}

fn stable_integration_id(
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

fn route_settings_target(item_id: &str) -> Result<CommandRoute, String> {
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

fn parse_effort(value: &str) -> Result<ReasoningEffort, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "minimal" | "none" => Ok(ReasoningEffort::Minimal),
        "low" => Ok(ReasoningEffort::Low),
        "medium" => Ok(ReasoningEffort::Medium),
        "high" => Ok(ReasoningEffort::High),
        "xhigh" => Ok(ReasoningEffort::XHigh),
        _ => Err(format!("unknown reasoning effort: {value}")),
    }
}

fn parse_permission_mode(value: &str) -> Result<PermissionMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "default" => Ok(PermissionMode::Default),
        "plan" => Ok(PermissionMode::Plan),
        "bypass" => Ok(PermissionMode::Bypass),
        _ => Err(format!("unknown permission mode: {value}")),
    }
}

fn required_setting_value(name: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{name} setting requires a non-empty stable id"))
    } else {
        Ok(value.to_string())
    }
}

fn optional_setting_value(
    value: &str,
    default_id: &str,
) -> Result<Option<String>, String> {
    let value = required_setting_value("session", value)?;
    Ok((value != default_id).then_some(value))
}

fn stable_message_route(
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

fn required_message_id_argument(command_id: &str, arguments: &str) -> Result<String, String> {
    optional_message_id_argument(command_id, arguments)?.ok_or_else(|| {
        format!("TUI command /{command_id} requires one persisted message id")
    })
}

fn optional_message_id_argument(
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

fn reject_arguments(command_id: &str, arguments: &str) -> Result<(), String> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(format!("TUI command /{command_id} does not accept arguments"))
    }
}
