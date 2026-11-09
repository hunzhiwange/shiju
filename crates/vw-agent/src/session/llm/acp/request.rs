//! ACP 流式请求执行入口。
//!
//! 本模块负责把一次 VibeWindow LLM 请求映射为 ACP prompt 运行：解析代理配置、
//! 查找或创建 ACP 会话、转发 ACP 输出事件，并在会话变化等可恢复错误上执行一次明确重试。

use serde_json::Value;
use tokio::sync::mpsc;
use vw_acp::{
    AcpJsonRpcMessage, PromptRequest, SessionStrategy, apply_lifecycle_snapshot_to_record,
    write_session_record,
};

use crate::app::agent::auth;
use crate::app::agent::config;
use crate::app::agent::provider::provider;
use crate::app::agent::session::{message, ui_types};

use super::config::{lookup_acp_command, normalize_acp_agent_config, parse_acp_options};
use super::replay::{build_request_prompt, parse_recent_count, parse_replay_strategy};
use super::session::{
    acp_session_name, close_cached_acp_client_if_unshared, find_or_create_cached_session_record,
    get_cached_acp_client, should_abort,
};
use super::updates::forward_acp_message;
use super::{
    CachedAcpClientEntry, Error, StreamEvent, acp_option_error, is_acp_session_changed_error,
    to_api_error,
};

