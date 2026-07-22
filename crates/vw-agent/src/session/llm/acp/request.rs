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
use super::{CachedAcpClientEntry, Error, StreamEvent, is_acp_session_changed_error, to_api_error};

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

    let cfg = config::get().await;
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
    let normalized_acp_cfg = with_copilot_auth_environment(
        model,
        provider_info,
        auth_info,
        &acp_agent_name,
        normalize_acp_agent_config(&acp_agent_name, &acp_cfg),
    );
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

    let requested_force_new_session =
        merged_options.get("acp_force_new_session").and_then(Value::as_bool).unwrap_or(false);
    let replay_strategy = parse_replay_strategy(merged_options);
    let replay_recent_count = parse_recent_count(merged_options);
    let initial_prompt_len = build_request_prompt(
        chat_messages,
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
    let cwd = merged_options
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| to_api_error("failed to resolve cwd for acp session"))?;
    let parsed_options = parse_acp_options(merged_options, &acp_agent_name, &normalized_acp_cfg)?;
    let close_client_after_request = parse_close_client_after_request(merged_options)?;
    let cwd_text = cwd.to_string_lossy().into_owned();
    let walk_boundary = vw_acp::find_git_repository_root(&cwd_text)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd_text.clone());
    let session_name = acp_session_name(session_id);
    let mut force_new_session = requested_force_new_session;
    let mut recovered_session_change = false;
    loop {
        let prompt = build_request_prompt(
            chat_messages,
            force_new_session,
            replay_strategy,
            replay_recent_count,
        );
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

        let cached_client_entry =
            get_cached_acp_client(&acp_agent_name, &normalized_acp_cfg, &cwd, &parsed_options);
        let cached_client = &cached_client_entry.client;
        // 同一个 ACP 客户端可能被多个本地请求复用；串行化 prompt 可以避免代理端
        // 把并发输入混到同一会话流里。
        let prompt_guard = cached_client.prompt_lock.lock().await;
        let mut record = match find_or_create_cached_session_record(
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
        {
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

        let acp_session_id = record.acp_session_id.clone();
        if let Some(mode_id) = parsed_options.session_mode.clone() {
            if let Err(err) = cached_client
                .client
                .set_session_mode(acp_session_id.clone(), &cwd, mode_id)
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
        if let Some(model_id) =
            parsed_options.session_options.as_ref().and_then(|options| options.model.clone())
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
        for (config_id, value) in parsed_options.session_config_options.clone() {
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
                    PromptRequest::new(prompt_cwd, prompt_text)
                        .with_session_strategy(SessionStrategy::ResumeLoadOrNew(prompt_session_id)),
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
                // ACP 代理报告会话 id 改变时，本地记录已经不可信。只重试一次并强制
                // 创建新会话，既能恢复常见漂移，又避免持续失败时无限循环。
                recovered_session_change = true;
                force_new_session = true;
                tracing::warn!(
                    target: "vw_agent",
                    model = %model.api.id,
                    acp_agent = %acp_agent_name,
                    "ACP session changed during prompt; retrying with a new session"
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

fn with_copilot_auth_environment(
    model: &provider::Model,
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    acp_agent_name: &str,
    mut acp_cfg: config::schema::AcpAgentConfig,
) -> config::schema::AcpAgentConfig {
    if !is_github_copilot_provider(&model.provider_id)
        || !is_copilot_acp_agent(acp_agent_name, &acp_cfg)
    {
        return acp_cfg;
    }

    let Some(token) = copilot_auth_token(provider_info, auth_info) else {
        return acp_cfg;
    };

    for key in ["GH_COPILOT_TOKEN", "GITHUB_COPILOT_TOKEN"] {
        acp_cfg.env.entry(key.to_string()).or_insert_with(|| token.clone());
    }
    acp_cfg
}

fn is_github_copilot_provider(provider_id: &str) -> bool {
    provider_id.trim().to_ascii_lowercase().starts_with("github-copilot")
}

fn is_copilot_acp_agent(acp_agent_name: &str, acp_cfg: &config::schema::AcpAgentConfig) -> bool {
    let agent_name = acp_agent_name.trim().to_ascii_lowercase();
    let command = acp_cfg.command.trim().to_ascii_lowercase();
    let has_copilot_arg =
        acp_cfg.args.iter().any(|arg| arg.trim().to_ascii_lowercase().contains("copilot"));

    agent_name.contains("copilot") || command.contains("copilot") || has_copilot_arg
}

fn copilot_auth_token(
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
) -> Option<String> {
    provider_info
        .key
        .as_deref()
        .map(str::trim)
        .filter(|value| is_usable_copilot_token(value))
        .map(ToString::to_string)
        .or_else(|| {
            for key in &provider_info.env {
                if let Ok(value) = std::env::var(key) {
                    let value = value.trim();
                    if is_usable_copilot_token(value) {
                        return Some(value.to_string());
                    }
                }
            }
            None
        })
        .or_else(|| match auth_info {
            Some(auth::Info::Api(api)) if !api.key.trim().is_empty() => {
                Some(api.key.trim().to_string())
            }
            _ => None,
        })
}

fn is_usable_copilot_token(value: &str) -> bool {
    !value.is_empty() && value != auth::OAUTH_DUMMY_KEY
}
#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
