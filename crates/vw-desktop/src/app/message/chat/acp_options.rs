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
            if runtime.acp_options_state.set_model_input_draft(value, runtime.is_requesting) {
                app.invalidate_project_chat_preferences_read();
            }
            Task::none()
        }
        ChatMessage::AcpSourceRefresh => {
            let session_id = app.active_session_id.clone().unwrap_or_else(|| "__empty__".into());
            let directory = super::stream::session_directory_for_save(app, &session_id);
            let runtime = app.get_session_runtime_mut(&session_id);
            let Some(agent) = runtime.acp_agent.clone() else {
                return Task::none();
            };
            if runtime.acp_options_state.pending
                || runtime.acp_options_state.source_pending.is_some()
                || runtime.is_requesting
            {
                return Task::none();
            }
            let request_id = next_source_request_id();
            runtime.acp_options_state.source_request_id = Some(request_id);
            runtime.acp_options_state.pending = true;
            // A newly selected fork agent has no binding of its own yet. Reading the
            // inherited binding here would advertise the previous agent's capabilities.
            let draft_options = (session_id == "__empty__"
                || runtime.draft.is_some()
                || (runtime.fork_agent_selected
                    && !runtime.fork_request_submitted
                    && runtime.fork_execution_started == Some(false)))
            .then(|| runtime.acp_options_state.options.clone());
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        desktop_session::message::chat::acp_options::load_source_options(
                            &gateway_session::GatewaySessionClient::new(client.transport().clone()),
                            &session_id, directory.as_deref(), &agent, draft_options,
                        ).await
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
            if runtime.acp_agent.as_deref() != Some(&agent)
                || runtime.acp_options_state.source_request_id != Some(request_id)
                || runtime.acp_options_state.source_pending.is_some()
            {
                return Task::none();
            }
            runtime.acp_options_state.pending = false;
            runtime.acp_options_state.source_request_id = None;
            match result {
                Ok(options) => {
                    if runtime.acp_options_state.source_pending.is_none() && !runtime.is_requesting {
                        runtime.acp_model_source = options.acp_model_source;
                        if let Some(selection) = &options.selected_model {
                            runtime.acp_model_source = selection.source;
                            if let Some(model) = &selection.model {
                                runtime.model = match &selection.provider_id {
                                    Some(provider)
                                        if model != "auto"
                                            && !model.starts_with(&format!("{provider}/")) =>
                                    {
                                        format!("{provider}/{model}")
                                    }
                                    _ => model.clone(),
                                };
                                runtime.auto_model = model == "auto";
                            }
                        }
                        runtime.acp_options_state.options = options;
                    }
                    Task::none()
                }
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
            if session_id == "__empty__" && app.project_path != project_path {
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
            if runtime.acp_options_state.source_request_id.as_ref() != Some(&request_id)
                || runtime.acp_options_state.source_pending.as_ref() != Some(&update)
                || runtime.acp_agent.as_deref() != Some(&update.agent)
            {
                return Task::none();
            }
            runtime.acp_options_state.source_pending = None;
            runtime.acp_options_state.source_request_id = None;
            match result {
                Ok(options) => {
                    if options.acp_model_source != update.selection.source {
                        return app.show_error_toast("后端未确认所选模型来源");
                    }
                    let boundary = runtime.acp_model_source != options.acp_model_source;
                    runtime.acp_model_source = options.acp_model_source;
                    if let Some(model) =
                        options.selected_model.as_ref().and_then(|s| s.model.as_ref())
                    {
                        runtime.model = model.clone();
                        runtime.auto_model = model == "auto";
                        runtime.task_mode_model = model.clone();
                    }
                    runtime.acp_options_state.options = options;
                    runtime.acp_options_state.model_input_draft = None;
                    runtime.acp_rebuild_required = true;
                    runtime.restore_model_from_history = false;
                    let unstarted_fork = runtime.fork_execution_started == Some(false);
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
                || runtime.is_requesting
                || runtime.acp_options_state.pending
                || runtime.acp_options_state.source_pending.is_some()
            {
                return Task::none();
            }
            let selection = match message {
                ChatMessage::AcpOptionSelected(selection) => {
                    use vw_api_acp_model::acp_session::AcpOptionKind;
                    if matches!(&selection.kind, AcpOptionKind::Model)
                        || matches!(&selection.kind, AcpOptionKind::Config { id } if id == "model")
                    {
                        return app.show_localized_toast(
                            crate::app::state::ToastKind::Warning,
                            "会话开始后不能切换模型",
                            vec![],
                        );
                    }
                    if matches!(&selection.kind, AcpOptionKind::Mode)
                        || !runtime.acp_options_state.options.options.iter().any(|option| {
                            option.kind == selection.kind
                                && option.choices.iter().any(|choice| choice.id == selection.value)
                        })
                    {
                        return Task::none();
                    }
                    Some(selection)
                }
                _ => {
                    runtime.acp_options_state.open = true;
                    None
                }
            };
            runtime.acp_options_state.pending = true;
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        match selection {
                            Some(selection) => {
                                gateway_session::GatewaySessionClient::new(client.transport().clone())
                                    .acp_session_select(
                                        &session_id,
                                        directory.as_deref(),
                                        &selection,
                                    )
                                    .await
                            }
                            None => {
                                gateway_session::GatewaySessionClient::new(client.transport().clone()).acp_session_options(&session_id, directory.as_deref()).await
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

/// Keep the confirmed choice intact while validation/persistence runs on the gateway.
pub(super) fn select_source(
    app: &mut App,
    source: vw_api_acp_model::acp_model_source::AcpModelSource,
    model: Option<String>,
) -> Task<Message> {
    use vw_api_acp_model::acp_model_source::{
        AcpModelSource, AcpModelSourceUpdate, AcpSessionModelSelection,
    };
    let session_id = app.active_session_id.clone().unwrap_or_else(|| "__empty__".into());
    let directory = super::stream::session_directory_for_save(app, &session_id);
    let project_path = app.project_path.clone();
    let request_id = next_source_request_id();
    let runtime = app.get_session_runtime_mut(&session_id);
    let Some(agent) = runtime.acp_agent.clone() else {
        return Task::none();
    };
    if runtime.is_requesting || runtime.acp_options_state.pending || runtime.acp_options_state.source_pending.is_some()
    {
        return app.show_error_toast("请等待当前请求结束后切换模型来源");
    }
    if runtime.acp_options_state.options.model_source_capabilities.iter().any(|capability| {
        matches!(capability, vw_api_acp_model::acp_model_source::AcpModelSourceCapability::Unsupported { source: rejected, .. } if *rejected == source)
    }) {
        return app.show_error_toast("此智能体不支持所选模型来源");
    }
    let model = model
        .unwrap_or_else(|| if runtime.auto_model { "auto".into() } else { runtime.model.clone() });
    let update = AcpModelSourceUpdate {
        agent,
        selection: AcpSessionModelSelection {
            source,
            provider_id: None,
            model: (source == AcpModelSource::SessionModel).then_some(model),
        },
    };
    runtime.acp_options_state.source_pending = Some(update.clone());
    runtime.acp_options_state.source_request_id = Some(request_id);
    let draft_options =
        (session_id == "__empty__" || runtime.draft.is_some()).then(|| runtime.acp_options_state.options.clone());
    app.invalidate_project_chat_preferences_read();
    Task::perform(
        async move {
            let result = async {
                let client = crate::app::gateway_client()?;
                desktop_session::message::chat::acp_options::save_source_options(
                    &gateway_session::GatewaySessionClient::new(client.transport().clone()),
                    &session_id, directory.as_deref(), &update, draft_options,
                ).await
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
