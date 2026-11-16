//! Commit controls to UI state only after the backend acknowledges and persists them.
use super::ChatMessage;
use crate::app::{App, Message};
use iced::Task;

fn next_source_request_id() -> u64 {
    desktop_session::message::chat::acp_options::next_source_request_id()
}

pub(super) fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::AcpModelInputDraftChanged(value) => {
            if app.current_session_has_started() {
                return Task::none();
            }
            let runtime = app.current_session_runtime_mut();
            if runtime
                .acp_options_state
                .set_model_input_draft(value, runtime.request_lifecycle.is_requesting)
            {
                app.invalidate_project_chat_preferences_read();
            }
            Task::none()
        }
        ChatMessage::AcpSourceRefresh => {
            let session_id = app.active_session_id.clone().unwrap_or_else(|| "__empty__".into());
            let directory = super::stream::session_directory_for_save(app, &session_id);
            let runtime = app.get_session_runtime_mut(&session_id);
            let Some((agent, request_id, draft_options)) =
                runtime.begin_acp_source_refresh(&session_id)
            else {
                return Task::none();
            };
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        desktop_session::message::chat::acp_options::load_source_options(
                            &gateway_session::GatewaySessionClient::new(client.transport().clone()),
                            &session_id,
                            directory.as_deref(),
                            &agent,
                            draft_options,
                        )
                        .await
                    }
                    .await;
                    ChatMessage::AcpSourceLoaded { session_id, request_id, agent, result }
                },
                Message::Chat,
            )
        }
        ChatMessage::AcpSourceLoaded { session_id, request_id, agent, result } => {
            let Some(runtime) = app.session_runtime_states.get_mut(&session_id) else {
                return Task::none();
            };
            match runtime.apply_acp_source_loaded(request_id, &agent, result) {
                Ok(()) => Task::none(),
                Err(error) => app.show_error_toast(error),
            }
        }

        ChatMessage::AcpModelSourceSaved {
            session_id,
            request_id,
            project_path,
            update,
            result,
        } => {
            if session_id == "__empty__" && app.navigation_tabs.project_path != project_path {
                if let Some(runtime) = app.session_runtime_states.get_mut(&session_id)
                    && runtime.acp_options_state.source_request_id == Some(request_id)
                {
                    runtime.acp_options_state.source_pending = None;
                    runtime.acp_options_state.source_request_id = None;
                }
                return Task::none();
            }
            let active = if session_id == "__empty__" {
                app.active_session_id.is_none()
            } else {
                app.active_session_id.as_deref() == Some(&session_id)
            };
            let Some(runtime) = app.session_runtime_states.get_mut(&session_id) else {
                return Task::none();
            };
            if !runtime.acp_options_state.source_save_matches(
                request_id,
                &update,
                runtime.acp_agent.as_deref(),
            ) {
                return Task::none();
            }
            match runtime.acp_options_state.apply_source_saved(&update, result) {
                Ok(confirmed) => {
                    let (boundary, unstarted_fork) = runtime.confirm_acp_source_saved(confirmed);
                    app.sync_active_session_preferences();
                    if boundary && active {
                        app.show_info_toast(if unstarted_fork {
                            "模型来源已保存；首次发送将保留分叉历史"
                        } else {
                            "模型来源已保存；下一轮创建新的 ACP 会话，不继承原生会话上下文"
                        })
                    } else {
                        Task::none()
                    }
                }
                Err(error) => app.show_error_toast(error),
            }
        }
        ChatMessage::AcpOptionsClose => {
            app.current_session_runtime_mut().acp_options_state.open = false;
            Task::none()
        }
        ChatMessage::AcpOptionsLoaded { session_id, result } => {
            let Some(runtime) = app.session_runtime_states.get_mut(&session_id) else {
                return Task::none();
            };
            match runtime.acp_options_state.apply_loaded_options(result) {
                Ok(()) => Task::none(),
                Err(error) => app.show_error_toast(error),
            }
        }

        ChatMessage::AcpOptionsOpen | ChatMessage::AcpOptionSelected(_) => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let directory = super::stream::session_directory_for_save(app, &session_id);
            let runtime = app.get_session_runtime_mut(&session_id);
            if runtime.acp_agent.is_none()
                || runtime.request_lifecycle.is_requesting
                || runtime.acp_options_state.pending
                || runtime.acp_options_state.source_pending.is_some()
            {
                return Task::none();
            }
            let selection = match message {
                ChatMessage::AcpOptionSelected(selection) => Some(selection),
                _ => None,
            };
            match runtime.acp_options_state.begin_option(selection.as_ref()) {
                Ok(true) => {}
                Ok(false) => return Task::none(),
                Err(source) => {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Warning,
                        source,
                        vec![],
                    );
                }
            }
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        desktop_session::message::chat::acp_options::load_or_select_options(
                            &gateway_session::GatewaySessionClient::new(client.transport().clone()),
                            &session_id,
                            directory.as_deref(),
                            selection.as_ref(),
                        )
                        .await
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

/// Keep the confirmed choice intact while validation/persistence runs on the gateway.
pub(super) fn select_source(
    app: &mut App,
    source: vw_api_acp_model::acp_model_source::AcpModelSource,
    model: Option<String>,
) -> Task<Message> {
    let session_id = app.active_session_id.clone().unwrap_or_else(|| "__empty__".into());
    let directory = super::stream::session_directory_for_save(app, &session_id);
    let project_path = app.navigation_tabs.project_path.clone();
    let request_id = next_source_request_id();
    let runtime = app.get_session_runtime_mut(&session_id);
    let Some(agent) = runtime.acp_agent.clone() else {
        return Task::none();
    };
    let model = model
        .unwrap_or_else(|| if runtime.auto_model { "auto".into() } else { runtime.model.clone() });
    let update = match runtime.acp_options_state.begin_source(
        request_id,
        agent,
        source,
        model,
        runtime.request_lifecycle.is_requesting,
    ) {
        Ok(update) => update,
        Err(error) => return app.show_error_toast(error),
    };
    let draft_options = (session_id == "__empty__" || runtime.draft.is_some())
        .then(|| runtime.acp_options_state.options.clone());
    app.invalidate_project_chat_preferences_read();
    Task::perform(
        async move {
            let result = async {
                let client = crate::app::gateway_client()?;
                desktop_session::message::chat::acp_options::save_source_options(
                    &gateway_session::GatewaySessionClient::new(client.transport().clone()),
                    &session_id,
                    directory.as_deref(),
                    &update,
                    draft_options,
                )
                .await
            }
            .await;
            ChatMessage::AcpModelSourceSaved {
                session_id,
                request_id,
                project_path,
                update,
                result,
            }
        },
        Message::Chat,
    )
}
