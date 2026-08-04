use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::controller::TuiControllerCommand;
use super::model::{
    MessageActionIntent, MessageActionKind, MessageActionState, UiMessage, UiMessageId,
};
use super::state::TuiState;

pub(crate) fn handle_message_action_key(
    state: &mut TuiState,
    key: KeyEvent,
) -> Option<TuiControllerCommand> {
    if !state.message_actions.active {
        let kind = if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('r'))
        {
            Some(MessageActionKind::Rewind)
        } else if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('m'))
        {
            Some(MessageActionKind::Browse)
        } else {
            None
        }?;
        open_message_actions(state, kind);
        return Some(TuiControllerCommand::Continue);
    }

    reconcile_selection(state);
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
        || matches!(key.code, KeyCode::Esc)
    {
        cancel_message_actions(state);
        return Some(TuiControllerCommand::Continue);
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => move_selection(state, -1, false),
        KeyCode::Down | KeyCode::Char('j') => move_selection(state, 1, false),
        KeyCode::Char('u') => move_selection(state, -1, true),
        KeyCode::Char('U') => move_selection(state, 1, true),
        KeyCode::Char(' ') => state.message_actions.expanded = !state.message_actions.expanded,
        KeyCode::Char('c') => return selected_intent(state, MessageAction::Copy),
        KeyCode::Char('e') => return selected_intent(state, MessageAction::Edit),
        KeyCode::Char('f') => return selected_intent(state, MessageAction::Fork),
        KeyCode::Char('r') => return selected_intent(state, MessageAction::Restore),
        KeyCode::Char('s') => return selected_intent(state, MessageAction::Summarize),
        KeyCode::Char('w') | KeyCode::Enter => {
            return selected_intent(state, MessageAction::Rewind);
        }
        _ => {}
    }
    Some(TuiControllerCommand::Continue)
}

pub(crate) fn open_message_actions(state: &mut TuiState, kind: MessageActionKind) -> bool {
    let selected_id =
        navigable_messages(state).last().map(|(_, message)| message.base().id.clone());
    if selected_id.is_none() {
        return false;
    }
    state.message_actions.active = true;
    state.message_actions.selected_id = selected_id;
    state.message_actions.expanded = false;
    state.message_actions.kind = kind;
    state.message_actions.preserved_prompt_draft = state.prompt.value.clone();
    true
}

fn cancel_message_actions(state: &mut TuiState) {
    let draft = state.message_actions.preserved_prompt_draft.clone();
    state.message_actions = MessageActionState::default();
    if state.prompt.value != draft {
        state.prompt.set_value(draft);
    }
}

fn reconcile_selection(state: &mut TuiState) {
    let ids = navigable_messages(state)
        .into_iter()
        .map(|(_, message)| message.base().id.clone())
        .collect::<Vec<_>>();
    if !state.message_actions.selected_id.as_ref().is_some_and(|selected| ids.contains(selected)) {
        state.message_actions.selected_id = ids.last().cloned();
    }
    if state.message_actions.selected_id.is_none() {
        state.message_actions = MessageActionState::default();
    }
}

fn move_selection(state: &mut TuiState, delta: isize, users_only: bool) {
    let choices = navigable_messages(state)
        .into_iter()
        .filter(|(_, message)| !users_only || matches!(message, UiMessage::User(_)))
        .map(|(_, message)| message.base().id.clone())
        .collect::<Vec<_>>();
    if choices.is_empty() {
        return;
    }
    let current = state
        .message_actions
        .selected_id
        .as_ref()
        .and_then(|selected| choices.iter().position(|id| id == selected));
    let next = match (current, delta.is_negative()) {
        (Some(current), true) => current.saturating_sub(delta.unsigned_abs()),
        (Some(current), false) => {
            current.saturating_add(delta.cast_unsigned()).min(choices.len() - 1)
        }
        (None, true) => choices.len() - 1,
        (None, false) => 0,
    };
    state.message_actions.selected_id = choices.get(next).cloned();
}

enum MessageAction {
    Copy,
    Edit,
    Fork,
    Restore,
    Summarize,
    Rewind,
}

fn selected_intent(state: &mut TuiState, action: MessageAction) -> Option<TuiControllerCommand> {
    let selected_id = state.message_actions.selected_id.clone()?;
    let message = state.messages.iter().find(|message| message.base().id == selected_id)?;
    let intent = match action {
        MessageAction::Copy => MessageActionIntent::Copy {
            message_id: selected_id,
            text: visible_message_text(message),
        },
        MessageAction::Edit => {
            let (message_id, text) = editable_user_at_or_before(state, &selected_id)?;
            MessageActionIntent::Edit {
                message_id,
                text,
                cancel_active_turn: state.prompt.is_busy(),
            }
        }
        MessageAction::Fork => MessageActionIntent::Fork { message_id: selected_id },
        MessageAction::Restore => MessageActionIntent::Restore { message_id: selected_id },
        MessageAction::Summarize => MessageActionIntent::Summarize { message_id: selected_id },
        MessageAction::Rewind => MessageActionIntent::Rewind { message_id: selected_id },
    };
    Some(TuiControllerCommand::MessageAction(intent))
}

fn editable_user_at_or_before(
    state: &TuiState,
    selected_id: &UiMessageId,
) -> Option<(UiMessageId, String)> {
    let selected_index =
        state.messages.iter().position(|message| &message.base().id == selected_id)?;
    state.messages[..=selected_index].iter().rev().find_map(|message| {
        let UiMessage::User(message) = message else {
            return None;
        };
        Some((message.base.id.clone(), message.text.clone()))
    })
}

fn navigable_messages(state: &TuiState) -> Vec<(usize, &UiMessage)> {
    state
        .messages
        .iter()
        .enumerate()
        .filter(|(_, message)| match message {
            UiMessage::User(_)
            | UiMessage::ToolCall(_)
            | UiMessage::ToolResult(_)
            | UiMessage::Error(_)
            | UiMessage::Attachment(_) => true,
            UiMessage::Assistant(message) => !message.text.trim().is_empty(),
            UiMessage::System(message) => !message.text.trim().is_empty(),
            UiMessage::Thinking(_)
            | UiMessage::Step(_)
            | UiMessage::Progress(_)
            | UiMessage::CompactBoundary(_)
            | UiMessage::Unknown(_) => false,
        })
        .collect()
}

fn visible_message_text(message: &UiMessage) -> String {
    match message {
        UiMessage::User(message) => message.text.clone(),
        UiMessage::Assistant(message) => message.text.clone(),
        UiMessage::ToolCall(message) => {
            message.summary.clone().unwrap_or_else(|| format!("{} tool call", message.tool_name))
        }
        UiMessage::ToolResult(message) => message.content.clone(),
        UiMessage::Thinking(message) => {
            message.summary.clone().unwrap_or_else(|| "[hidden reasoning]".to_string())
        }
        UiMessage::Step(message) => format!("Step #{} {:?}", message.step_index, message.state),
        UiMessage::System(message) => message.text.clone(),
        UiMessage::Error(message) => message.message.clone(),
        UiMessage::Attachment(message) => message.name.clone(),
        UiMessage::Progress(message) => message.label.clone(),
        UiMessage::CompactBoundary(message) => message.summary.clone(),
        UiMessage::Unknown(message) => message.summary.clone(),
    }
}
