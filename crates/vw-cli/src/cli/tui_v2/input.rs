//! tui_v2 输入层的 slash command 与 prompt suggestion 逻辑。
//!
//! 本模块只负责两类纯输入协议：
//! - `/...` 命令的解析、建议与执行
//! - prompt footer 可直接消费的轻量 suggestion 列表
//!
//! 不在这里处理终端按键、overlay 焦点或真正的 runtime 取消；这些边界分别留给
//! controller 与 app 宿主。

use super::commands::inspect::InspectCommand;
use super::commands::integrations::IntegrationCommand;
use super::commands::directory::DirectoryCommand;
use super::commands::registry::{
    CommandAction, CommandExecutionPolicy, CommandRegistry, CommandSpec, command_score,
};
use super::commands::session::SessionCommand;
use super::commands::settings::SettingsCommand;
use super::commands::schedules::{ScheduleCommand, parse_schedule_command};
use super::completion::{
    CompletionKind, active_completion_token, completion_suggestions,
    directory_command_suggestions,
};
use super::model::{
    UiConfirmIntent, UiConfirmOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay,
    UiSystemMessage, UiSystemMessageLevel,
};
use super::state::{TuiAction, TuiState, reduce_tui_state};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiSlashCommandKind {
    Help,
    Exit,
    Clear,
    Model,
    Acp,
    Resume,
    Status,
    Context,
    Usage,
    Cost,
    Stats,
    Diff,
    Files,
    Export,
    Rename,
    Branch,
    Compact,
    Rewind,
    Settings,
    Effort,
    Fast,
    Theme,
    Plan,
    Permissions,
    Cd,
    Project,
    Mcp,
    Memory,
    Skills,
    Plugins,
    Hooks,
    Doctor,
    ReloadSkills,
    ReloadPlugins,
    Tasks,
    Agents,
    Workflows,
    Schedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiSlashCommandInvocation {
    pub(crate) raw: String,
    pub(crate) token: String,
    pub(crate) argument: Option<String>,
    pub(crate) kind: Option<TuiSlashCommandKind>,
    pub(crate) action: Option<CommandAction>,
    pub(crate) execution_policy: Option<CommandExecutionPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptSuggestion {
    pub(crate) replacement: String,
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) kind: CompletionKind,
    pub(crate) score: i32,
    pub(crate) continue_completion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiSlashCommandOutcome {
    Continue,
    Quit,
    Resume { session_id: Option<String> },
    Inspect { command: InspectCommand, argument: Option<String> },
    Session(SessionCommand),
    Settings(SettingsCommand),
    Directory(DirectoryCommand),
    Integration(IntegrationCommand),
    OpenBackgroundTasks,
    OpenAgents,
    OpenWorkflows,
    RunWorkflow {
        uuid: String,
        query: String,
    },
    Schedule(ScheduleCommand),
    OpenRewindSelector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiPromptSuggestionMotion {
    Previous,
    Next,
    PreviousPage,
    NextPage,
}

pub(crate) const FILE_PICKER_VISIBLE_ROWS: usize = 8;

pub(crate) fn slash_command_help_text(
    registry: &CommandRegistry,
    include_unavailable: bool,
) -> String {
    let mut lines = vec!["斜杠命令:".to_string()];
    lines.extend(
        registry
            .commands()
            .iter()
            .filter(|spec| include_unavailable || spec.availability.is_available())
            .map(|spec| {
                let aliases = if spec.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" | /{}", spec.aliases.join(" | /"))
                };
                let argument = spec
                    .argument_hint
                    .as_deref()
                    .map(|hint| format!(" {hint}"))
                    .unwrap_or_default();
                let unavailable = spec
                    .availability
                    .unavailable_reason()
                    .map(|reason| format!("（不可用: {reason}）"))
                    .unwrap_or_default();
                format!("  /{}{aliases}{argument}   {}{unavailable}", spec.name, spec.summary)
            }),
    );
    lines.push("技能: 输入 `$` 选择技能；`/skills` 管理平台与启停。".to_string());
    lines.push("快捷键: F2 授权 · F3 待办 · F4 任务".to_string());
    lines.join("\n")
}

pub(crate) fn parse_slash_command(
    state: &TuiState,
    input: &str,
) -> Option<TuiSlashCommandInvocation> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let body = trimmed.trim_start_matches('/').trim();
    let (token, argument) = match body.split_once(char::is_whitespace) {
        Some((token, argument)) => (token.trim(), normalize_argument(argument)),
        None => (body, None),
    };
    let spec = state.command_registry.resolve(token);
    let kind = spec.and_then(CommandSpec::legacy_kind);

    Some(TuiSlashCommandInvocation {
        raw: trimmed.to_string(),
        token: token.to_string(),
        argument,
        kind,
        action: spec.map(|spec| spec.action.clone()),
        execution_policy: spec.map(|spec| spec.execution_policy),
    })
}

