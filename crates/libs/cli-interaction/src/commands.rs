//! 活动 TUI 的稳定命令目录与纯路由结果。
//!
//! 本模块只描述命令及其当前可用性，不持有 Gateway client，也不执行副作用。

use session_ui_types::ChatSessionMeta;
use vw_tui::V3SelectionItem;

use cli_interaction_model::background_task::TaskControlAction;
use cli_interaction_model::integration::{IntegrationItem, IntegrationKind};
use cli_interaction_model::{PermissionMode, ReasoningEffort};
use cli_gateway_workspace::workspace_files::normalize_workspace_relative_path;

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
    ExportSession { path: String },
    CopyAnswer { code_block: Option<usize> },
    RunDoctor,
    ShowDiff,
    OpenWorkspaceFiles { path: String },
    ReadWorkspaceFile { path: String },
    OpenHookCatalog,
    ShowHookAudit,
    ConfirmHookMutation { plugin_id: String, hook_hash: String, trusted: bool },
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
    CommandSpec::local(
        "copy",
        "复制最近完成回答；/copy code <序号> 复制代码块（从1开始）",
        || CommandRoute::CopyAnswer { code_block: None },
    ),
    CommandSpec::local("export", "导出已加载会话：/export <工作区相对路径>（不覆盖）", || CommandRoute::ExportSession { path: String::new() }),
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
    CommandSpec::local(
        "reset",
        "将当前会话回退到持久化消息（别名 /rewind）",
        || CommandRoute::ResetSession { message_id: None },
    ),
    CommandSpec::local("compact", "压缩当前会话上下文（别名 /summarize）", || {
        CommandRoute::CompactSession { message_id: None }
    }),
    CommandSpec::local("archive", "归档当前会话", || CommandRoute::ArchiveSession),
    CommandSpec::local("model", "选择当前会话模型", || CommandRoute::OpenModelPicker),
    CommandSpec::local("effort", "选择推理强度", || CommandRoute::OpenEffortPicker),
    CommandSpec::local("plan", "配置计划模式", || CommandRoute::OpenPermissionPicker {
        plan_only: true,
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
    CommandSpec::local("memory", "查询或刷新记忆", || CommandRoute::QueryMemory {
        query: None,
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
        "session_resume_retry" if item_id == "retry" => Ok(CommandRoute::OpenSessionPicker),
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
        SETTINGS_MODEL_SURFACE => {
            required_setting_value("model", item_id).map(CommandRoute::ApplyModel)
        }
        SETTINGS_EFFORT_SURFACE => parse_effort(item_id).map(CommandRoute::ApplyEffort),
        SETTINGS_PERMISSION_SURFACE | SETTINGS_PLAN_SURFACE => {
            parse_permission_mode(item_id).map(CommandRoute::ApplyPermissionMode)
        }
        SETTINGS_ACP_SURFACE => {
            Ok(CommandRoute::ApplyAcpBackend(optional_setting_value(item_id, "default")?))
        }
        SETTINGS_DELEGATE_SURFACE => {
            Ok(CommandRoute::ApplyDelegateAgent(optional_setting_value(item_id, "main")?))
        }
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
            let last_content = preview.last_content.as_deref().unwrap_or_default().trim();
            let label = if !title.is_empty() {
                title.to_string()
            } else if let Some(line) =
                session_text::initial_title_from_content(last_content)
            {
                line.to_owned()
            } else {
                preview.id.clone()
            };
            let description = if last_content.is_empty() || label == last_content {
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

use routes::route_command_id;
use routes::route_slash_command;
use routes::route_workspace_file_selection;
use routes::route_settings_target;
use routes::parse_effort;
use routes::parse_permission_mode;
use routes::required_setting_value;
use routes::optional_setting_value;
use routes::stable_message_route;

mod routes;
