//! 网关聊天模块
//!
//! 本模块提供了用于 Webhook 端点和通道处理器的聊天功能实现。
//! 根据使用场景的不同，提供了两种不同级别的聊天接口：
//!
//! - **简单聊天** (`run_gateway_chat_simple`)：不使用工具的基础聊天，用于向后兼容和测试场景
//! - **完整聊天** (`run_gateway_chat_with_tools`)：支持工具调用的完整聊天，用于 WhatsApp、Linq、Nextcloud Talk 等通道处理器
//!
//! 模块还提供了响应净化和消息日志记录等辅助功能，确保网关通信的安全性和可观测性。

use crate::app::agent::multimodal;
use crate::app::agent::multimodal::ocr::{ImageTextExtractor, SessionImageTextExtractor};
use crate::app::agent::providers::{ChatMessage, ProviderCapabilityError};
use crate::app::agent::tools::Tool;
use crate::app::agent::util::truncate_with_ellipsis;
use std::sync::Arc;

use super::state::{AppState, SharedQueryEngine};

async fn session_query_engine(
    state: &AppState,
    session_id: &str,
) -> anyhow::Result<SharedQueryEngine> {
    {
        let engines = state.session_query_engines.lock().await;
        if let Some(engine) = engines.get(session_id) {
            return Ok(Arc::clone(engine));
        }
    }

    let config = state.config.lock().clone();
    let created = Arc::new(tokio::sync::Mutex::new(
        crate::app::agent::agent::loop_::query_engine::QueryEngine::new_with_provider(
            config,
            session_id,
            Some(Arc::clone(&state.provider)),
        )
        .await?,
    ));

    let mut engines = state.session_query_engines.lock().await;
    Ok(Arc::clone(engines.entry(session_id.to_string()).or_insert_with(|| Arc::clone(&created))))
}

pub(crate) async fn invalidate_session_query_engine(state: &AppState, session_id: &str) {
    let mut engines = state.session_query_engines.lock().await;
    engines.remove(session_id);
}

pub(crate) async fn fork_session_query_engine(
    state: &AppState,
    source_session_id: &str,
    target_session_id: &str,
) -> anyhow::Result<()> {
    let source_engine = {
        let engines = state.session_query_engines.lock().await;
        engines.get(source_session_id).cloned()
    };

    let Some(source_engine) = source_engine else {
        return Ok(());
    };

    let snapshot = source_engine.lock().await.snapshot();
    let config = state.config.lock().clone();
    let mut forked_engine =
        crate::app::agent::agent::loop_::query_engine::QueryEngine::new_with_provider(
            config,
            target_session_id,
            Some(Arc::clone(&state.provider)),
        )
        .await?;
    forked_engine.restore_snapshot(snapshot);

    let mut engines = state.session_query_engines.lock().await;
    engines.insert(target_session_id.to_string(), Arc::new(tokio::sync::Mutex::new(forked_engine)));
    Ok(())
}

/// 运行简单的网关聊天（不使用工具）
///
/// 该函数提供了一个基础的聊天接口，适用于 Webhook 端点的向后兼容场景和测试用途。
/// 它不启用任何工具调用功能，仅执行纯文本对话交互。
///
/// # 功能说明
///
/// - 构建工作空间感知的系统提示词，保持与通道行为的一致性
/// - 准备多模态配置支持的消息格式
/// - 调用底层 Provider 进行对话生成
///
/// # 参数
///
/// - `state`：应用状态的共享引用，包含配置、Provider、模型等运行时信息
/// - `message`：用户输入的聊天消息内容
///
/// # 返回值
///
/// 返回 `anyhow::Result<String>`：
/// - `Ok(String)`：模型生成的回复文本
/// - `Err(anyhow::Error)`：在消息准备或调用过程中发生的错误
///
/// # 示例
///
/// ```rust,no_run
/// use crate::app::agent::gateway::state::AppState;
///
/// async fn handle_webhook(state: &AppState) -> anyhow::Result<String> {
///     let user_message = "你好，请介绍一下你自己";
///     let response = run_gateway_chat_simple(state, user_message).await?;
///     Ok(response)
/// }
/// ```
pub async fn run_gateway_chat_simple(state: &AppState, message: &str) -> anyhow::Result<String> {
    let extractor = SessionImageTextExtractor::new();
    run_gateway_chat_simple_with_extractor(state, message, &extractor).await
}