/// 执行一次 ACP 流式请求并把结果转发给上层事件消费者。
///
/// 参数说明：
/// - `model`：当前模型配置，用于解析对应 ACP 代理。
/// - `merged_options`：运行时选项，包含 cwd、ACP 权限、会话模式和历史策略等。
/// - `chat_messages`：OpenAI 风格的本地会话消息。
/// - `session_id`：VibeWindow 本地会话 id，用于生成稳定 ACP 会话名。
/// - `abort`：可选取消信号；触发后会尽力取消 ACP 端会话。
/// - `on_event`：流式事件回调，接收文本增量、推理增量、完成和错误事件。
///
/// 返回值为 `Ok(())` 表示请求完成并已发送 `Done` 事件；错误会保留为上层统一
/// `Error`。配置缺失、空 prompt、ACP 调用失败和取消都会作为错误返回。
pub(crate) async fn do_stream_request_acp(
    model: &provider::Model,
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    merged_options: &Value,
    chat_messages: &Value,
    session_id: &str,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut impl FnMut(StreamEvent),
) -> Result<(), Error> {
    if should_abort(abort) {
        tracing::debug!(
            target: "vw_agent",
            model = %model.api.id,
            "ACP stream aborted before request start"
        );
        on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
            message: "aborted".to_string(),
        }));
        return Err(Error::Aborted);
    }

    let cwd = merged_options
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| to_api_error("failed to resolve cwd for acp session"))?;
    let cwd_text = cwd.to_string_lossy().into_owned();
    let _session_guard = super::lifecycle::try_lock(session_id, &cwd_text).map_err(to_api_error)?;
    let selection_store =
        super::lifecycle::Store::open(session_id, &cwd_text).map_err(to_api_error)?;
    let previous_selection = selection_store.read().map_err(to_api_error)?;
    let source =
        super::super::prompt::acp_model_source(merged_options).map_err(super::acp_option_error)?;
    if source == vw_api_types::acp_model_source::AcpModelSource::SessionModel {
        validate_session_model(model, provider_info, auth_info)?;
    }
    let cfg = config::get_current().await.map_err(to_api_error)?;
    let (acp_agent_name, acp_cfg) =
        lookup_acp_command(&cfg, model, merged_options).ok_or_else(|| {
            tracing::warn!(
                target: "vw_agent",
                model = %model.api.id,
                requested_acp_agent = merged_options
                    .get("acp_agent")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default(),
                "ACP command not configured"
            );
            Error::Api(message::AssistantError::Unknown {
                message: format!("acp command not configured for model {}", model.api.id),
            })
        })?;
    super::skill_mounts::reject_expanded_invocations(&acp_agent_name, chat_messages)?;
    let selected_model = vw_api_types::acp_model_source::AcpSessionModelSelection {
        source,
        provider_id: (source == vw_api_types::acp_model_source::AcpModelSource::SessionModel)
            .then(|| model.provider_id.clone()),
        model: (source == vw_api_types::acp_model_source::AcpModelSource::SessionModel)
            .then(|| model.id.clone()),
    };
    let locked_selection = previous_selection.as_ref().filter(|selection| selection.has_started());
    if locked_selection
        .is_some_and(|old| old.agent != acp_agent_name || old.model != selected_model)
    {
        return Err(to_api_error("changing a session agent or model requires a new session"));
    }
    let source_changed = previous_selection
        .as_ref()
        .is_some_and(|old| old.model.source != source || old.agent != acp_agent_name);
    let selected_generation = previous_selection
        .as_ref()
        .map_or(0, |old| old.generation)
        .checked_add(u64::from(source_changed))
        .ok_or_else(|| to_api_error("ACP source generation exhausted"))?;
    let identity = super::lifecycle::session_identity(session_id, &cwd_text);
    let mut normalized_acp_cfg = normalize_acp_agent_config(&acp_agent_name, &acp_cfg);
    // Older model selection writes could clear the anchor of a running session.
    let recovered = if previous_selection.is_some() && locked_selection.is_none() {
        let launch = match source {
            vw_api_types::acp_model_source::AcpModelSource::CliLocal => {
                super::recovery::Launch::Local { agent: &acp_agent_name, config: &normalized_acp_cfg }
            }
            vw_api_types::acp_model_source::AcpModelSource::SessionModel => {
                let project = crate::project::instance::project()
                    .ok_or_else(|| to_api_error("ACP launch requires project context"))?;
                super::recovery::Launch::Managed(super::model_source::launch_identity(
                    project.id,
                    &acp_agent_name,
                    model,
                    "",
                ))
            }
        };
        super::recovery::recover_session(
            session_id,
            &identity,
            &cwd_text,
            selected_generation,
            &launch,
        )
        .await
        .map_err(to_api_error)?
    } else {
        None
    };
    let generation = recovered.as_ref().map_or(selected_generation, |session| session.generation);
    let scoped_session = format!("{session_id}:acp-source-{generation}:{identity}");
    let mut parsed_options =
        parse_acp_options(merged_options, &acp_agent_name, &normalized_acp_cfg)?;
    let mut cfg = cfg;
    crate::session::agent_binding::apply_to_config(session_id, &mut cfg)
        .await
        .map_err(to_api_error)?;
    let chat_messages = super::config::apply_agent_defaults(
        &cfg,
        merged_options,
        &acp_agent_name,
        &cwd,
        &mut parsed_options,
        chat_messages,
    )
    .await?;
    parsed_options.runtime_identity = Some(scoped_session.clone());
    parsed_options.skill_view = super::skill_mounts::MountedSkills::prepare(
        &parsed_options.skill_mounts,
        &cwd,
        &scoped_session,
        &cfg.skills.security,
    )?;
    if acp_agent_name == "openclaw" {
        super::skill_mounts::configure_native_scanner(
            &acp_agent_name,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
        if source == vw_api_types::acp_model_source::AcpModelSource::CliLocal
            && parsed_options.skill_view.is_some()
        {
            return Err(acp_option_error(
                "OpenClaw skill mounts require an owned session-model Gateway; cli_local Gateway isolation is unverified",
            ));
        }
    }
    super::skill_mounts::prepare_supplier(&acp_agent_name, parsed_options.skill_view.as_ref())?;
    super::skill_mounts::validate(&acp_agent_name, !parsed_options.skill_mounts.is_empty())?;

    let previous_native = if source == vw_api_types::acp_model_source::AcpModelSource::CliLocal
        && (recovered.is_some() || locked_selection.is_some())
    {
        let state = if let Some(recovered) = &recovered {
            recovered.record.vwacp.clone()
        } else if let Some(previous) = locked_selection {
            vw_acp::resolve_session_record(&previous.record_id).await.map_err(to_api_error)?.vwacp
        } else {
            None
        };
        let model = vw_acp::get_desired_model_id(state.as_ref());
        parsed_options.session_options.get_or_insert_with(Default::default).model = model;
        state
    } else {
        None
    };

    // The profile owns the relay, token and private files for this request only.
    // CLI-local execution preserves the user's original configuration.
    let launch_profile = match source {
        vw_api_types::acp_model_source::AcpModelSource::SessionModel => {
            let cancellation = async {
                if let Some(mut receiver) = abort.cloned() {
                    loop {
                        if *receiver.borrow() {
                            return;
                        }
                        if receiver.changed().await.is_err() {
                            break;
                        }
                    }
                }
                std::future::pending::<()>().await;
            };
            // Dropping preparation also drops its owned Gateway during startup.
            let profile = tokio::select! {
                result = super::model_source::prepare(&acp_agent_name, &normalized_acp_cfg, model, &scoped_session) => result.map_err(to_api_error)?,
                () = cancellation => {
                    on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError { message: "aborted".to_string() }));
                    return Err(Error::Aborted);
                }
            };
            Some(profile)
        }
        vw_api_types::acp_model_source::AcpModelSource::CliLocal => None,
    };
    let mut normalized_acp_cfg = launch_profile
        .as_ref()
        .map(|profile| profile.config().clone())
        .unwrap_or(normalized_acp_cfg);
    let command = normalized_acp_cfg.command.trim();
    if command.is_empty() {
        tracing::warn!(
            target: "vw_agent",
            model = %model.api.id,
            acp_agent = %acp_agent_name,
            "ACP command is empty"
        );
        return Err(Error::Api(message::AssistantError::Unknown {
            message: format!("acp command is empty for model {}", model.api.id),
        }));
    }

    // A desktop rebuild request must not replace the history we just recovered.
    let requested_force_new_session = recovered.is_none()
        && merged_options.get("acp_force_new_session").and_then(Value::as_bool).unwrap_or(false);
    let replay_strategy = parse_replay_strategy(merged_options);
    let replay_recent_count = parse_recent_count(merged_options);
    let initial_prompt_len = build_request_prompt(
        &chat_messages,
        requested_force_new_session,
        replay_strategy,
        replay_recent_count,
    )
    .len();
    tracing::info!(
        target: "vw_agent",
        model = %model.api.id,
        acp_agent = %acp_agent_name,
        command,
        args_count = normalized_acp_cfg.args.len(),
        prompt_len = initial_prompt_len,
        "resolved ACP request configuration"
    );
    let acp_agent_name = acp_agent_name.to_string();
    if acp_agent_name != "openclaw" {
        super::skill_mounts::configure_native_scanner(
            &acp_agent_name,
            &mut normalized_acp_cfg,
            parsed_options.skill_view.as_ref(),
        )?;
    }
    let close_client_after_request =
        parse_close_client_after_request(merged_options)? || launch_profile.is_some();
    let cwd_text = cwd.to_string_lossy().into_owned();
    let walk_boundary = vw_acp::find_git_repository_root(&cwd_text)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd_text.clone());
    let session_name = acp_session_name(&scoped_session);
    // Managed launches must never replay an advertised native model/config over the Provider.
    if source == vw_api_types::acp_model_source::AcpModelSource::SessionModel {
        if let Some(options) = parsed_options.session_options.as_mut() {
            options.model = None;
        }
    }
    let mut force_new_session = requested_force_new_session;
    let mut recovered_session_change = false;
    loop {
        let prompt = super::content::build_request_content(
            &chat_messages,
            force_new_session,
            replay_strategy,
            replay_recent_count,
        )
        .map_err(to_api_error)?;
        if prompt.is_empty() {
            tracing::warn!(
                target: "vw_agent",
                model = %model.api.id,
                acp_agent = %acp_agent_name,
                "ACP prompt is empty"
            );
            return Err(Error::Api(message::AssistantError::Unknown {
                message: "acp prompt is empty".to_string(),
            }));
        }

        if let Some(view) = &parsed_options.skill_view {
            view.0.validate().map_err(to_api_error)?;
        }
        super::session::stop_incompatible_cached_clients(
            &acp_agent_name,
            &normalized_acp_cfg,
            &cwd,
            &parsed_options,
        )
        .await?;
        let cached_client_entry =
            get_cached_acp_client(&acp_agent_name, &normalized_acp_cfg, &cwd, &parsed_options);
        let _request_cleanup = super::session::RequestClientCleanup::new(&cached_client_entry);
        let cached_client = &cached_client_entry.client;
        // 同一个 ACP 客户端可能被多个本地请求复用；串行化 prompt 可以避免代理端
        // 把并发输入混到同一会话流里。
        let prompt_guard = cached_client.prompt_lock.lock().await;
        let record = if let Some(recovered) = &recovered {
            Ok(recovered.record.clone())
        } else {
            find_or_create_cached_session_record(
                cached_client.as_ref(),
                &acp_agent_name,
                &normalized_acp_cfg,
                &cwd,
                &walk_boundary,
                session_name.clone(),
                &parsed_options,
                force_new_session,
            )
            .await
        };
        let mut record = match record {
            Ok(record) => record,
            Err(err) => {
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                return Err(err);
            }
        };

        let bound_model_id =
            launch_profile.as_ref().and_then(|profile| profile.session_model_id.as_deref());
        if let Some(model_id) = bound_model_id {
            // Managed launch identity takes precedence over a saved native selection.
            vw_acp::set_desired_model_id(&mut record, Some(model_id));
        }
        // Capture the non-elevated preference before replaying controls. Automatic
        // native access sync must never become the saved user preference.
        if vw_acp::get_desired_mode_id(record.vwacp.as_ref()).is_none() {
            vw_acp::set_desired_mode_id(&mut record, parsed_options.session_mode.as_deref());
        }
        let saved_config = vw_acp::get_desired_config_options(record.vwacp.as_ref());
        for (id, value) in &parsed_options.session_config_options {
            if !saved_config.contains_key(id)
                && matches!(id.as_str(), "mode" | "edit_approval_policy")
            {
                vw_acp::set_desired_config_option(&mut record, id, value);
            }
        }
        super::controls::prepare_access_restore(&cached_client.client, &mut record);
        let acp_session_id = record.acp_session_id.clone();
        if !super::controls::has_saved_config_category(
            &record,
            agent_client_protocol::SessionConfigOptionCategory::Mode,
        ) && let Some(mode_id) = vw_acp::get_desired_mode_id(record.vwacp.as_ref())
            .or_else(|| parsed_options.session_mode.clone())
        {
            if let Err(err) = apply_default_mode(
                &cached_client.client,
                &acp_session_id,
                &cwd,
                cached_client.client.restored_permission_value("mode", &mode_id).to_owned(),
            )
            .await
            {
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                return Err(err);
            }
        }
        if (bound_model_id.is_some()
            || (launch_profile.is_none()
                && !super::controls::has_saved_config_category(
                    &record,
                    agent_client_protocol::SessionConfigOptionCategory::Model,
                )))
            && let Some(model_id) =
                vw_acp::get_desired_model_id(record.vwacp.as_ref()).or_else(|| {
                    parsed_options
                        .session_options
                        .as_ref()
                        .and_then(|options| options.model.clone())
                })
            && !cached_client
                .client
                .session_capabilities(&acp_session_id)
                .and_then(|info| info.models)
                .is_some_and(|models| models.current_model_id.0.as_ref() == model_id)
        {
            if let Err(err) = cached_client
                .client
                .set_session_model(acp_session_id.clone(), &cwd, model_id)
                .await
                .map_err(to_api_error)
            {
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                return Err(err);
            }
        }
        let mut config_options: std::collections::BTreeMap<_, _> =
            parsed_options.session_config_options.iter().cloned().collect();
        if super::controls::has_saved_config_category(
            &record,
            agent_client_protocol::SessionConfigOptionCategory::ThoughtLevel,
        ) {
            config_options.retain(|id, _| {
                !matches!(id.as_str(), "reasoning_effort" | "thought_level" | "effort")
            });
        }
        let info = cached_client.client.session_capabilities(&acp_session_id);
        let native_config = |id: &str| {
            info.as_ref().and_then(|info| info.config_options.as_ref()).and_then(|options| {
                options.iter().find(|option| {
                    option.id.0.as_ref() == id && super::controls::is_model_config(option)
                })
            })
        };
        if locked_selection.is_some() || recovered.is_some() {
            config_options.retain(|id, _| id != "model" && native_config(id).is_none());
        }
        config_options.extend(vw_acp::get_desired_config_options(record.vwacp.as_ref()));
        if let Some(state) = previous_native.as_ref() {
            for option in state
                .config_options
                .iter()
                .flatten()
                .filter(|option| super::controls::is_model_config(option))
            {
                let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind else {
                    continue;
                };
                let id = option.id.to_string();
                let value = state
                    .desired_config_options
                    .as_ref()
                    .and_then(|values| values.get(&id))
                    .cloned()
                    .unwrap_or_else(|| select.current_value.to_string());
                config_options.insert(id, value);
            }
        }
        if launch_profile.is_some() {
            config_options.retain(|id, _| id != "model" && native_config(id).is_none());
        }
        for (config_id, value) in config_options {
            if let Some(option) = native_config(&config_id)
                && let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind
                && select.current_value.0.as_ref() == value
            {
                continue;
            }
            let value =
                cached_client.client.restored_permission_value(&config_id, &value).to_owned();
            if let Err(err) = cached_client
                .client
                .set_session_config_option(acp_session_id.clone(), &cwd, config_id, value)
                .await
                .map_err(to_api_error)
            {
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                return Err(err);
            }
        }

        if launch_profile.is_none()
            && let Some(info) = cached_client.client.session_capabilities(&acp_session_id)
        {
            if vw_acp::get_desired_model_id(record.vwacp.as_ref()).is_none()
                && let Some(models) = info.models.as_ref()
                && !info.config_options.iter().flatten().any(super::controls::is_model_config)
            {
                vw_acp::set_desired_model_id(&mut record, Some(models.current_model_id.0.as_ref()));
            }
            for option in info
                .config_options
                .iter()
                .flatten()
                .filter(|option| super::controls::is_model_config(option))
            {
                if let agent_client_protocol::SessionConfigKind::Select(select) = &option.kind {
                    vw_acp::set_desired_config_option(
                        &mut record,
                        option.id.0.as_ref(),
                        select.current_value.0.as_ref(),
                    );
                }
            }
            vw_acp::sync_config_options(&mut record, info.config_options);
        }
        apply_lifecycle_snapshot_to_record(
            &mut record,
            &cached_client.client.get_agent_lifecycle_snapshot(),
        );
        if let Err(err) = write_session_record(&record).await.map_err(to_api_error) {
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            drop(prompt_guard);
            return Err(err);
        }

        let selection = super::lifecycle::Selection {
            agent: acp_agent_name.clone(),
            model: selected_model.clone(),
            generation,
            record_id: record.vwacp_record_id.clone(),
        };
        if recovered.is_none()
            && let Err(error) = selection_store.commit(&selection)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        let (acp_message_tx, mut acp_message_rx) = {
            let (tx, rx) = mpsc::unbounded_channel::<AcpJsonRpcMessage>();
            (tx, rx)
        };
        // output_tx 只在本轮请求期间暴露给 ACP 输出回调。请求结束或取消时立即清空，
        // 避免后续代理消息被投递到已经失效的消费者。
        *cached_client.output_tx.lock() = Some(acp_message_tx.clone());

        let prompt_client = cached_client.client.clone();
        let prompt_cwd = cwd.clone();
        let prompt_text = prompt.clone();
        let prompt_session_id = record.acp_session_id.clone();
        let mut acp_task = tokio::spawn(async move {
            let mut on_prompt_event = |_event: vw_acp::PromptEvent| {};
            prompt_client
                .run_prompt(
                    PromptRequest::from_content(prompt_cwd, prompt_text)
                        // 历史由 ACP 对端保存；恢复失败不能静默换成没有历史的新会话。
                        .with_session_strategy(SessionStrategy::ResumeOrLoad(prompt_session_id)),
                    &mut on_prompt_event,
                )
                .await
                .map_err(to_api_error)
        });

        let mut acp_error: Option<Error> = None;
        let mut delta_count = 0usize;
        let mut latest_usage = ui_types::TokenUsage::default();
        let mut finish_reason = None;
        let mut abort_rx = abort.cloned();
        loop {
            if should_abort(abort) {
                *cached_client.output_tx.lock() = None;
                let _ = cached_client.client.cancel(record.acp_session_id.clone()).await;
                acp_task.abort();
                let _ = acp_task.await;
                on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
                    message: "aborted".to_string(),
                }));
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                return Err(Error::Aborted);
            }
            if let Some(abort_rx) = abort_rx.as_mut() {
                tokio::select! {
                    changed = abort_rx.changed() => {
                        match changed {
                            Ok(_) if *abort_rx.borrow() => {
                                *cached_client.output_tx.lock() = None;
                                let _ = cached_client.client.cancel(record.acp_session_id.clone()).await;
                                acp_task.abort();
                                let _ = acp_task.await;
                                on_event(StreamEvent::Error(message::AssistantError::MessageAbortedError {
                                    message: "aborted".to_string(),
                                }));
                                close_client_after_request_if_needed(
                                    close_client_after_request,
                                    &cached_client_entry,
                                )
                                .await;
                                drop(prompt_guard);
                                return Err(Error::Aborted);
                            }
                            Ok(_) | Err(_) => {}
                        }
                    }
                    maybe_message = acp_message_rx.recv() => {
                        if let Some(message) = maybe_message {
                            forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count);
                        }
                    }
                    joined = &mut acp_task => {
                        *cached_client.output_tx.lock() = None;
                        match joined {
                            Ok(Ok(result)) => {
                                finish_reason = result.finish_reason.clone();
                                if let Some(usage) = result.usage {
                                    latest_usage = ui_types::TokenUsage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                        cached_tokens: usage.cached_tokens,
                                        reasoning_tokens: usage.reasoning_tokens,
                                    };
                                }
                                if result.session_id != record.acp_session_id {
                                    record.acp_session_id = result.session_id;
                                }
                            }
                            Ok(Err(err)) => {
                                acp_error = Some(to_api_error(err));
                            }
                            Err(err) => {
                                let api_err = to_api_error(err.to_string());
                                acp_error = Some(api_err);
                            }
                        }
                        while let Ok(message) = acp_message_rx.try_recv() {
                            forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count);
                        }
                        break;
                    }
                }
            } else {
                tokio::select! {
                    maybe_message = acp_message_rx.recv() => {
                        if let Some(message) = maybe_message {
                            forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count);
                        }
                    }
                    joined = &mut acp_task => {
                        *cached_client.output_tx.lock() = None;
                        match joined {
                            Ok(Ok(result)) => {
                                finish_reason = result.finish_reason.clone();
                                if let Some(usage) = result.usage {
                                    latest_usage = ui_types::TokenUsage {
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                        cached_tokens: usage.cached_tokens,
                                        reasoning_tokens: usage.reasoning_tokens,
                                    };
                                }
                                if result.session_id != record.acp_session_id {
                                    record.acp_session_id = result.session_id;
                                }
                            }
                            Ok(Err(err)) => {
                                acp_error = Some(to_api_error(err));
                            }
                            Err(err) => {
                                let api_err = to_api_error(err.to_string());
                                acp_error = Some(api_err);
                            }
                        }
                        while let Ok(message) = acp_message_rx.try_recv() {
                            forward_acp_message(&message, on_event, &mut latest_usage, &mut delta_count);
                        }
                        break;
                    }
                }
            }
        }

        *cached_client.output_tx.lock() = None;
        if let Some(err) = acp_error {
            if !recovered_session_change && is_acp_session_changed_error(&err) {
                // 会话漂移不是丢弃历史的授权。只重试一次恢复原记录，失败明确上抛。
                recovered_session_change = true;
                force_new_session = false;
                tracing::warn!(
                    target: "vw_agent",
                    model = %model.api.id,
                    acp_agent = %acp_agent_name,
                    "ACP session changed during prompt; retrying the original session"
                );
                close_client_after_request_if_needed(
                    close_client_after_request,
                    &cached_client_entry,
                )
                .await;
                drop(prompt_guard);
                continue;
            }
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            drop(prompt_guard);
            return Err(err);
        }

        let now = vw_acp::iso_now();
        record.last_used_at = now.clone();
        record.last_prompt_at = Some(now);
        record.closed = Some(false);
        apply_lifecycle_snapshot_to_record(
            &mut record,
            &cached_client.client.get_agent_lifecycle_snapshot(),
        );
        if let Err(err) = write_session_record(&record).await.map_err(to_api_error) {
            close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
                .await;
            drop(prompt_guard);
            return Err(err);
        }

        // Keep the original recovery path available if the CLI refuses to load
        // the saved session or the user cancels the first recovery attempt.
        if recovered.is_some()
            && let Err(error) = selection_store.commit(&selection)
        {
            close_client_after_request_if_needed(true, &cached_client_entry).await;
            return Err(to_api_error(error));
        }

        on_event(StreamEvent::Done { finish_reason, usage: latest_usage });

        tracing::info!(
            target: "vw_agent",
            model = %model.api.id,
            acp_agent = %acp_agent_name,
            delta_count,
            "forwarding ACP deltas to stream consumer"
        );
        close_client_after_request_if_needed(close_client_after_request, &cached_client_entry)
            .await;
        drop(prompt_guard);
        return Ok(());
    }
}