pub(crate) fn prompt_suggestions(state: &TuiState) -> Vec<TuiPromptSuggestion> {
    if state.prompt.completion_hidden {
        return Vec::new();
    }
    completion_suggestions(state)
}

pub(crate) fn slash_prompt_suggestions(state: &TuiState) -> Vec<TuiPromptSuggestion> {
    let raw = state.prompt.value.as_str();
    let trimmed = raw.trim_start();
    let (value, embedded_start) = if trimmed.starts_with('/') {
        (trimmed, None)
    } else {
        let Some((start, token)) = active_completion_token(raw) else {
            return Vec::new();
        };
        if !token.starts_with('/') {
            return Vec::new();
        }
        (token, Some(start))
    };

    let body = value.trim_start_matches('/');
    let (token, maybe_argument) = match body.split_once(char::is_whitespace) {
        Some((token, argument)) => (token.trim(), Some(argument.trim_start())),
        None => (body.trim(), None),
    };

    if let Some(argument) = maybe_argument
        && let Some(spec) = state.command_registry.resolve(token)
    {
        return argument_suggestions(spec, argument.trim(), state);
    }

    let partial = token.to_ascii_lowercase();
    let mut suggestions = state
        .command_registry
        .matching_commands(partial.as_str())
        .into_iter()
        .map(|spec| TuiPromptSuggestion {
            replacement: format!("/{}{}", spec.name, if spec.takes_argument { " " } else { "" }),
            label: format!("/{}", spec.name),
            detail: Some(command_detail(spec)),
            kind: CompletionKind::SlashCommand,
            score: command_score(spec),
            continue_completion: spec.takes_argument,
        })
        .collect::<Vec<_>>();
    if let Some(start) = embedded_start {
        for suggestion in &mut suggestions {
            suggestion.replacement =
                format!("{}{}", &raw[..start], suggestion.replacement);
        }
    }
    suggestions
}

pub(crate) fn apply_first_suggestion(state: &TuiState) -> Option<String> {
    apply_selected_suggestion(state)
}

pub(crate) fn selected_suggestion_index(
    state: &TuiState,
    suggestions: &[TuiPromptSuggestion],
) -> Option<usize> {
    if suggestions.is_empty() {
        None
    } else {
        Some(
            state
                .prompt
                .selected_suggestion_index
                .unwrap_or_default()
                .min(suggestions.len().saturating_sub(1)),
        )
    }
}

pub(crate) fn selected_prompt_suggestion(state: &TuiState) -> Option<TuiPromptSuggestion> {
    let suggestions = prompt_suggestions(state);
    let selected_index = selected_suggestion_index(state, &suggestions)?;
    suggestions.get(selected_index).cloned()
}

pub(crate) fn is_file_picker_suggestions(suggestions: &[TuiPromptSuggestion]) -> bool {
    suggestions.iter().any(|suggestion| {
        matches!(suggestion.kind, CompletionKind::Directory | CompletionKind::File)
    })
}