async fn run_gateway_chat_simple_with_extractor(
    state: &AppState,
    message: &str,
    extractor: &dyn ImageTextExtractor,
) -> anyhow::Result<String> {
    // 构造用户消息列表
    let user_messages = vec![ChatMessage::user(message)];

    // 构建系统提示词
    // 通过注入工作空间感知的系统上下文，保持 webhook/gateway 提示词与通道行为一致
    let (system_prompt, protocol_messages) = {
        let config_guard = state.config.lock();
        let timezone = crate::app::agent::util::locale::parse_timezone(
            &config_guard.app_ui.system_settings.timezone,
        )?;
        let system_prompt = crate::app::agent::channels::build_system_prompt(
            &config_guard.workspace_dir,
            &state.model,
            &[], // tools - 简单聊天不使用工具
            &[], // skills stay outside the system prompt
            Some(&config_guard.identity),
            None, // bootstrap_max_chars - 使用默认值
            timezone,
        );
        let skills = crate::app::agent::skills::load_skills_with_config(
            &config_guard.workspace_dir,
            &config_guard,
        );
        let protocol_messages = crate::app::agent::channels::build_workspace_protocol_messages(
            &config_guard.workspace_dir,
            &skills,
            false,
            8_192,
        );
        (system_prompt, protocol_messages)
    };

    // 协议元消息只在本次 provider 请求中存在；简单网关禁用 Skill Tool，因此仅注入 AGENTS。
    let mut messages = Vec::with_capacity(protocol_messages.len() + 1 + user_messages.len());
    messages.extend(protocol_messages);
    messages.push(ChatMessage::system(system_prompt));
    messages.extend(user_messages);

    // 获取多模态配置并准备消息格式
    let multimodal_config = state.config.lock().multimodal.clone();
    let image_marker_count = multimodal::count_image_markers(&messages);
    let prepared = if image_marker_count == 0 {
        multimodal::prepare_messages_for_provider(&messages, &multimodal_config).await?
    } else if multimodal_config.ocr.enabled && !multimodal_config.ocr.model_supports_images {
        multimodal::ocr::prepare_messages_with_ocr(&messages, &multimodal_config, extractor).await?
    } else if state.provider.supports_vision() {
        multimodal::prepare_messages_for_provider(&messages, &multimodal_config).await?
    } else {
        return Err(ProviderCapabilityError {
            provider: "gateway".to_string(),
            capability: "vision".to_string(),
            message: format!(
                "received {image_marker_count} image marker(s), but this provider does not support vision input"
            ),
        }
        .into());
    };

    // 调用 Provider 执行对话生成
    state.provider.chat_with_history(&prepared.messages, &state.model, state.temperature).await
}

