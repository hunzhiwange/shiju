//! 通道入站消息的主处理流程。
//!
//! 本模块负责把一条外部通道消息串联到代理执行：应用入站钩子、处理运行时
//! 命令、加载会话历史、构造系统提示、启动草稿/输入状态反馈，并最终把 LLM
//! 结果交给响应层。安全相关的语义防护和 canary token 也在这里按消息轮次接入。

use super::*;

/// 处理一条来自通道的消息。
///
/// 参数：
/// - `ctx`：通道运行时共享上下文，包含 provider、工具、历史和通道注册表。
/// - `msg`：已经标准化的通道消息。
/// - `cancellation_token`：用于在外部关闭或重启时提前终止本轮处理。
///
/// 返回值：无返回值，所有用户可见结果都通过目标通道发送。
///
/// 错误处理：本函数把可恢复错误转换为日志、通道通知或提前返回，避免单条消息
/// 失败拖垮监听循环。
pub(crate) async fn process_channel_message(
    ctx: Arc<ChannelRuntimeContext>,
    msg: traits::ChannelMessage,
    cancellation_token: CancellationToken,
) {
    if cancellation_token.is_cancelled() {
        return;
    }

    log_inbound_message(&msg);

    let msg = match apply_inbound_hooks(ctx.as_ref(), msg).await {
        Some(modified) => modified,
        None => return,
    };

    let target_channel = ctx.channels_by_name.get(&msg.channel).cloned();
    if let Err(err) = maybe_apply_runtime_config_update(ctx.as_ref()).await {
        tracing::warn!("Failed to apply runtime config update: {err}");
    }
    // 运行时命令需要优先截获，避免 `/approve`、`/model` 等控制指令进入模型上下文。
    if handle_runtime_command_if_needed(ctx.as_ref(), &msg, target_channel.as_ref()).await {
        return;
    }

    // canary 需要在构造本轮提示前确定开关，以便令牌只覆盖当前用户回合。
    let canary_enabled_for_turn =
        match apply_semantic_guard(ctx.as_ref(), &msg, target_channel.as_ref()).await {
            Some(enabled) => enabled,
            None => return,
        };

    let history_key = conversation_history_key(&msg);
    let route = classify_message_route(ctx.as_ref(), &msg.content)
        .unwrap_or_else(|| get_route_selection(ctx.as_ref(), &history_key));
    let active_provider = match get_or_create_provider(ctx.as_ref(), &route.provider).await {
        Ok(provider) => provider,
        Err(err) => {
            notify_provider_init_failure(&msg, target_channel.as_ref(), &route.provider, err).await;
            return;
        }
    };
    maybe_auto_save_message(ctx.as_ref(), &msg).await;

    println!("  ⏳ Processing message...");
    let started_at = Instant::now();

    let had_prior_history = has_prior_history(ctx.as_ref(), &history_key);
    let user_turn_content = msg.content.clone();

    append_sender_turn(ctx.as_ref(), &history_key, ChatMessage::user(&user_turn_content));

    let prior_turns =
        load_prior_turns(ctx.as_ref(), &history_key, &msg.content, had_prior_history).await;
    let expose_internal_tool_details =
        msg.channel == "cli" || should_expose_internal_tool_details(&msg.content);
    let excluded_tools_snapshot =
        excluded_tools_snapshot_for_channel(ctx.as_ref(), msg.channel.as_str());
    // 工具可见性写入系统提示而不是运行期临时过滤，便于模型理解哪些能力不可用。
    let mut system_prompt = build_channel_system_prompt(
        ctx.system_prompt.as_str(),
        &msg.channel,
        &msg.reply_target,
        expose_internal_tool_details,
    );
    system_prompt.push_str(&build_runtime_tool_visibility_prompt(
        ctx.tools_registry.as_ref(),
        &excluded_tools_snapshot,
        active_provider.supports_native_tools(),
    ));
    let canary_guard = crate::app::agent::security::CanaryGuard::new(canary_enabled_for_turn);
    let (system_prompt, turn_canary_token) = canary_guard.inject_turn_token(&system_prompt);
    let session_history = to_session_history(&prior_turns);
    let mut history = vec![ChatMessage::system(system_prompt)];
    history.extend(prior_turns);
    let use_streaming = target_channel.as_ref().is_some_and(|ch| ch.supports_draft_updates());

    tracing::debug!(
        channel = %msg.channel,
        has_target_channel = target_channel.is_some(),
        use_streaming,
        supports_draft = target_channel.as_ref().map_or(false, |ch| ch.supports_draft_updates()),
        "Draft streaming decision"
    );

    let StreamingUiState { delta_tx, draft_message_id, draft_updater } =
        prepare_streaming_ui_state(
            target_channel.as_ref(),
            &msg,
            expose_internal_tool_details,
            use_streaming,
        )
        .await;

    maybe_add_processing_reaction(target_channel.as_ref(), &msg).await;

    let (typing_cancellation, typing_task) =
        start_typing_feedback(target_channel.as_ref(), msg.reply_target.clone());

    let history_len_before_tools = history.len();

    // 超时预算随工具迭代上限增长，避免合法的多工具回合被固定短超时截断。
    let timeout_budget_secs =
        channel_message_timeout_budget_secs(ctx.message_timeout_secs, ctx.max_iterations);
    let llm_result = run_message_execution(
        ctx.as_ref(),
        &msg,
        &route,
        session_history,
        delta_tx,
        timeout_budget_secs,
        &cancellation_token,
    )
    .await;

    finish_feedback_tasks(draft_updater, typing_cancellation, typing_task).await;

    let reaction_done_emoji = reaction_done_emoji(&llm_result);

    handle_llm_result(
        ctx.as_ref(),
        msg,
        target_channel.as_ref(),
        &route,
        llm_result,
        &history_key,
        &history,
        history_len_before_tools,
        had_prior_history,
        started_at,
        draft_message_id.as_deref(),
        &user_turn_content,
        turn_canary_token.as_deref(),
        reaction_done_emoji,
        &canary_guard,
    )
    .await;
}