pub(crate) fn move_prompt_suggestion_selection(
    state: &TuiState,
    motion: TuiPromptSuggestionMotion,
) -> Option<usize> {
    let suggestions = prompt_suggestions(state);
    let len = suggestions.len();
    let current_index = selected_suggestion_index(state, &suggestions)?;
    let is_file_picker = is_file_picker_suggestions(&suggestions);
    Some(match motion {
        TuiPromptSuggestionMotion::Previous if is_file_picker => current_index.saturating_sub(1),
        TuiPromptSuggestionMotion::Previous => {
            current_index.checked_sub(1).unwrap_or(len.saturating_sub(1))
        }
        TuiPromptSuggestionMotion::Next if is_file_picker => {
            current_index.saturating_add(1).min(len.saturating_sub(1))
        }
        TuiPromptSuggestionMotion::Next => (current_index + 1) % len,
        TuiPromptSuggestionMotion::PreviousPage if is_file_picker => {
            current_index.saturating_sub(FILE_PICKER_VISIBLE_ROWS)
        }
        TuiPromptSuggestionMotion::NextPage if is_file_picker => {
            current_index.saturating_add(FILE_PICKER_VISIBLE_ROWS).min(len.saturating_sub(1))
        }
        TuiPromptSuggestionMotion::PreviousPage | TuiPromptSuggestionMotion::NextPage => {
            return None;
        }
    })
}

pub(crate) fn apply_selected_suggestion(state: &TuiState) -> Option<String> {
    let suggestion = selected_prompt_suggestion(state)?;
    let replacement = if suggestion.kind == CompletionKind::Skill
        && !suggestion.replacement.chars().last().is_some_and(char::is_whitespace)
    {
        format!("{} ", suggestion.replacement)
    } else {
        suggestion.replacement
    };
    (replacement != state.prompt.value).then_some(replacement)
}

