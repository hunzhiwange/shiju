//! tui_v2 输入层的 slash command 与 prompt suggestion 逻辑。
//!
//! 本模块只负责两类纯输入协议：
//! - `/...` 命令的解析、建议与执行
//! - prompt footer 可直接消费的轻量 suggestion 列表
//!
//! 不在这里处理终端按键、overlay 焦点或真正的 runtime 取消；这些边界分别留给
//! controller 与 app 宿主。

use std::collections::HashSet;

use super::model::{
    UiConfirmOverlay, UiMessage, UiMessageBase, UiMessageId, UiOverlay, UiSystemMessage,
    UiSystemMessageLevel,
};
use super::state::{TuiAction, TuiState, reduce_tui_state};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiSlashCommandKind {
    Help,
    Exit,
    Clear,
    Model,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiSlashCommandInvocation {
    pub(crate) raw: String,
    pub(crate) token: String,
    pub(crate) argument: Option<String>,
    pub(crate) kind: Option<TuiSlashCommandKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptSuggestion {
    pub(crate) replacement: String,
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiSlashCommandOutcome {
    Continue,
    Quit,
    Resume { session_id: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiPromptSuggestionMotion {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy)]
struct TuiSlashCommandSpec {
    kind: TuiSlashCommandKind,
    name: &'static str,
    aliases: &'static [&'static str],
    summary: &'static str,
    takes_argument: bool,
}

const SLASH_COMMANDS: [TuiSlashCommandSpec; 5] = [
    TuiSlashCommandSpec {
        kind: TuiSlashCommandKind::Help,
        name: "help",
        aliases: &[],
        summary: "显示可用的斜杠命令",
        takes_argument: false,
    },
    TuiSlashCommandSpec {
        kind: TuiSlashCommandKind::Clear,
        name: "clear",
        aliases: &["new"],
        summary: "清空当前会话内容",
        takes_argument: false,
    },
    TuiSlashCommandSpec {
        kind: TuiSlashCommandKind::Model,
        name: "model",
        aliases: &[],
        summary: "查看、选择或手工输入当前模型",
        takes_argument: true,
    },
    TuiSlashCommandSpec {
        kind: TuiSlashCommandKind::Resume,
        name: "resume",
        aliases: &[],
        summary: "恢复最近一次或指定的会话快照",
        takes_argument: true,
    },
    TuiSlashCommandSpec {
        kind: TuiSlashCommandKind::Exit,
        name: "exit",
        aliases: &["quit"],
        summary: "退出 TUI",
        takes_argument: false,
    },
];

pub(crate) fn slash_command_help_text() -> &'static str {
    "斜杠命令:\n  /help            查看帮助\n  /clear | /new   清空当前会话前先二次确认\n  /model [name]   从模型列表选择，或手工输入 provider/model、自定义模型 ID\n  /resume [id]    恢复最近一次或指定会话\n  /exit | /quit   退出 TUI 前先二次确认\n快捷键:\n  F2              打开待处理问题 / 授权面板\n  F3              打开待办面板\n  F4              打开任务面板"
}

pub(crate) fn parse_slash_command(input: &str) -> Option<TuiSlashCommandInvocation> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let body = trimmed.trim_start_matches('/').trim();
    let (token, argument) = match body.split_once(char::is_whitespace) {
        Some((token, argument)) => (token.trim(), normalize_argument(argument)),
        None => (body, None),
    };
    let kind = slash_command_spec(token).map(|spec| spec.kind);

    Some(TuiSlashCommandInvocation {
        raw: trimmed.to_string(),
        token: token.to_string(),
        argument,
        kind,
    })
}

pub(crate) fn prompt_suggestions(state: &TuiState) -> Vec<TuiPromptSuggestion> {
    let value = state.prompt.value.trim_start();
    if !value.starts_with('/') {
        return Vec::new();
    }

    let body = value.trim_start_matches('/');
    let (token, maybe_argument) = match body.split_once(char::is_whitespace) {
        Some((token, argument)) => (token.trim(), Some(argument.trim_start())),
        None => (body.trim(), None),
    };

    if let Some(argument) = maybe_argument
        && let Some(spec) = slash_command_spec(token)
    {
        return argument_suggestions(spec, argument.trim(), state);
    }

    let partial = token.to_ascii_lowercase();
    SLASH_COMMANDS
        .iter()
        .filter(|spec| partial.is_empty() || slash_command_matches(spec, partial.as_str()))
        .map(|spec| TuiPromptSuggestion {
            replacement: format!("/{}{}", spec.name, if spec.takes_argument { " " } else { "" }),
            label: format!("/{}", spec.name),
            detail: Some(command_detail(spec)),
        })
        .collect()
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

pub(crate) fn move_prompt_suggestion_selection(
    state: &TuiState,
    motion: TuiPromptSuggestionMotion,
) -> Option<usize> {
    let suggestions = prompt_suggestions(state);
    let len = suggestions.len();
    let current_index = selected_suggestion_index(state, &suggestions)?;
    Some(match motion {
        TuiPromptSuggestionMotion::Previous => {
            current_index.checked_sub(1).unwrap_or(len.saturating_sub(1))
        }
        TuiPromptSuggestionMotion::Next => (current_index + 1) % len,
    })
}

pub(crate) fn apply_selected_suggestion(state: &TuiState) -> Option<String> {
    selected_prompt_suggestion(state)
        .map(|suggestion| suggestion.replacement)
        .filter(|replacement| replacement != &state.prompt.value)
}

pub(crate) fn execute_slash_command(
    state: &mut TuiState,
    invocation: &TuiSlashCommandInvocation,
) -> TuiSlashCommandOutcome {
    match invocation.kind {
        Some(TuiSlashCommandKind::Help) => {
            push_local_system_message(state, slash_command_help_text(), UiSystemMessageLevel::Info);
            TuiSlashCommandOutcome::Continue
        }
        Some(TuiSlashCommandKind::Exit) => {
            reduce_tui_state(
                state,
                TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                    title: "退出 TUI".to_string(),
                    body: "确认离开 TUI 并返回 shell 吗？".to_string(),
                    confirm_label: "退出".to_string(),
                    cancel_label: "继续留在这里".to_string(),
                    destructive: false,
                })),
            );
            TuiSlashCommandOutcome::Continue
        }
        Some(TuiSlashCommandKind::Clear) => {
            reduce_tui_state(
                state,
                TuiAction::OverlayPushed(UiOverlay::Confirm(UiConfirmOverlay {
                    title: "清空会话".to_string(),
                    body: "确认清空当前会话内容，但保留当前会话上下文吗？".to_string(),
                    confirm_label: "清空".to_string(),
                    cancel_label: "保留当前内容".to_string(),
                    destructive: true,
                })),
            );
            TuiSlashCommandOutcome::Continue
        }
        Some(TuiSlashCommandKind::Model) => {
            if let Some(model) = invocation.argument.as_ref() {
                let next_provider = provider_name_for_model_input(state, model);
                reduce_tui_state(state, TuiAction::StatusProviderSet(next_provider));
                reduce_tui_state(state, TuiAction::StatusModelSet(Some(model.clone())));
                push_local_system_message(
                    state,
                    "模型已切换，当前值见输入框底部。",
                    UiSystemMessageLevel::Success,
                );
            } else {
                let hint = if state.model_catalog.is_empty() {
                    "输入 /model <provider/model> 或 /model <自定义模型 ID> 切换模型。"
                } else {
                    "输入 /model 后继续键入筛选，按 Up/Down 切换候选项，按 Tab 或 Enter 接受当前建议，也可直接手工输入自定义模型 ID。"
                };
                push_local_system_message(
                    state,
                    format!("{hint}\n当前值见输入框底部。"),
                    UiSystemMessageLevel::Info,
                );
            }
            TuiSlashCommandOutcome::Continue
        }
        Some(TuiSlashCommandKind::Resume) => {
            TuiSlashCommandOutcome::Resume { session_id: invocation.argument.clone() }
        }
        None => {
            let label = if invocation.token.trim().is_empty() {
                "/".to_string()
            } else {
                format!("/{}", invocation.token)
            };
            push_local_system_message(
                state,
                format!("未知的斜杠命令: {label}"),
                UiSystemMessageLevel::Warning,
            );
            TuiSlashCommandOutcome::Continue
        }
    }
}

