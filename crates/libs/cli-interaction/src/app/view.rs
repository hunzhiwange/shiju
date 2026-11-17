//! 会话视图与消息投影。

#[cfg(all(not(test), not(target_arch = "wasm32")))]
use super::{
    ChatWidget,
    ChatWidgetConfig,
    GatewayUiRuntime,
    GatewayV3Bridge,
    PermissionMode,
    ReasoningEffort,
    SESSION_HISTORY_DISPLAY_LIMIT,
    TuiState,
    UiMessage,
    UiStepState,
    V3Attachment,
    V3AttachmentKind,
    V3DisplayEvent,
    V3SelectionItem,
    V3StepStatus,
    command_palette_items_with_integrations,
};
#[cfg(any(test, target_arch = "wasm32"))]
use super::{
    ChatWidget,
    ChatWidgetConfig,
    GatewayUiRuntime,
    GatewayV3Bridge,
    PermissionMode,
    ReasoningEffort,
    SESSION_HISTORY_DISPLAY_LIMIT,
    TuiState,
    UiAttachmentKind,
    UiMessage,
    UiStepState,
    V3Attachment,
    V3AttachmentKind,
    V3DisplayEvent,
    V3SelectionItem,
    V3StepStatus,
    command_palette_items_with_integrations,
};

pub(super) fn build_session_widget(
    runtime: &GatewayUiRuntime,
    state: &TuiState,
    draft: Option<&str>,
) -> ChatWidget {
    let display_data = GatewayV3Bridge::session_display_data(runtime, state);
    let mut widget = ChatWidget::new(ChatWidgetConfig::default());
    widget.set_command_palette_items(command_palette_items_with_integrations(
        &state.integration_catalog,
    ));
    widget.push_session_header(display_data);
    replay_session_history(&mut widget, state);
    if let Some(draft) = draft {
        widget.replace_draft(draft);
    }
    widget
}

pub(super) fn setting_item(id: &str, label: &str, selected: bool) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: selected.then(|| "当前值".to_string()),
        search_value: Some(format!("{id} {label}")),
        ..V3SelectionItem::default()
    }
}

pub(super) fn read_only_item(id: &str, label: &str, description: &str) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: Some(description.to_string()),
        search_value: Some(format!("{id} {label} {description}")),
        disabled: true,
        disabled_reason: Some("状态由 Gateway 管理".to_string()),
        ..V3SelectionItem::default()
    }
}

pub(super) fn selectable_item(id: &str, label: &str, description: &str) -> V3SelectionItem {
    V3SelectionItem {
        id: id.to_string(),
        label: label.to_string(),
        description: Some(description.to_string()),
        search_value: Some(format!("{id} {label} {description}")),
        ..V3SelectionItem::default()
    }
}

pub(super) fn effort_id(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Auto => "auto",
        ReasoningEffort::None => "none",
        ReasoningEffort::Max => "max",
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::XHigh => "xhigh",
    }
}

pub(super) fn permission_mode_id(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::Plan => "plan",
        PermissionMode::AcceptEdits => "accept-edits",
        PermissionMode::Bypass => "bypass",
    }
}

/// 作为 CLI 交互入口启动 Gateway、初始化会话并运行终端循环。
pub(super) fn replay_session_history(widget: &mut ChatWidget, state: &TuiState) {
    let displayable =
        state.messages.iter().filter(|message| !matches!(message, UiMessage::Step(_))).count();
    let skip = displayable.saturating_sub(SESSION_HISTORY_DISPLAY_LIMIT);
    for message in
        state.messages.iter().filter(|message| !matches!(message, UiMessage::Step(_))).skip(skip)
    {
        widget.push_history(display_event_from_message(message));
    }
}

pub(super) fn display_event_from_message(message: &UiMessage) -> V3DisplayEvent {
    match message {
        UiMessage::User(message) => {
            V3DisplayEvent::User { text: message.text.clone(), attachments: Vec::new() }
        }
        UiMessage::Assistant(message) => {
            V3DisplayEvent::Assistant { markdown: message.text.clone() }
        }
        UiMessage::ToolCall(message) => V3DisplayEvent::ToolCall {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            tool: message.tool_name.clone(),
            command: message
                .arguments
                .clone()
                .or_else(|| message.summary.clone())
                .unwrap_or_default(),
        },
        UiMessage::ToolResult(message) => V3DisplayEvent::ToolResult {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            tool: message.tool_name.clone(),
            command: message.command.clone().unwrap_or_default(),
            output: message.content.clone(),
            exit_code: i32::from(message.is_error),
            duration_ms: 0,
        },
        UiMessage::Thinking(message) => V3DisplayEvent::Thinking { text: message.content.clone() },
        UiMessage::Step(message) => V3DisplayEvent::Step {
            title: message
                .finish_reason
                .clone()
                .unwrap_or_else(|| format!("step {}", message.step_index)),
            status: match message.state {
                UiStepState::Pending => V3StepStatus::Pending,
                UiStepState::Running => V3StepStatus::InProgress,
                UiStepState::Complete => V3StepStatus::Completed,
                UiStepState::Cancelled | UiStepState::Failed => V3StepStatus::Failed,
            },
        },
        UiMessage::System(message) => V3DisplayEvent::System { text: message.text.clone() },
        UiMessage::Error(message) => V3DisplayEvent::Error { message: message.message.clone() },
        UiMessage::Attachment(message) => V3DisplayEvent::Attachment {
            attachment: V3Attachment {
                kind: match message.kind {
                    cli_interaction_model::UiAttachmentKind::Image => V3AttachmentKind::Image,
                    cli_interaction_model::UiAttachmentKind::PastedText => V3AttachmentKind::PastedText,
                    cli_interaction_model::UiAttachmentKind::Resource
                    | cli_interaction_model::UiAttachmentKind::Link => V3AttachmentKind::Resource,
                    cli_interaction_model::UiAttachmentKind::File => V3AttachmentKind::File,
                },
                label: message.name.clone(),
                path: (!message.uri.trim().is_empty()).then(|| message.uri.clone()),
                media_type: message.media_type.clone(),
                text: message.preview.clone().or_else(|| message.alt_text.clone()),
                bytes: None,
            },
        },
        UiMessage::Progress(message) => V3DisplayEvent::Progress {
            label: message.label.clone(),
            detail: match (message.current, message.total) {
                (Some(current), Some(total)) => Some(format!("{current}/{total}")),
                (Some(current), None) => Some(current.to_string()),
                _ => None,
            },
        },
        UiMessage::CompactBoundary(message) => {
            V3DisplayEvent::CompactBoundary { summary: message.summary.clone() }
        }
        UiMessage::Unknown(message) => V3DisplayEvent::Unknown {
            type_name: message.external_kind.clone(),
            detail: message.summary.clone(),
        },
    }
}
