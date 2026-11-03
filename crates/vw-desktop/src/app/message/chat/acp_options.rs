//! Commit controls to UI state only after the backend acknowledges and persists them.
use super::ChatMessage;
use crate::app::{App, Message};
use iced::Task;

pub(super) fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::AcpOptionsClose => {
            app.current_session_runtime_mut().acp_options_open = false;
            Task::none()
        }
        ChatMessage::AcpOptionsLoaded { session_id, result } => {
            let runtime = app.get_session_runtime_mut(&session_id);
            runtime.acp_options_pending = false;
            match result {
                Ok(options) => {
                    runtime.acp_options = options;
                    Task::none()
                }
                Err(error) => app.show_error_toast(error),
            }
        }
        ChatMessage::AcpOptionsOpen | ChatMessage::AcpOptionSelected(_) => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let directory = super::stream::session_directory_for_save(app, &session_id);
            let runtime = app.get_session_runtime_mut(&session_id);
            if runtime.acp_agent.is_none() || runtime.acp_options_pending {
                return Task::none();
            }
            let selection = match message {
                ChatMessage::AcpOptionSelected(selection) => {
                    if runtime.is_requesting
                        || !runtime.acp_options.connected
                        || !runtime.acp_options.options.iter().any(|option| {
                            option.kind == selection.kind
                                && option.choices.iter().any(|choice| choice.id == selection.value)
                        })
                    {
                        return Task::none();
                    }
                    Some(selection)
                }
                _ => {
                    runtime.acp_options_open = true;
                    None
                }
            };
            runtime.acp_options_pending = true;
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        match selection {
                            Some(selection) => {
                                client
                                    .acp_session_select(
                                        &session_id,
                                        directory.as_deref(),
                                        &selection,
                                    )
                                    .await
                            }
                            None => {
                                client.acp_session_options(&session_id, directory.as_deref()).await
                            }
                        }
                    }
                    .await;
                    ChatMessage::AcpOptionsLoaded { session_id, result }
                },
                Message::Chat,
            )
        }
        _ => Task::none(),
    }
}