fn slash_command_spec(token: &str) -> Option<&'static TuiSlashCommandSpec> {
    SLASH_COMMANDS.iter().find(|spec| {
        spec.name.eq_ignore_ascii_case(token)
            || spec.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(token))
    })
}

fn slash_command_matches(spec: &TuiSlashCommandSpec, partial: &str) -> bool {
    spec.name.starts_with(partial) || spec.aliases.iter().any(|alias| alias.starts_with(partial))
}

fn argument_suggestions(
    spec: &TuiSlashCommandSpec,
    current_argument: &str,
    state: &TuiState,
) -> Vec<TuiPromptSuggestion> {
    match spec.kind {
        TuiSlashCommandKind::Model => model_argument_suggestions(current_argument, state),
        TuiSlashCommandKind::Resume => state
            .session
            .preview
            .as_ref()
            .filter(|preview| preview.id != current_argument)
            .map(|preview| TuiPromptSuggestion {
                replacement: format!("/resume {}", preview.id),
                label: format!("/resume {}", preview.id),
                detail: Some(format!("最近一次快照: {}", preview.title)),
            })
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn model_argument_suggestions(
    current_argument: &str,
    state: &TuiState,
) -> Vec<TuiPromptSuggestion> {
    const MAX_MODEL_SUGGESTIONS: usize = 12;

    let query = current_argument.trim();
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();
    let active_model = state.status.model_name.as_deref();
    let mut catalog_contains_active_model = false;

    for entry in &state.model_catalog {
        if !entry.matches_query(query) {
            continue;
        }

        let qualified_id = entry.qualified_id();
        if active_model.is_some_and(|model| {
            model.eq_ignore_ascii_case(qualified_id.as_str())
                || model.eq_ignore_ascii_case(entry.model_id.as_str())
        }) {
            catalog_contains_active_model = true;
        }
        if qualified_id.eq_ignore_ascii_case(query) {
            continue;
        }

        let replacement = format!("/model {qualified_id}");
        if !seen.insert(replacement.clone()) {
            continue;
        }

        suggestions.push(TuiPromptSuggestion {
            replacement: replacement.clone(),
            label: replacement,
            detail: Some(entry.suggestion_detail()),
        });

        if suggestions.len() >= MAX_MODEL_SUGGESTIONS {
            break;
        }
    }

    if suggestions.len() < MAX_MODEL_SUGGESTIONS
        && !catalog_contains_active_model
        && let Some(model) = active_model
            .filter(|model| !model.trim().is_empty())
            .filter(|model| model != &query)
            .filter(|model| query.is_empty() || candidate_matches_query(model, query))
    {
        let replacement = format!("/model {model}");
        if seen.insert(replacement.clone()) {
            suggestions.push(TuiPromptSuggestion {
                replacement: replacement.clone(),
                label: replacement,
                detail: Some("复用当前正在使用的模型".to_string()),
            });
        }
    }

    suggestions
}

fn command_detail(spec: &TuiSlashCommandSpec) -> String {
    let alias_detail = if spec.aliases.is_empty() {
        String::new()
    } else {
        format!("；别名: {}", spec.aliases.join(", "))
    };
    format!("{}{}", spec.summary, alias_detail)
}

fn normalize_argument(argument: &str) -> Option<String> {
    let argument = argument.trim();
    if argument.is_empty() { None } else { Some(argument.to_string()) }
}

fn candidate_matches_query(candidate: &str, query: &str) -> bool {
    candidate.to_ascii_lowercase().contains(query.to_ascii_lowercase().as_str())
}

fn provider_name_for_model_input(state: &TuiState, model: &str) -> Option<String> {
    let model = model.trim();
    if model.is_empty() {
        return state.status.provider_name.clone();
    }

    if let Some((provider_id, _)) = model.split_once('/') {
        let provider_id = provider_id.trim();
        if !provider_id.is_empty() {
            return Some(provider_id.to_string());
        }
    }

    state
        .model_catalog
        .iter()
        .find(|entry| {
            state.status.provider_name.as_deref().is_some_and(|provider_id| {
                entry.provider_id.eq_ignore_ascii_case(provider_id)
                    && entry.model_id.eq_ignore_ascii_case(model)
            })
        })
        .or_else(|| {
            state.model_catalog.iter().find(|entry| entry.model_id.eq_ignore_ascii_case(model))
        })
        .map(|entry| entry.provider_id.clone())
        .or_else(|| state.status.provider_name.clone())
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
