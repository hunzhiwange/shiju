//! 消息响应处理模块
//!
//! 本模块负责处理 LLM（大语言模型）执行完成后返回的各种结果类型，包括：
//! - 成功响应：处理并返回模型生成的文本内容
//! - 取消响应：处理因新消息到达而被取消的请求
//! - 错误响应：处理各类错误情况（上下文溢出、工具迭代限制等）
//! - 超时响应：处理模型响应超时的情况
//!
//! # 主要功能
//!
//! - 响应内容的安全检查（金丝雀令牌防护）
//! - 钩子系统的集成（消息发送前的拦截和修改）
//! - 响应内容的净化处理
//! - 会话历史的更新和管理
//! - 多种错误情况的优雅降级处理
//!
//! # 与其他模块的关系
//!
//! 该模块是通道管理器（channel manager）的核心组成部分，与以下模块紧密协作：
//! - `security::CanaryGuard` - 提供响应内容的安全检查
//! - `hooks` - 提供消息生命周期钩子
//! - `memory` - 管理会话历史记录

use super::*;

/// 处理 LLM 执行结果并发送响应消息
///
/// 该函数是消息处理流程的核心，负责根据 LLM 执行结果的不同状态采取相应的处理措施。
/// 它处理从简单的成功响应到复杂的错误恢复等各种情况。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含配置、工具注册表、钩子等运行时信息
/// * `msg` - 原始的通道消息，包含发送者、内容、回复目标等信息
/// * `target_channel` - 目标通道实例，用于发送响应消息（可选）
/// * `route` - 通道路由选择信息，包含 provider 和 model 标识
/// * `llm_result` - LLM 执行结果，可能是成功、失败或取消
/// * `history_key` - 会话历史的存储键
/// * `history` - 当前的聊天历史记录
/// * `history_len_before_tools` - 执行工具之前的聊天历史长度
/// * `had_prior_history` - 是否在本次请求之前就存在历史记录
/// * `started_at` - 请求开始的时间点，用于计算响应延迟
/// * `draft_message_id` - 草稿消息 ID（如果使用了草稿机制）
/// * `user_turn_content` - 用户回合的原始内容
/// * `turn_canary_token` - 当前回合的金丝雀令牌，用于检测信息泄露
/// * `reaction_done_emoji` - 完成时显示的表情符号
/// * `canary_guard` - 金丝雀防护实例，用于检查响应中是否包含敏感信息
///
/// # 处理流程
///
/// 1. **取消情况** (`LlmExecutionResult::Cancelled`)
///    - 记录取消事件并清理草稿消息
///    - 适用于因收到更新消息而取消旧请求的场景
///
/// 2. **成功情况** (`LlmExecutionResult::Completed(Ok(Ok(response)))`)
///    - 执行金丝雀令牌检查，防止内部上下文泄露
///    - 运行 `on_message_sending` 钩子，允许修改或拦截消息
///    - 净化响应内容（移除敏感的工具调用信息）
///    - 更新会话历史
///    - 发送响应到目标通道
///
/// 3. **错误情况** (`LlmExecutionResult::Completed(Ok(Err(e)))`)
///    - 工具循环取消：类似请求取消的处理
///    - 上下文窗口溢出：尝试压缩历史记录并提示用户重试
///    - 工具迭代限制：暂停任务并保留上下文
///    - 其他错误：回滚用户回合（如适用）并显示错误信息
///
/// 4. **超时情况** (`LlmExecutionResult::Completed(Err(_))`)
///    - 记录超时信息
///    - 更新历史记录标记任务超时
///    - 发送超时提示消息
///
/// # 示例
///
/// ```ignore
/// handle_llm_result(
///     &ctx,
///     message,
///     Some(&channel),
///     &route,
///     llm_result,
///     "user_123_history",
///     &history,
///     5,
///     true,
///     Instant::now(),
///     Some("draft_456"),
///     "用户的问题内容",
///     Some("canary_token_xyz"),
///     "✅",
///     &canary_guard,
/// ).await;
/// ```
///
/// # 安全性
///
/// - 金丝雀令牌检查确保响应中不包含敏感的内部上下文
/// - 钩子系统可以拦截包含敏感信息的响应
/// - 错误信息经过净化处理，避免泄露 API 密钥等敏感信息
///
/// # 副作用
///
/// - 可能修改会话历史记录
/// - 可能发送消息到目标通道
/// - 可能更新或删除草稿消息
/// - 会添加/移除消息反应表情
pub(crate) async fn handle_llm_result(
    ctx: &ChannelRuntimeContext,
    msg: traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    route: &ChannelRouteSelection,
    llm_result: LlmExecutionResult,
    history_key: &str,
    history: &[ChatMessage],
    history_len_before_tools: usize,
    had_prior_history: bool,
    started_at: Instant,
    draft_message_id: Option<&str>,
    user_turn_content: &str,
    turn_canary_token: Option<&str>,
    reaction_done_emoji: &str,
    canary_guard: &crate::app::agent::security::CanaryGuard,
) {
    match llm_result {
        // 情况 1：请求被取消（通常是因为收到了更新的消息）
        LlmExecutionResult::Cancelled => {
            // 记录取消事件的日志
            tracing::info!(
                channel = %msg.channel,
                sender = %msg.sender,
                "Cancelled in-flight channel request due to newer message"
            );
            // 记录运行时追踪事件，用于监控和分析
            runtime_trace::record_event(
                "channel_message_cancelled",
                Some(msg.channel.as_str()),
                Some(route.provider.as_str()),
                Some(route.model.as_str()),
                None,
                Some(false),
                Some("cancelled due to newer inbound message"),
                serde_json::json!({
                    "sender": msg.sender,
                    "elapsed_ms": started_at.elapsed().as_millis(),
                }),
            );
            // 如果存在草稿消息，则取消它
            if let (Some(channel), Some(draft_id)) = (target_channel, draft_message_id) {
                if let Err(err) = channel.cancel_draft(&msg.reply_target, draft_id).await {
                    tracing::debug!("Failed to cancel draft on {}: {err}", channel.name());
                }
            }
        }
        // 情况 2：请求成功完成并返回了响应
        LlmExecutionResult::Completed(Ok(Ok(response))) => {
            let mut outbound_response = response;

            // 金丝雀令牌检查：检测响应中是否包含敏感的内部上下文
            if canary_guard.response_contains_canary(&outbound_response, turn_canary_token) {
                runtime_trace::record_event(
                    "channel_message_blocked_canary_guard",
                    Some(msg.channel.as_str()),
                    Some(route.provider.as_str()),
                    Some(route.model.as_str()),
                    None,
                    Some(false),
                    Some("blocked response containing per-turn canary token"),
                    serde_json::json!({
                        "sender": msg.sender,
                        "message_id": msg.id,
                    }),
                );
                // 替换响应为安全提示信息
                outbound_response =
                    "I blocked that response because it attempted to reveal protected internal context."
                        .to_string();
            }

            // 执行消息发送钩子，允许拦截或修改响应内容
            if let Some(hooks) = &ctx.hooks {
                match hooks
                    .run_on_message_sending(
                        msg.channel.clone(),
                        msg.reply_target.clone(),
                        outbound_response.clone(),
                    )
                    .await
                {
                    // 钩子选择取消消息发送
                    crate::app::agent::hooks::HookResult::Cancel(reason) => {
                        tracing::info!(%reason, "outgoing message suppressed by hook");
                        return;
                    }
                    // 钩子允许消息发送，可能修改了内容
                    crate::app::agent::hooks::HookResult::Continue((
                        hook_channel,
                        hook_recipient,
                        mut modified_content,
                    )) => {
                        // 警告：钩子尝试修改通道路由（不被允许，仅应用内容修改）
                        if hook_channel != msg.channel || hook_recipient != msg.reply_target {
                            tracing::warn!(
                                from_channel = %msg.channel,
                                from_recipient = %msg.reply_target,
                                to_channel = %hook_channel,
                                to_recipient = %hook_recipient,
                                "on_message_sending attempted to rewrite channel routing; only content mutation is applied"
                            );
                        }

                        // 检查修改后的内容是否超出长度限制
                        let modified_len = modified_content.chars().count();
                        if modified_len > CHANNEL_HOOK_MAX_OUTBOUND_CHARS {
                            tracing::warn!(
                                limit = CHANNEL_HOOK_MAX_OUTBOUND_CHARS,
                                attempted = modified_len,
                                "hook-modified outbound content exceeded limit; truncating"
                            );
                            modified_content = truncate_with_ellipsis(
                                &modified_content,
                                CHANNEL_HOOK_MAX_OUTBOUND_CHARS.saturating_sub(3),
                            );
                        }

                        // 记录内容修改日志
                        if modified_content != outbound_response {
                            tracing::info!(
                                channel = %msg.channel,
                                sender = %msg.sender,
                                before_len = outbound_response.chars().count(),
                                after_len = modified_content.chars().count(),
                                "outgoing message content modified by hook"
                            );
                        }

                        outbound_response = modified_content;
                    }
                }
            }

            // 净化响应内容，移除敏感的工具调用信息
            let sanitized_response =
                sanitize_channel_response(&outbound_response, ctx.tools_registry.as_ref());

            // 如果净化后的响应为空但原始响应非空，说明存在格式问题
            let delivered_response = if sanitized_response.is_empty()
                && !outbound_response.trim().is_empty()
            {
                "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
                    .to_string()
            } else {
                sanitized_response
            };

            // 记录出站消息事件
            runtime_trace::record_event(
                "channel_message_outbound",
                Some(msg.channel.as_str()),
                Some(route.provider.as_str()),
                Some(route.model.as_str()),
                None,
                Some(true),
                None,
                serde_json::json!({
                    "sender": msg.sender,
                    "elapsed_ms": started_at.elapsed().as_millis(),
                    "response": scrub_credentials(&delivered_response),
                }),
            );

            // 提取工具上下文摘要，用于在响应前附加工具执行信息
            let tool_summary = extract_tool_context_summary(history, history_len_before_tools);

            // 构建历史记录中的响应内容
            // 注意：Telegram 通道不附加工具摘要（避免消息过长）
            let history_response = if tool_summary.is_empty() || msg.channel == "telegram" {
                delivered_response.clone()
            } else {
                format!("{tool_summary}\n{delivered_response}")
            };

            if let Err(err) = super::super::session_turn::persist_channel_turn(
                ctx,
                &msg,
                route,
                &history_response,
            )
            .await
            {
                if let Some(channel) = target_channel {
                    let error = crate::providers::sanitize_api_error(&err.to_string());
                    let _ = channel
                        .send(
                            &SendMessage::new(
                                format!("会话持久化失败：{error}。请重试。"),
                                &msg.reply_target,
                            )
                            .in_thread(msg.thread_ts.clone()),
                        )
                        .await;
                }
                return;
            }

            // 将助手响应追加到会话历史
            append_sender_turn(ctx, history_key, ChatMessage::assistant(&history_response));

            // 如果这是新会话的第一条消息，触发会话标题刷新
            if !had_prior_history {
                let session_id = ctx.message_snapshot.as_ref().and_then(|snapshot| {
                    sender_session_store()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&snapshot.scope)
                        .cloned()
                });
                if let Some(session_id) = session_id {
                    spawn_channel_session_title_refresh(
                        session_id,
                        msg.content.clone(),
                        Some(route.model.clone()),
                    );
                }
            }

            // 在控制台打印响应摘要
            println!(
                "  🤖 Reply ({}ms): {}",
                started_at.elapsed().as_millis(),
                truncate_with_ellipsis(&delivered_response, 80)
            );

            // 发送响应到目标通道
            if let Some(channel) = target_channel {
                // 如果存在草稿消息，则完成草稿
                if let Some(draft_id) = draft_message_id {
                    if let Err(e) = channel
                        .finalize_draft(&msg.reply_target, draft_id, &delivered_response)
                        .await
                    {
                        // 草稿完成失败，回退到发送新消息
                        tracing::warn!("Failed to finalize draft: {e}; sending as new message");
                        let _ = channel
                            .send(
                                &SendMessage::new(&delivered_response, &msg.reply_target)
                                    .in_thread(msg.thread_ts.clone()),
                            )
                            .await;
                    }
                } else {
                    // 直接发送新消息
                    if let Err(e) = channel
                        .send(
                            &SendMessage::new(delivered_response, &msg.reply_target)
                                .in_thread(msg.thread_ts.clone()),
                        )
                        .await
                    {
                        eprintln!("  ❌ Failed to reply on {}: {e}", channel.name());
                    }
                }
            }
        }
        // 情况 3：请求完成但返回了错误
        LlmExecutionResult::Completed(Ok(Err(e))) => {
            // 子情况 3a：工具循环被取消
            if crate::app::agent::agent::loop_::is_tool_loop_cancelled(&e) {
                tracing::info!(
                    channel = %msg.channel,
                    sender = %msg.sender,
                    "Cancelled in-flight channel request due to newer message"
                );
                runtime_trace::record_event(
                    "channel_message_cancelled",
                    Some(msg.channel.as_str()),
                    Some(route.provider.as_str()),
                    Some(route.model.as_str()),
                    None,
                    Some(false),
                    Some("cancelled during tool-call loop"),
                    serde_json::json!({
                        "sender": msg.sender,
                        "elapsed_ms": started_at.elapsed().as_millis(),
                    }),
                );
                // 取消草稿消息
                if let (Some(channel), Some(draft_id)) = (target_channel, draft_message_id) {
                    if let Err(err) = channel.cancel_draft(&msg.reply_target, draft_id).await {
                        tracing::debug!("Failed to cancel draft on {}: {err}", channel.name());
                    }
                }
            }
            // 子情况 3b：上下文窗口溢出
            else if is_context_window_overflow_error(&e) {
                // The processor already attempted shared compaction. Keep raw history
                // intact when recovery fails instead of truncating it in the channel layer.
                let error_text = "⚠️ Context window exceeded and context compaction could not recover the request. Your history is unchanged. Please retry or start a new session.";
                eprintln!(
                    "  ⚠️ Context window exceeded after {}ms",
                    started_at.elapsed().as_millis()
                );

                runtime_trace::record_event(
                    "channel_message_error",
                    Some(msg.channel.as_str()),
                    Some(route.provider.as_str()),
                    Some(route.model.as_str()),
                    None,
                    Some(false),
                    Some("context window exceeded"),
                    serde_json::json!({
                        "sender": msg.sender,
                        "elapsed_ms": started_at.elapsed().as_millis(),
                        "history_compacted": false,
                    }),
                );

                // 发送错误提示到目标通道
                if let Some(channel) = target_channel {
                    if let Some(draft_id) = draft_message_id {
                        let _ =
                            channel.finalize_draft(&msg.reply_target, draft_id, error_text).await;
                    } else {
                        let _ = channel
                            .send(
                                &SendMessage::new(error_text, &msg.reply_target)
                                    .in_thread(msg.thread_ts.clone()),
                            )
                            .await;
                    }
                }
            }
            // 子情况 3c：达到工具迭代限制
            else if is_tool_iteration_limit_error(&e) {
                let limit = ctx.max_iterations.max(1);
                let pause_text = format!(
                    "⚠️ Reached tool-iteration limit ({limit}) for this turn. Context and progress were preserved. Reply \"continue\" to resume, or increase `agents.main.max_iterations`."
                );

                runtime_trace::record_event(
                    "channel_message_error",
                    Some(msg.channel.as_str()),
                    Some(route.provider.as_str()),
                    Some(route.model.as_str()),
                    None,
                    Some(false),
                    Some("tool iteration limit reached"),
                    serde_json::json!({
                        "sender": msg.sender,
                        "elapsed_ms": started_at.elapsed().as_millis(),
                        "max_iterations": limit,
                    }),
                );

                // 在历史中记录暂停状态，以便后续可以恢复
                append_sender_turn(
                    ctx,
                    history_key,
                    ChatMessage::assistant(
                        "[Task paused at tool-iteration limit — context preserved. Ask to continue.]",
                    ),
                );

                // 发送暂停提示到目标通道
                if let Some(channel) = target_channel {
                    if let Some(draft_id) = draft_message_id {
                        let _ =
                            channel.finalize_draft(&msg.reply_target, draft_id, &pause_text).await;
                    } else {
                        let _ = channel
                            .send(
                                &SendMessage::new(pause_text, &msg.reply_target)
                                    .in_thread(msg.thread_ts.clone()),
                            )
                            .await;
                    }
                }
            }
            // 子情况 3d：其他类型的错误
            else {
                eprintln!("  ❌ LLM error after {}ms: {e}", started_at.elapsed().as_millis());

                // 净化错误信息，避免泄露敏感信息
                let safe_error = crate::app::agent::providers::sanitize_api_error(&e.to_string());

                runtime_trace::record_event(
                    "channel_message_error",
                    Some(msg.channel.as_str()),
                    Some(route.provider.as_str()),
                    Some(route.model.as_str()),
                    None,
                    Some(false),
                    Some(&safe_error),
                    serde_json::json!({
                        "sender": msg.sender,
                        "elapsed_ms": started_at.elapsed().as_millis(),
                    }),
                );

                // 检查是否为视觉能力错误，如果是则考虑回滚用户回合
                let should_rollback_user_turn = e
                    .downcast_ref::<crate::app::agent::providers::ProviderCapabilityError>()
                    .is_some_and(|capability| capability.capability.eq_ignore_ascii_case("vision"));

                let rolled_back = should_rollback_user_turn
                    && rollback_orphan_user_turn(ctx, history_key, user_turn_content);

                // 如果没有回滚，则在历史中记录失败状态
                if !rolled_back {
                    append_sender_turn(
                        ctx,
                        history_key,
                        ChatMessage::assistant("[Task failed — not continuing this request]"),
                    );
                }

                // 发送错误提示到目标通道
                if let Some(channel) = target_channel {
                    if let Some(draft_id) = draft_message_id {
                        let _ = channel
                            .finalize_draft(&msg.reply_target, draft_id, &format!("⚠️ Error: {e}"))
                            .await;
                    } else {
                        let _ = channel
                            .send(
                                &SendMessage::new(format!("⚠️ Error: {e}"), &msg.reply_target)
                                    .in_thread(msg.thread_ts.clone()),
                            )
                            .await;
                    }
                }
            }
        }
        // 情况 4：请求超时
        LlmExecutionResult::Completed(Err(_)) => {
            // 构建超时错误消息
            let timeout_msg = format!(
                "LLM response timed out after {}s (base={}s, max_iterations={})",
                channel_message_timeout_budget_secs(ctx.message_timeout_secs, ctx.max_iterations),
                ctx.message_timeout_secs,
                ctx.max_iterations
            );

            runtime_trace::record_event(
                "channel_message_timeout",
                Some(msg.channel.as_str()),
                Some(route.provider.as_str()),
                Some(route.model.as_str()),
                None,
                Some(false),
                Some(&timeout_msg),
                serde_json::json!({
                    "sender": msg.sender,
                    "elapsed_ms": started_at.elapsed().as_millis(),
                }),
            );

            eprintln!("  ❌ {} (elapsed: {}ms)", timeout_msg, started_at.elapsed().as_millis());

            // 在历史中记录超时状态
            append_sender_turn(
                ctx,
                history_key,
                ChatMessage::assistant("[Task timed out — not continuing this request]"),
            );

            // 发送超时提示到目标通道
            if let Some(channel) = target_channel {
                let error_text =
                    "⚠️ Request timed out while waiting for the model. Please try again.";
                if let Some(draft_id) = draft_message_id {
                    let _ = channel.finalize_draft(&msg.reply_target, draft_id, error_text).await;
                } else {
                    let _ = channel
                        .send(
                            &SendMessage::new(error_text, &msg.reply_target)
                                .in_thread(msg.thread_ts.clone()),
                        )
                        .await;
                }
            }
        }
    }

    // 更新消息反应：移除"处理中"表情，添加"完成"表情
    if let Some(channel) = target_channel {
        // 移除 👀 (眼睛) 表情，表示正在处理
        let _ = channel.remove_reaction(&msg.reply_target, &msg.id, "\u{1F440}").await;
        // 添加完成表情（通常是 ✅）
        let _ = channel.add_reaction(&msg.reply_target, &msg.id, reaction_done_emoji).await;
    }
}

#[cfg(test)]
#[path = "message_response_tests.rs"]
mod message_response_tests;
