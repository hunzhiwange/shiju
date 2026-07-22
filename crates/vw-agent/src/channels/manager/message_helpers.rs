//! 消息处理助手模块
//!
//! 本模块提供通道管理器中处理消息的核心辅助函数，负责消息的日志记录、
//! 钩子处理、安全防护、错误通知以及历史记录管理等功能。
//!
//! # 主要功能
//!
//! - **入站消息处理**：记录消息日志、应用入站钩子、执行语义安全检测
//! - **自动保存**：将符合条件的消息自动存储到记忆系统
//! - **历史管理**：查询和加载对话历史记录
//! - **错误通知**：向用户报告 provider 初始化失败等异常情况
//!
//! # 安全特性
//!
//! 模块集成了语义防护（Semantic Guard）机制，用于检测和拦截潜在的
//! 提示注入攻击，保护系统免受恶意输入的影响。

use super::*;

/// 记录入站消息的日志信息
///
/// 将消息同时输出到控制台和运行时追踪系统，便于调试和监控。
/// 控制台输出使用截断格式以保持可读性，追踪系统则记录更完整的信息。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者、内容、通道等元数据
///
/// # 日志内容
///
/// 控制台输出格式：`💬 [通道] from 发送者: 内容预览`
///
/// 追踪系统记录的字段：
/// - `sender`: 消息发送者标识
/// - `message_id`: 消息唯一标识
/// - `reply_target`: 回复目标
/// - `content_preview`: 消息内容预览（最多160字符）
pub(crate) fn log_inbound_message(msg: &traits::ChannelMessage) {
    println!(
        "  💬 [{}] from {}: {}",
        msg.channel,
        msg.sender,
        truncate_with_ellipsis(&msg.content, 80)
    );
    runtime_trace::record_event(
        "channel_message_inbound",
        Some(msg.channel.as_str()),
        None,
        None,
        None,
        None,
        None,
        serde_json::json!({
            "sender": msg.sender,
            "message_id": msg.id,
            "reply_target": msg.reply_target,
            "content_preview": truncate_with_ellipsis(&msg.content, 160),
        }),
    );
}

/// 对入站消息应用钩子处理
///
/// 如果配置了消息接收钩子，则执行钩子逻辑，允许钩子修改或取消消息。
/// 这是消息处理流水线中的第一个可扩展点。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文，包含钩子配置
/// - `msg`: 待处理的通道消息
///
/// # 返回值
///
/// - `Some(modified_msg)`: 消息通过钩子（可能被修改），继续处理流程
/// - `None`: 钩子取消了消息，应停止后续处理
///
/// # 钩子行为
///
/// - `HookResult::Cancel(reason)`: 钩子拒绝消息，记录原因并返回 `None`
/// - `HookResult::Continue(modified)`: 钩子允许消息通过，返回可能修改后的消息
///
/// # 示例
///
/// ```ignore
/// if let Some(processed_msg) = apply_inbound_hooks(&ctx, raw_msg).await {
///     // 继续处理消息
/// } else {
///     // 消息被钩子取消
/// }
/// ```
pub(crate) async fn apply_inbound_hooks(
    ctx: &ChannelRuntimeContext,
    msg: traits::ChannelMessage,
) -> Option<traits::ChannelMessage> {
    if let Some(hooks) = &ctx.hooks {
        match hooks.run_on_message_received(msg).await {
            crate::app::agent::hooks::HookResult::Cancel(reason) => {
                tracing::info!(%reason, "incoming message dropped by hook");
                None
            }
            crate::app::agent::hooks::HookResult::Continue(modified) => Some(modified),
        }
    } else {
        Some(msg)
    }
}