struct StreamingUiState {
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    draft_message_id: Option<String>,
    draft_updater: Option<tokio::task::JoinHandle<()>>,
}

fn excluded_tools_snapshot_for_channel(
    ctx: &ChannelRuntimeContext,
    channel_name: &str,
) -> Vec<String> {
    if channel_name == "cli" { Vec::new() } else { snapshot_non_cli_excluded_tools(ctx) }
}

async fn prepare_streaming_ui_state(
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
    expose_internal_tool_details: bool,
    use_streaming: bool,
) -> StreamingUiState {
    let (delta_tx, delta_rx) = if use_streaming {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(64);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let draft_message_id = maybe_send_initial_draft(target_channel, msg, use_streaming).await;
    let draft_updater = spawn_draft_updater(
        delta_rx,
        draft_message_id.as_deref(),
        target_channel,
        msg.reply_target.clone(),
        expose_internal_tool_details,
    );

    StreamingUiState { delta_tx, draft_message_id, draft_updater }
}

async fn maybe_send_initial_draft(
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
    use_streaming: bool,
) -> Option<String> {
    if !use_streaming {
        return None;
    }

    let Some(channel) = target_channel else {
        return None;
    };

    // 先发占位草稿，再异步累积 delta，降低慢模型在聊天通道中的无反馈时间。
    match channel
        .send_draft(&SendMessage::new("...", &msg.reply_target).in_thread(msg.thread_ts.clone()))
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::debug!("Failed to send draft on {}: {e}", channel.name());
            None
        }
    }
}

fn spawn_draft_updater(
    delta_rx: Option<tokio::sync::mpsc::Receiver<String>>,
    draft_message_id: Option<&str>,
    target_channel: Option<&Arc<dyn Channel>>,
    reply_target: String,
    expose_internal_tool_details: bool,
) -> Option<tokio::task::JoinHandle<()>> {
    let (Some(mut rx), Some(draft_id_ref), Some(channel_ref)) =
        (delta_rx, draft_message_id, target_channel)
    else {
        return None;
    };

    let channel = Arc::clone(channel_ref);
    let draft_id = draft_id_ref.to_string();
    let suppress_internal_progress = !expose_internal_tool_details;
    Some(crate::app::agent::util::spawn(async move {
        let mut accumulated = String::new();
        while let Some(delta) = rx.recv().await {
            if delta == crate::app::agent::agent::loop_::DRAFT_CLEAR_SENTINEL {
                accumulated.clear();
                continue;
            }
            let (is_internal_progress, visible_delta) = split_internal_progress_delta(&delta);
            if suppress_internal_progress && is_internal_progress {
                continue;
            }

            // 通道草稿 API 通常是替换整条消息，因此这里保留完整累计内容。
            accumulated.push_str(visible_delta);
            if let Err(e) = channel.update_draft(&reply_target, &draft_id, &accumulated).await {
                tracing::debug!("Draft update failed: {e}");
            }
        }
    }))
}

async fn maybe_add_processing_reaction(
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
) {
    let Some(channel) = target_channel else {
        return;
    };

    if let Err(e) = channel.add_reaction(&msg.reply_target, &msg.id, "\u{1F440}").await {
        tracing::debug!("Failed to add reaction: {e}");
    }
}

fn start_typing_feedback(
    target_channel: Option<&Arc<dyn Channel>>,
    reply_target: String,
) -> (Option<CancellationToken>, Option<tokio::task::JoinHandle<()>>) {
    let typing_cancellation = target_channel.map(|_| CancellationToken::new());
    let typing_task = match (target_channel, typing_cancellation.as_ref()) {
        (Some(channel), Some(token)) => {
            Some(spawn_scoped_typing_task(Arc::clone(channel), reply_target, token.clone()))
        }
        _ => None,
    };
    (typing_cancellation, typing_task)
}

async fn finish_feedback_tasks(
    draft_updater: Option<tokio::task::JoinHandle<()>>,
    typing_cancellation: Option<CancellationToken>,
    typing_task: Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(handle) = draft_updater {
        let _ = handle.await;
    }

    if let Some(token) = typing_cancellation.as_ref() {
        token.cancel();
    }
    if let Some(handle) = typing_task {
        log_worker_join_result(handle.await);
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod message_tests;
