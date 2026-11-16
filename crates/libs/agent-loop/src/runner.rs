//! Agent 消息处理运行器模块
//!
//! 本模块保留兼容入口，供通道层（如 Telegram、Discord 等）直接处理单条消息。
//! 实际的会话级装配和多轮状态维护已下沉到 `query_engine` 模块。
//!
//! # 主要功能
//!
//! - 为现有调用方保留 `process_message(config, message, session_id)` 入口
//! - 将单消息请求桥接到会话级 `QueryEngine`
//! - 避免通道层感知底层编排分层调整
//!
//! # 使用场景
//!
//! 当外部通道（如 Telegram Bot、Discord Bot）需要处理用户消息并启用完整的
//! agent 能力（包括工具调用、记忆、安全策略等）时，使用本模块提供的
//! [`process_message`] 函数。

use agent_config::runtime::Config;
use anyhow::Result;

use super::query_engine::QueryEngine;

/// 通过兼容包装处理单条消息
///
/// 该函数保留原有调用签名，内部构建一个短生命周期 `QueryEngine` 并提交单条消息。
/// 调用方后续如果需要多轮复用 provider、工具和历史，可以直接改为持有 `QueryEngine`。
///
/// # 参数
///
/// * `config` - Agent 配置对象，包含所有运行时参数设置
/// * `message` - 用户输入的原始消息文本
/// * `session_id` - 会话标识符，用于隔离不同用户或会话的记忆和状态
///
/// # 返回值
///
/// * `Ok(String)` - Agent 最终生成的文本响应
/// * `Err(anyhow::Error)` - 处理过程中发生的任何错误
///
/// # 处理流程
///
/// 1. 基于配置创建会话级 `QueryEngine`
/// 2. 向引擎提交当前用户消息
/// 3. 返回最终文本响应
///
/// # 示例
///
/// ```ignore
/// use agent_config::runtime::Config;
/// use agent_loop::runner::process_message;
///
/// async fn handle_telegram_message(config: Config, user_input: &str, user_id: &str) {
///     match process_message(config, user_input, user_id).await {
///         Ok(response) => send_to_telegram(&response).await,
///         Err(e) => eprintln!("处理消息失败: {}", e),
///     }
/// }
/// ```
///
/// # 注意事项
///
/// - 该入口仍保持单消息调用模型，兼容现有网关/通道代码
/// - 若需要真正的多轮会话复用，请直接持有 `QueryEngine`
pub async fn process_message<H: super::query_engine::QueryEngineHost>(config: Config, message: &str, session_id: &str) -> Result<String> {
    let mut engine = QueryEngine::<H>::new(config, session_id).await?;
    engine.submit_message(message).await
}