/// 应用语义安全防护检测
///
/// 对消息内容进行语义分析，检测潜在的提示注入攻击或其他安全威胁。
/// 这是系统安全防护层的重要组成部分。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文
/// - `msg`: 待检测的通道消息
/// - `target_channel`: 可选的目标通道引用，用于发送警告消息
///
/// # 返回值
///
/// - `Some(false)`: 命令消息（以 `/` 开头）或语义防护未启用，无需进一步检测
/// - `Some(true)`: 消息通过检测，且金丝雀令牌（canary tokens）已启用
/// - `None`: 消息被语义防护拦截，不应继续处理
///
/// # 检测流程
///
/// 1. 跳过命令消息（以 `/` 开头的消息）
/// 2. 从运行时配置加载语义防护设置
/// 3. 如果启用，使用 `SemanticGuard` 对内容进行向量相似度检测
/// 4. 若检测到威胁，记录事件并可选地向用户发送警告
///
/// # 配置依赖
///
/// 需要在配置中设置以下字段：
/// - `security.semantic_guard`: 是否启用语义防护
/// - `security.semantic_guard_collection`: 用于检测的向量集合名称
/// - `security.semantic_guard_threshold`: 检测阈值
/// - `memory`: 记忆配置（用于向量存储访问）
/// - `api_key`: 可选的 API 密钥
pub(crate) async fn apply_semantic_guard(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
) -> Option<bool> {
    // 命令消息跳过语义检测
    if msg.content.trim_start().starts_with('/') {
        return Some(false);
    }

    // 从运行时配置文件加载语义防护配置
    let semantic_cfg: Option<(
        bool,
        bool,
        String,
        f64,
        crate::app::agent::config::MemoryConfig,
        Option<String>,
    )> = if let Some(config_path) = runtime_config_path(ctx) {
        // WASM 目标不支持文件系统操作
        #[cfg(target_arch = "wasm32")]
        {
            None
        }

        // 原生平台：读取并解析配置文件
        #[cfg(not(target_arch = "wasm32"))]
        match tokio::fs::read_to_string(&config_path).await {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(mut cfg) => {
                    cfg.config_path = config_path;
                    apply_env_overrides(&mut cfg);
                    Some((
                        cfg.security.canary_tokens,
                        cfg.security.semantic_guard,
                        cfg.security.semantic_guard_collection,
                        cfg.security.semantic_guard_threshold,
                        cfg.memory,
                        cfg.api_key,
                    ))
                }
                Err(err) => {
                    tracing::debug!("semantic guard: failed to parse runtime config: {err}");
                    None
                }
            },
            Err(err) => {
                tracing::debug!("semantic guard: failed to read runtime config: {err}");
                None
            }
        }
    } else {
        None
    };

    // 解构配置，如果配置不可用则跳过检测
    let Some((
        canary_enabled,
        semantic_enabled,
        semantic_collection,
        semantic_threshold,
        memory_cfg,
        api_key,
    )) = semantic_cfg
    else {
        return Some(false);
    };

    // 执行语义检测
    if semantic_enabled {
        let semantic_guard = crate::app::agent::security::SemanticGuard::from_config(
            &memory_cfg,
            semantic_enabled,
            semantic_collection.as_str(),
            semantic_threshold,
            api_key.as_deref(),
        );

        // 如果检测到威胁
        if let Some(detection) = semantic_guard.detect(&msg.content).await {
            block_semantic_guard_message(
                msg,
                target_channel,
                &detection,
                semantic_threshold,
                &semantic_collection,
            )
            .await;
            return None;
        }
    }

    Some(canary_enabled)
}

pub(crate) async fn block_semantic_guard_message(
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    detection: &crate::security::semantic_guard::SemanticMatch,
    semantic_threshold: f64,
    semantic_collection: &str,
) {
    runtime_trace::record_event(
        "channel_message_blocked_semantic_guard",
        Some(msg.channel.as_str()),
        None,
        None,
        None,
        Some(false),
        Some("blocked by semantic prompt-injection guard"),
        serde_json::json!({
            "sender": msg.sender,
            "message_id": msg.id,
            "score": detection.score,
            "threshold": semantic_threshold,
            "category": detection.category,
            "collection": semantic_collection,
        }),
    );

    if let Some(channel) = target_channel {
        let warning = format!(
            "Request blocked by `security.semantic_guard` before provider execution.\n\
 semantic_match={:.2} (threshold {:.2}), category={}",
            detection.score, semantic_threshold, detection.category
        );
        let _ = channel
            .send(&SendMessage::new(warning, &msg.reply_target).in_thread(msg.thread_ts.clone()))
            .await;
    }
}

/// 通知用户 provider 初始化失败
///
/// 当 AI 模型 provider 初始化失败时，向用户发送友好的错误通知，
/// 提示用户使用 `/models` 命令选择其他 provider。
///
/// # 参数
///
/// - `msg`: 原始消息引用，用于确定回复目标
/// - `target_channel`: 目标通道引用
/// - `provider_name`: 失败的 provider 名称
/// - `err`: 初始化错误
///
/// # 安全处理
///
/// 错误信息会经过 `sanitize_api_error` 函数处理，移除敏感信息
/// （如 API 密钥、内部路径等）后再展示给用户。
pub(crate) async fn notify_provider_init_failure(
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    provider_name: &str,
    err: anyhow::Error,
) {
    // 清理错误信息，移除敏感内容
    let safe_err = crate::app::agent::providers::sanitize_api_error(&err.to_string());
    let message = format!(
        "⚠️ Failed to initialize provider `{}`. Please run `/models` to choose another provider.\nDetails: {safe_err}",
        provider_name
    );
    if let Some(channel) = target_channel {
        let _ = channel
            .send(&SendMessage::new(message, &msg.reply_target).in_thread(msg.thread_ts.clone()))
            .await;
    }
}