/// Validate the captured client model before touching a CLI process. Credentials stay
/// in the Provider/auth snapshot; CLI-local execution does not consult this boundary.
pub(super) fn validate_session_model(
    model: &provider::Model,
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
) -> Result<(), Error> {
    if model.status != "active" || super::super::prompt::is_acp_model(model) {
        return Err(to_api_error("ACP session_model requires an active non-ACP Provider/model"));
    }
    if model.provider_id != provider_info.id {
        return Err(to_api_error("ACP session_model Provider identity mismatch"));
    }
    let has_credentials =
        provider_info.key.as_deref().is_some_and(|value| is_usable_copilot_token(value.trim()))
            || provider_info.env.iter().any(|key| {
                crate::app::agent::env::get(key)
                    .as_deref()
                    .is_some_and(|value| is_usable_copilot_token(value.trim()))
            })
            || match auth_info {
                Some(auth::Info::Api(info)) => is_usable_copilot_token(info.key.trim()),
                // Refresh remains the responsibility of the existing Provider transport.
                Some(auth::Info::Oauth(info)) => {
                    !info.access.trim().is_empty() || !info.refresh.trim().is_empty()
                }
                Some(auth::Info::Wellknown(info)) => !info.token.trim().is_empty(),
                None => false,
            };
    if !has_credentials {
        return Err(Error::Api(message::AssistantError::ProviderAuthError {
            provider_id: model.provider_id.clone(),
            message: "ACP session_model requires Provider credentials".to_string(),
        }));
    }
    Ok(())
}