pub(crate) fn execute_slash_command(
    state: &mut TuiState,
    invocation: &TuiSlashCommandInvocation,
) -> TuiSlashCommandOutcome {
    let dispatched = match state
        .command_registry
        .dispatch(invocation.token.as_str(), invocation.argument.clone())
    {
        Ok(dispatched) => dispatched,
        Err(error) => {
            let message = if error.starts_with("unknown slash command:") {
                let label = if invocation.token.trim().is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", invocation.token)
                };
                format!("未知的斜杠命令: {label}")
            } else {
                error
            };
            push_local_system_message(state, message, UiSystemMessageLevel::Warning);
            return TuiSlashCommandOutcome::Continue;
        }
    };

    match dispatched.action {
        CommandAction::Builtin(TuiSlashCommandKind::Help) => {
            let include_unavailable = match dispatched.argument.as_deref() {
                None => false,
                Some(argument) if argument.eq_ignore_ascii_case("all") => true,
                Some(_) => {
                    push_local_system_message(
                        state,
                        "用法: /help [all]",
                        UiSystemMessageLevel::Warning,
                    );
                    return TuiSlashCommandOutcome::Continue;
                }
            };
            let help = slash_command_help_text(&state.command_registry, include_unavailable);
            push_local_system_message(state, help, UiSystemMessageLevel::Info);
            TuiSlashCommandOutcome::Continue
        }
        CommandAction::Builtin(TuiSlashCommandKind::Exit) => {
            reduce_tui_state(
                state,
                TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                    title: "退出 TUI".to_string(),
                    body: "确认离开 TUI 并返回 shell 吗？".to_string(),
                    confirm_label: "退出".to_string(),
                    cancel_label: "继续留在这里".to_string(),
                    destructive: false,
                    intent: UiConfirmIntent::ExitTui,
                })),
            );
            TuiSlashCommandOutcome::Continue
        }
        CommandAction::Builtin(TuiSlashCommandKind::Clear) => {
            TuiSlashCommandOutcome::Session(SessionCommand::Clear)
        }
        CommandAction::Builtin(TuiSlashCommandKind::Model) => {
            TuiSlashCommandOutcome::Settings(SettingsCommand::OpenModelPicker)
        }
        CommandAction::Builtin(TuiSlashCommandKind::Acp) => {
            TuiSlashCommandOutcome::Settings(SettingsCommand::OpenAcpBackendPicker)
        }
        CommandAction::Builtin(TuiSlashCommandKind::Settings) => {
            TuiSlashCommandOutcome::Settings(SettingsCommand::Open)
        }
        CommandAction::Builtin(TuiSlashCommandKind::Effort) => {
            let Some(argument) = dispatched.argument.as_deref() else {
                return TuiSlashCommandOutcome::Settings(SettingsCommand::Open);
            };
            match parse_reasoning_effort(argument) {
                Some(effort) => TuiSlashCommandOutcome::Settings(SettingsCommand::Effort(effort)),
                None => {
                    push_local_system_message(
                        state,
                        "用法: /effort <minimal|low|medium|high|xhigh>",
                        UiSystemMessageLevel::Warning,
                    );
                    TuiSlashCommandOutcome::Continue
                }
            }
        }
        CommandAction::Builtin(TuiSlashCommandKind::Fast) => {
            let enabled = !matches!(dispatched.argument.as_deref(), Some("off" | "false" | "0"));
            TuiSlashCommandOutcome::Settings(SettingsCommand::Fast(enabled))
        }
        CommandAction::Builtin(TuiSlashCommandKind::Theme) => match dispatched.argument {
            Some(theme) => TuiSlashCommandOutcome::Settings(SettingsCommand::Theme(theme)),
            None => TuiSlashCommandOutcome::Settings(SettingsCommand::Open),
        },
        CommandAction::Builtin(TuiSlashCommandKind::Plan) => TuiSlashCommandOutcome::Settings(
            SettingsCommand::PermissionMode(super::model::PermissionMode::Plan),
        ),
        CommandAction::Builtin(TuiSlashCommandKind::Permissions) => {
            TuiSlashCommandOutcome::Settings(SettingsCommand::Permissions)
        }
        CommandAction::Builtin(TuiSlashCommandKind::Cd) => {
            let Some(path) = dispatched
                .argument
                .map(|path| path.trim_matches('"').trim_matches('\'').to_string())
                .filter(|path| !path.is_empty())
            else {
                push_local_system_message(
                    state,
                    "用法: /cd <path>",
                    UiSystemMessageLevel::Warning,
                );
                return TuiSlashCommandOutcome::Continue;
            };
            TuiSlashCommandOutcome::Directory(DirectoryCommand::Change(path.into()))
        }
        CommandAction::Builtin(TuiSlashCommandKind::Project) => {
            TuiSlashCommandOutcome::Directory(DirectoryCommand::Project)
        }
        CommandAction::Builtin(kind @ (
            TuiSlashCommandKind::Mcp
            | TuiSlashCommandKind::Memory
            | TuiSlashCommandKind::Skills
            | TuiSlashCommandKind::Plugins
            | TuiSlashCommandKind::Hooks
            | TuiSlashCommandKind::Doctor
            | TuiSlashCommandKind::ReloadSkills
            | TuiSlashCommandKind::ReloadPlugins
        )) => TuiSlashCommandOutcome::Integration(match kind {
            TuiSlashCommandKind::Mcp => IntegrationCommand::Mcp,
            TuiSlashCommandKind::Memory => IntegrationCommand::Memory,
            TuiSlashCommandKind::Skills => IntegrationCommand::Skills,
            TuiSlashCommandKind::Plugins => IntegrationCommand::Plugins,
            TuiSlashCommandKind::Hooks => IntegrationCommand::Hooks,
            TuiSlashCommandKind::Doctor => IntegrationCommand::Doctor,
            TuiSlashCommandKind::ReloadSkills => IntegrationCommand::ReloadSkills,
            TuiSlashCommandKind::ReloadPlugins => IntegrationCommand::ReloadPlugins,
            _ => unreachable!("integration match contains only integration kinds"),
        }),
        CommandAction::Builtin(TuiSlashCommandKind::Tasks) => {
            TuiSlashCommandOutcome::OpenBackgroundTasks
        }
        CommandAction::Builtin(TuiSlashCommandKind::Agents) => {
            TuiSlashCommandOutcome::OpenAgents
        }
        CommandAction::Builtin(TuiSlashCommandKind::Workflows) => {
            TuiSlashCommandOutcome::OpenWorkflows
        }
        CommandAction::Builtin(TuiSlashCommandKind::Schedule) => {
            match parse_schedule_command(dispatched.argument.as_deref()) {
                Ok(command) => TuiSlashCommandOutcome::Schedule(command),
                Err(error) => {
                    push_local_system_message(state, error, UiSystemMessageLevel::Warning);
                    TuiSlashCommandOutcome::Continue
                }
            }
        }
        CommandAction::Builtin(TuiSlashCommandKind::Resume) => {
            TuiSlashCommandOutcome::Resume { session_id: dispatched.argument }
        }
        CommandAction::Builtin(
            kind @ (TuiSlashCommandKind::Status
            | TuiSlashCommandKind::Context
            | TuiSlashCommandKind::Usage
            | TuiSlashCommandKind::Cost
            | TuiSlashCommandKind::Stats
            | TuiSlashCommandKind::Diff
            | TuiSlashCommandKind::Files
            | TuiSlashCommandKind::Export),
        ) => {
            let command = match kind {
                TuiSlashCommandKind::Status => InspectCommand::Status,
                TuiSlashCommandKind::Context => InspectCommand::Context,
                TuiSlashCommandKind::Usage => InspectCommand::Usage,
                TuiSlashCommandKind::Cost => InspectCommand::Cost,
                TuiSlashCommandKind::Stats => InspectCommand::Stats,
                TuiSlashCommandKind::Diff => InspectCommand::Diff,
                TuiSlashCommandKind::Files => InspectCommand::Files,
                TuiSlashCommandKind::Export => InspectCommand::Export,
                _ => unreachable!("inspect match only contains inspect command kinds"),
            };
            TuiSlashCommandOutcome::Inspect { command, argument: dispatched.argument }
        }
        CommandAction::Builtin(TuiSlashCommandKind::Rename) => {
            let Some(title) = dispatched
                .argument
                .map(|title| title.trim().to_string())
                .filter(|title| !title.is_empty())
            else {
                push_local_system_message(
                    state,
                    "用法: /rename <title>",
                    UiSystemMessageLevel::Warning,
                );
                return TuiSlashCommandOutcome::Continue;
            };
            TuiSlashCommandOutcome::Session(SessionCommand::Rename { title })
        }
        CommandAction::Builtin(TuiSlashCommandKind::Branch) => {
            let message_id = dispatched.argument.as_deref().map(normalize_gateway_message_id);
            TuiSlashCommandOutcome::Session(SessionCommand::Fork { message_id })
        }
        CommandAction::Builtin(TuiSlashCommandKind::Compact) => {
            let Some(message_id) = latest_gateway_message_id(state) else {
                push_local_system_message(
                    state,
                    "当前会话没有可用于 compact 的持久化 message ID",
                    UiSystemMessageLevel::Warning,
                );
                return TuiSlashCommandOutcome::Continue;
            };
            TuiSlashCommandOutcome::Session(SessionCommand::Compact { message_id })
        }
        CommandAction::Builtin(TuiSlashCommandKind::Rewind) => {
            match dispatched.argument.as_deref() {
                Some(message_id) => TuiSlashCommandOutcome::Session(SessionCommand::Reset {
                    message_id: normalize_gateway_message_id(message_id),
                }),
                None => TuiSlashCommandOutcome::OpenRewindSelector,
            }
        }
        CommandAction::PromptExpansion(prefix) => {
            let expanded = dispatched
                .argument
                .as_deref()
                .map(|argument| format!("{prefix} {argument}"))
                .unwrap_or(prefix);
            reduce_tui_state(state, TuiAction::PromptValueSet(expanded));
            push_local_system_message(
                state,
                format!("/{} 已展开到输入框，请确认后提交。", dispatched.name),
                UiSystemMessageLevel::Info,
            );
            TuiSlashCommandOutcome::Continue
        }
        CommandAction::Gateway(target) if target.starts_with("workflow:") => {
            let Some(query) = dispatched.argument.filter(|value| !value.trim().is_empty()) else {
                push_local_system_message(
                    state,
                    format!("用法: /{} <input>", dispatched.name),
                    UiSystemMessageLevel::Warning,
                );
                return TuiSlashCommandOutcome::Continue;
            };
            TuiSlashCommandOutcome::RunWorkflow {
                uuid: target.trim_start_matches("workflow:").to_string(),
                query,
            }
        }
        CommandAction::Gateway(_)
        | CommandAction::LocalEffect(_)
        | CommandAction::ExternalEffect(_) => {
            push_local_system_message(
                state,
                format!("/{} 已注册，但当前命令处理器尚未接入", dispatched.name),
                UiSystemMessageLevel::Warning,
            );
            TuiSlashCommandOutcome::Continue
        }
    }
}