/// 根据配置自动保存消息到记忆系统
///
/// 当满足以下条件时，将消息内容自动存储到对话记忆中：
/// 1. `auto_save_memory` 配置项已启用
/// 2. 消息内容长度达到最小字符数要求
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文，包含自动保存配置和记忆存储
/// - `msg`: 待保存的通道消息
///
/// # 存储格式
///
/// - 类别：`MemoryCategory::Conversation`
/// - 键：由 `conversation_memory_key` 函数生成，基于通道和用户信息
/// - 无过期时间
pub(crate) async fn maybe_auto_save_message(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) {
    if ctx.auto_save_memory && msg.content.chars().count() >= AUTOSAVE_MIN_MESSAGE_CHARS {
        let autosave_key = conversation_memory_key(msg);
        let _ =
            ctx.memory.store(&autosave_key, &msg.content, MemoryCategory::Conversation, None).await;
    }
}

/// 检查指定键是否存在对话历史记录
///
/// 查询会话历史缓存，判断是否存在之前保存的对话轮次。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文，包含对话历史缓存
/// - `history_key`: 历史记录的查询键
///
/// # 返回值
///
/// - `true`: 存在至少一条历史记录
/// - `false`: 无历史记录或键不存在
///
/// # 线程安全
///
/// 使用 `Mutex` 保护共享状态，锁 poisoned 时会恢复并返回 false。
pub(crate) fn has_prior_history(ctx: &ChannelRuntimeContext, history_key: &str) -> bool {
    ctx.conversation_histories
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(history_key)
        .is_some_and(|turns| !turns.is_empty())
}

/// 加载之前的对话轮次
///
/// 从缓存中获取历史对话，并进行规范化处理。如果是首次对话（无历史记录），
/// 会将记忆上下文注入到用户消息中，为 AI 提供相关背景信息。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文
/// - `history_key`: 历史记录的查询键
/// - `user_turn_content`: 当前用户消息内容
/// - `had_prior_history`: 是否之前已存在历史记录
///
/// # 返回值
///
/// 返回规范化的对话轮次列表（`ChatMessage` 向量）
///
/// # 记忆上下文注入
///
/// 当 `had_prior_history` 为 `false` 时，会：
/// 1. 调用 `build_memory_context` 获取相关记忆
/// 2. 将记忆上下文追加到最后一轮用户消息的开头
///
/// 这样 AI 在首次对话时就能获得用户的相关背景信息。
///
/// # 示例流程
///
/// ```ignore
/// let history_key = format!("{}_{}", channel_id, user_id);
/// let had_prior = has_prior_history(&ctx, &history_key);
/// let turns = load_prior_turns(&ctx, &history_key, &user_msg, had_prior).await;
/// // turns 可直接用于构建 AI 请求
/// ```
pub(crate) async fn load_prior_turns(
    ctx: &ChannelRuntimeContext,
    history_key: &str,
    user_turn_content: &str,
    had_prior_history: bool,
) -> Vec<ChatMessage> {
    // 从缓存获取原始历史记录
    let prior_turns_raw = ctx
        .conversation_histories
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(history_key)
        .cloned()
        .unwrap_or_default();

    // 规范化历史记录格式
    let mut prior_turns = normalize_cached_channel_turns(prior_turns_raw);

    // 首次对话时注入记忆上下文
    if !had_prior_history {
        let memory_context =
            build_memory_context(ctx.memory.as_ref(), user_turn_content, ctx.min_relevance_score)
                .await;
        // 如果最后一轮是用户消息，在其前面追加记忆上下文
        if let Some(last_turn) = prior_turns.last_mut() {
            if last_turn.role == "user" && !memory_context.is_empty() {
                last_turn.content = format!("{memory_context}{user_turn_content}");
            }
        }
    }

    prior_turns
}

#[cfg(test)]
#[path = "message_helpers_tests.rs"]
mod message_helpers_tests;