async fn close_client_after_request_if_needed(should_close: bool, entry: &CachedAcpClientEntry) {
    if should_close {
        close_cached_acp_client_if_unshared(entry).await;
    }
}

fn parse_close_client_after_request(merged_options: &Value) -> Result<bool, Error> {
    let Some(value) = merged_options.get("acp_close_client_after_request") else {
        return Ok(false);
    };
    value
        .as_bool()
        .ok_or_else(|| super::acp_option_error("acp_close_client_after_request must be a boolean"))
}

fn is_usable_copilot_token(value: &str) -> bool {
    !value.is_empty() && value != auth::OAUTH_DUMMY_KEY
}

/// Validate advertised choices before requesting a mode, including permissive peers.
async fn apply_default_mode(
    client: &vw_acp::AcpClient,
    session_id: &str,
    cwd: &std::path::Path,
    mode: String,
) -> Result<(), Error> {
    let supported =
        client.session_capabilities(session_id).and_then(|info| info.modes).is_some_and(|modes| {
            modes.available_modes.iter().any(|candidate| candidate.id.to_string() == mode)
        });
    if !supported {
        return Err(super::acp_option_error(format!("ACP session mode is not advertised: {mode}")));
    }
    client.set_session_mode(session_id.to_string(), cwd, mode).await.map_err(to_api_error)
}
#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