fn parse_reasoning_effort(value: &str) -> Option<super::model::ReasoningEffort> {
    match value.trim().to_ascii_lowercase().as_str() {
        "minimal" | "none" => Some(super::model::ReasoningEffort::Minimal),
        "low" => Some(super::model::ReasoningEffort::Low),
        "medium" => Some(super::model::ReasoningEffort::Medium),
        "high" => Some(super::model::ReasoningEffort::High),
        "xhigh" | "x-high" => Some(super::model::ReasoningEffort::XHigh),
        _ => None,
    }
}

fn normalize_gateway_message_id(message_id: &str) -> String {
    message_id.trim().strip_prefix("gateway:").unwrap_or(message_id.trim()).to_string()
}

fn latest_gateway_message_id(state: &TuiState) -> Option<String> {
    state
        .messages
        .iter()
        .rev()
        .find_map(|message| message.id().as_str().strip_prefix("gateway:").map(str::to_string))
}

fn argument_suggestions(
    spec: &CommandSpec,
    current_argument: &str,
    state: &TuiState,
) -> Vec<TuiPromptSuggestion> {
    match &spec.action {
        CommandAction::Builtin(TuiSlashCommandKind::Resume) => state
            .session
            .preview
            .as_ref()
            .filter(|preview| preview.id != current_argument)
            .map(|preview| TuiPromptSuggestion {
                replacement: format!("/resume {}", preview.id),
                label: format!("/resume {}", preview.id),
                detail: Some(format!("最近一次快照: {}", preview.title)),
                kind: CompletionKind::SlashCommand,
                score: 100,
                continue_completion: false,
            })
            .into_iter()
            .collect(),
        CommandAction::Builtin(TuiSlashCommandKind::Cd) => {
            directory_command_suggestions(state, current_argument)
        }
        _ => Vec::new(),
    }
}

fn command_detail(spec: &CommandSpec) -> String {
    let alias_detail = if spec.aliases.is_empty() {
        String::new()
    } else {
        format!("；别名: {}", spec.aliases.join(", "))
    };
    let argument_detail =
        spec.argument_hint.as_deref().map(|hint| format!("；参数: {hint}")).unwrap_or_default();
    let availability_detail = spec
        .availability
        .unavailable_reason()
        .map(|reason| format!("；不可用: {reason}"))
        .unwrap_or_default();
    let availability_label = if spec.availability.is_available() { "" } else { " · disabled" };
    format!(
        "{}{}{}；来源: {:?}{}{}",
        spec.summary,
        alias_detail,
        argument_detail,
        spec.source,
        availability_label,
        availability_detail
    )
}

fn normalize_argument(argument: &str) -> Option<String> {
    let argument = argument.trim();
    if argument.is_empty() { None } else { Some(argument.to_string()) }
}

fn push_local_system_message(
    state: &mut TuiState,
    text: impl Into<String>,
    level: UiSystemMessageLevel,
) {
    state.append_message(UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local(format!(
            "ui-slash-system-{}",
            state.messages.len()
        ))),
        text: text.into(),
        level,
    }));
}
