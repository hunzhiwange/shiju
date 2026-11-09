//! Commit controls to UI state only after the backend acknowledges and persists them.
use super::ChatMessage;
use crate::app::{App, Message};
use iced::Task;

pub(super) fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::AcpModelInputDraftChanged(value) => {
            if app.current_session_has_started() {
                return Task::none();
            }
            let runtime = app.current_session_runtime_mut();
            if !runtime.is_requesting && runtime.acp_source_pending.is_none() {
                runtime.acp_model_input_draft = Some(value);
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
            if runtime.acp_options_pending
                || runtime.acp_source_pending.is_some()
                || runtime.is_requesting
            {
                return Task::none();
            }
            runtime.acp_options_pending = true;
            let draft_options = (session_id == "__empty__" || runtime.draft.is_some())
                .then(|| runtime.acp_options.clone());
            Task::perform(
                async move {
                    let result = async {
                        let client = crate::app::gateway_client()?;
                        let mut options = match draft_options {
                            Some(options) => options,
                            None => {
                                client
                                    .acp_session_options(&session_id, directory.as_deref())
                                    .await?
                            }
                        };
                        if options.model_source_capabilities.is_empty() {
                            let capabilities = client.acp_model_source_capabilities(&agent).await?;
                            if options.selected_model.is_none() && !options.connected {
                                options.acp_model_source = capabilities.acp_model_source;
                            }
                            options.model_source_capabilities =
                                capabilities.model_source_capabilities;
                            options.verified_cli_release = capabilities.verified_cli_release;
                            options.verified_platform = capabilities.verified_platform;
                        }
                        Ok(options)
                    }
                    .await;
                    ChatMessage::AcpSourceLoaded { session_id, agent, result }
                },
                Message::Chat,
            )
        }
        ChatMessage::AcpSourceLoaded { session_id, agent, result } => {
            let Some(runtime) = app.session_runtime_states.get_mut(&session_id) else {
                return Task::none();
            };
            if runtime.acp_agent.as_deref() != Some(&agent) {
                return Task::none();
            }
            runtime.acp_options_pending = false;
            match result {
                Ok(options) => {
                    if runtime.acp_source_pending.is_none() && !runtime.is_requesting {
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
                        runtime.acp_options = options;
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
                    && runtime.acp_source_request_id == Some(request_id)
                {
                    runtime.acp_source_pending = None;
                    runtime.acp_source_request_id = None;
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
            if runtime.acp_source_request_id.as_ref() != Some(&request_id)
                || runtime.acp_source_pending.as_ref() != Some(&update)
                || runtime.acp_agent.as_deref() != Some(&update.agent)
            {
                return Task::none();
            }
            runtime.acp_source_pending = None;
            runtime.acp_source_request_id = None;
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
                    runtime.acp_options = options;
                    runtime.acp_model_input_draft = None;
                    runtime.acp_rebuild_required = true;
                    runtime.restore_model_from_history = false;
                    app.sync_active_session_preferences();
                    if boundary && active {
                        app.show_info_toast(
                            "模型来源已保存；下一轮创建新的 ACP 会话，不继承原生会话上下文",
                        )
                    } else {
                        Task::none()
                    }
                }
                Err(error) => app.show_error_toast(error),
            }
        }
        ChatMessage::AcpOptionsClose => {
            app.current_session_runtime_mut().acp_options_open = false;
            Task::none()
        }
        ChatMessage::AcpOptionsLoaded { session_id, result } => {
            let Some(runtime) = app.session_runtime_states.get_mut(&session_id) else {
                return Task::none();
            };
            runtime.acp_options_pending = false;
            match result {
                Ok(options) => {
                    runtime.acp_options = options;
                    Task::none()
                }
                Err(error) => {
                    // A failed control must not leave an earlier native success on screen.
                    if let Some(status) = runtime.acp_options.access_status.as_mut() {
                        status.support =
                            vw_gateway_client::vw_api_types::acp_session::AcpAccessSupport::Failed;
                        status.explanation =
                            "会话选项切换或读取失败，请刷新或重试；当前能力尚未确认".into();
                    }
                    app.show_error_toast(error)
                }
            }
        }
        ChatMessage::AcpOptionsOpen | ChatMessage::AcpOptionSelected(_) => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let directory = super::stream::session_directory_for_save(app, &session_id);
            let runtime = app.get_session_runtime_mut(&session_id);
            if runtime.acp_agent.is_none()
                || runtime.acp_options_pending
                || runtime.acp_source_pending.is_some()
            {
                return Task::none();
            }
            let selection = match message {
                ChatMessage::AcpOptionSelected(selection) => {
                    use vw_gateway_client::vw_api_types::acp_session::AcpOptionKind;
                    if matches!(&selection.kind, AcpOptionKind::Model)
                        || matches!(&selection.kind, AcpOptionKind::Config { id } if id == "model")
                    {
                        return app.show_localized_toast(
                            crate::app::state::ToastKind::Warning,
                            "会话开始后不能切换模型",
                            vec![],
                        );
                    }
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

/// Keep the confirmed choice intact while validation/persistence runs on the gateway.
pub(super) fn select_source(
    app: &mut App,
    source: vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource,
    model: Option<String>,
) -> Task<Message> {
    use vw_gateway_client::vw_api_types::acp_model_source::{
        AcpModelSource, AcpModelSourceCapability, AcpModelSourceUpdate, AcpSessionModelSelection,
    };
    let session_id = app.active_session_id.clone().unwrap_or_else(|| "__empty__".into());
    let directory = super::stream::session_directory_for_save(app, &session_id);
    let project_path = app.project_path.clone();
    static NEXT_REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let request_id = NEXT_REQUEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let runtime = app.get_session_runtime_mut(&session_id);
    let Some(agent) = runtime.acp_agent.clone() else {
        return Task::none();
    };
    if runtime.is_requesting || runtime.acp_options_pending || runtime.acp_source_pending.is_some()
    {
        return app.show_error_toast("请等待当前请求结束后切换模型来源");
    }
    if runtime.acp_options.model_source_capabilities.iter().any(|capability| {
        matches!(capability, vw_gateway_client::vw_api_types::acp_model_source::AcpModelSourceCapability::Unsupported { source: rejected, .. } if *rejected == source)
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
    runtime.acp_source_pending = Some(update.clone());
    runtime.acp_source_request_id = Some(request_id);
    let draft_options =
        (session_id == "__empty__" || runtime.draft.is_some()).then(|| runtime.acp_options.clone());
    app.invalidate_project_chat_preferences_read();
    Task::perform(
        async move {
            let result = async {
                let client = crate::app::gateway_client()?;
                let directory = directory.as_deref().ok_or("请先选择项目目录")?;
                if let Some(mut options) = draft_options {
                    if options.model_source_capabilities.is_empty() {
                        let capabilities = client.acp_model_source_capabilities(&update.agent).await?;
                        options.model_source_capabilities = capabilities.model_source_capabilities;
                        options.verified_cli_release = capabilities.verified_cli_release;
                        options.verified_platform = capabilities.verified_platform;
                    }
                    if !options.model_source_capabilities.iter().any(|capability| {
                        matches!(capability, AcpModelSourceCapability::Supported { source: supported } if *supported == source)
                    }) {
                        return Err("此智能体不支持所选模型来源".to_string());
                    }
                    options.acp_model_source = source;
                    options.selected_model = Some(update.selection.clone());
                    return Ok(options);
                }
                client.acp_session_model_source(&session_id, Some(directory), &update).await
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
