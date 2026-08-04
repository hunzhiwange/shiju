use super::super::model::{UiConfirmIntent, UiConfirmOverlay, UiMessage};
use super::super::state::TuiState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionCommand {
    Resume { session_id: String },
    Rename { title: String },
    Fork { message_id: Option<String> },
    Reset { message_id: String },
    Compact { message_id: String },
    Clear,
}

pub(crate) fn session_confirmation_overlay(command: &SessionCommand) -> Option<UiConfirmOverlay> {
    let intent = command.confirmation_intent()?;
    let (title, body, confirm_label) = match &intent {
        UiConfirmIntent::ClearSession => (
            "清空会话".to_string(),
            "确认创建新的空会话吗？当前会话会先持久化。".to_string(),
            "清空".to_string(),
        ),
        UiConfirmIntent::ResetSession { message_id } => (
            "回退会话".to_string(),
            format!("确认回退到 message {message_id} 并回滚相关代码变更吗？"),
            "回退".to_string(),
        ),
        UiConfirmIntent::CompactSession { message_id } => (
            "压缩上下文".to_string(),
            format!("确认从 message {message_id} 生成会话摘要吗？"),
            "压缩".to_string(),
        ),
        UiConfirmIntent::Generic
        | UiConfirmIntent::ExitTui
        | UiConfirmIntent::IntegrationDelete { .. }
        | UiConfirmIntent::ScheduleDelete { .. }
        | UiConfirmIntent::EnableFullAccess { .. } => return None,
    };
    Some(UiConfirmOverlay {
        title,
        body,
        confirm_label,
        cancel_label: "取消".to_string(),
        destructive: true,
        intent,
    })
}

impl SessionCommand {
    pub(crate) fn confirmation_intent(&self) -> Option<UiConfirmIntent> {
        match self {
            Self::Reset { message_id } => {
                Some(UiConfirmIntent::ResetSession { message_id: message_id.clone() })
            }
            Self::Compact { message_id } => {
                Some(UiConfirmIntent::CompactSession { message_id: message_id.clone() })
            }
            Self::Clear => Some(UiConfirmIntent::ClearSession),
            Self::Resume { .. } | Self::Rename { .. } | Self::Fork { .. } => None,
        }
    }

    fn requires_persist(&self) -> bool {
        !matches!(self, Self::Rename { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionReplacement {
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) messages: Vec<UiMessage>,
    pub(crate) scope: Option<String>,
}

pub(crate) trait SessionCommandRuntime {
    fn persist_current(&mut self, state: &TuiState) -> Result<(), String>;
    fn apply(
        &mut self,
        command: &SessionCommand,
        state: &TuiState,
    ) -> Result<SessionReplacement, String>;
}

pub(crate) fn execute_session_command(
    state: &mut TuiState,
    command: &SessionCommand,
    confirmation: Option<&UiConfirmIntent>,
    runtime: &mut dyn SessionCommandRuntime,
) -> Result<(), String> {
    if let Some(required) = command.confirmation_intent()
        && confirmation != Some(&required)
    {
        return Err("destructive session command requires matching confirmation intent".to_string());
    }
    if command.requires_persist() {
        runtime.persist_current(state)?;
    }
    let replacement = runtime.apply(command, state)?;
    let draft = state.prompt.value.clone();
    state.clear_messages();
    state.messages = replacement.messages;
    state.session.session_id = Some(replacement.session_id);
    state.session.title = replacement.title.clone();
    state.session.scope = replacement.scope;
    state.status.session_title = replacement.title;
    state.prompt.set_value(draft);
    Ok(())
}