/// 运行带工具的完整网关聊天
///
/// 该函数提供了功能完整的聊天接口，支持工具调用能力。
/// 主要用于通道处理器（如 WhatsApp、Linq、Nextcloud Talk）的消息处理。
///
/// # 功能说明
///
/// - 委托给核心 Agent 处理器 `process_message` 进行完整的消息处理流程
/// - 支持会话级别的上下文管理和工具调用
/// - 集成完整的配置、工具、技能和记忆系统
///
/// # 参数
///
/// - `state`：应用状态的共享引用，包含配置、Provider、模型等运行时信息
/// - `message`：用户输入的聊天消息内容
/// - `session_id`：会话标识符，用于维持对话上下文和会话状态管理
///
/// # 返回值
///
/// 返回 `anyhow::Result<String>`：
/// - `Ok(String)`：模型生成的回复文本（可能包含工具调用结果）
/// - `Err(anyhow::Error)`：在消息处理过程中发生的错误
///
/// # 示例
///
/// ```rust,no_run
/// use crate::app::agent::gateway::state::AppState;
///
/// async fn handle_whatsapp_message(state: &AppState) -> anyhow::Result<String> {
///     let user_message = "请帮我查询今天的天气";
///     let session_id = "user_123_session";
///     let response = run_gateway_chat_with_tools(state, user_message, session_id).await?;
///     Ok(response)
/// }
/// ```
pub async fn run_gateway_chat_with_tools(
    state: &AppState,
    message: &str,
    session_id: &str,
) -> anyhow::Result<String> {
    let engine = session_query_engine(state, session_id).await?;
    let mut engine = engine.lock_owned().await;
    let response = engine.submit_message(message).await;

    if response.is_ok() {
        let session_state = engine.session_state();
        let usage = session_state.usage.as_ui_token_usage();
        tracing::debug!(
            session_id,
            turn_count = session_state.turn_count,
            llm_calls = session_state.usage.llm_calls,
            total_tokens = session_state.usage.total_tokens(),
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            cached_tokens = usage.cached_tokens,
            reasoning_tokens = usage.reasoning_tokens,
            non_system_messages = session_state.budget.non_system_messages,
            remaining_history_messages = session_state.budget.remaining_history_messages,
            max_history_messages = session_state.budget.max_history_messages,
            "gateway chat query engine session state"
        );
    }

    response
}

/// 净化网关响应内容
///
/// 该函数对生成的响应内容进行安全净化处理，移除或转义可能存在的恶意内容、
/// 工具调用残留或其他不安全元素。如果净化后内容为空但原始内容非空，
/// 则返回友好的错误提示消息。
///
/// # 参数
///
/// - `response`：待净化的原始响应字符串
/// - `tools`：可用工具列表，用于识别和处理工具调用相关内容
///
/// # 返回值
///
/// 返回 `String`：
/// - 净化后的安全响应内容
/// - 如果净化导致内容丢失，则返回友好的错误提示消息
///
/// # 示例
///
/// ```rust,no_run
/// use crate::app::agent::tools::Tool;
///
/// fn process_response(response: &str, tools: &[Box<dyn Tool>]) -> String {
///     let safe_response = sanitize_gateway_response(response, tools);
///     safe_response
/// }
/// ```
pub fn sanitize_gateway_response(response: &str, tools: &[Box<dyn Tool>]) -> String {
    // 调用通道响应净化函数进行内容清理
    let sanitized = crate::app::agent::channels::sanitize_channel_response(response, tools);
    let sanitized = truncate_with_ellipsis(&sanitized, 16_000);
    let sanitized =
        sanitized.strip_suffix("...").map_or(sanitized.clone(), |prefix| format!("{prefix}…"));
    // 检查净化结果：如果净化后为空但原始内容非空，说明发生了内容丢失
    if sanitized.is_empty() && !response.trim().is_empty() {
        // 返回友好的错误提示，避免返回空响应
        "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
            .to_string()
    } else {
        sanitized
    }
}

/// 记录通道消息日志
///
/// 该函数用于记录来自不同通道的消息，便于调试和监控。
/// 消息内容会被截断以避免日志过长，同时保留足够的信息用于追踪。
///
/// # 参数
///
/// - `channel`：通道名称（如 "WhatsApp"、"Linq"、"Nextcloud Talk"）
/// - `sender`：消息发送者标识（用户名、用户ID等）
/// - `content`：消息内容（将被截断至50字符以避免日志过长）
///
/// # 日志级别
///
/// 使用 `INFO` 级别记录，确保在正常运行时可见
///
/// # 示例
///
/// ```rust,no_run
/// fn log_incoming_message() {
///     let channel = "WhatsApp";
///     let sender = "user_123";
///     let content = "这是一条很长的测试消息内容，但只会显示前50个字符...";
///     log_channel_message(channel, sender, content);
///     // 日志输出：INFO WhatsApp message from user_123: 这是一条很长的测试消息内容，但只会显示前50个...
/// }
/// ```
pub fn log_channel_message(channel: &str, sender: &str, content: &str) {
    // 使用 tracing::info! 记录消息日志，内容截断至50字符避免日志过长
    tracing::info!("{} message from {}: {}", channel, sender, truncate_with_ellipsis(content, 50));
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
