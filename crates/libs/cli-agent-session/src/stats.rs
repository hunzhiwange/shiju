//! CLI 会话统计模块
//!
//! 本模块提供 CLI 交互会话的统计信息收集与展示功能。
//! 主要用于追踪和展示用户与代理之间的交互数据，包括：
//! - 消息计数（用户消息、助手消息）
//! - 工具调用事件计数
//! - Token 使用统计（输入和输出）
//!
//! # 主要组件
//!
//! - [`CliStats`][]: 会话统计数据的容器结构体
//! - [`build_session_title`][]: 根据统计数据构建会话标题的函数

/// CLI 会话统计信息
///
/// 用于收集和存储 CLI 会话期间的各项统计数据。
/// 该结构体通过 `Default` trait 提供零值初始化，所有字段从零开始计数。
///
/// # 字段说明
///
/// - `user_messages`: 用户发送的消息总数
/// - `assistant_messages`: 助手（AI）生成的消息总数
/// - `tool_events`: 工具调用事件的总次数
/// - `input_tokens`: 输入 token 的累计使用量
/// - `output_tokens`: 输出 token 的累计使用量
///
/// # 示例
///
/// ```ignore
/// use crate::stats::CliStats;
///
/// let mut stats = CliStats::default();
/// stats.user_messages += 1;
/// stats.input_tokens += 150;
/// stats.output_tokens += 300;
/// ```
#[derive(Default)]
pub(crate) struct CliStats {
    /// 用户发送的消息数量
    pub(crate) user_messages: usize,

    /// 助手生成的消息数量
    pub(crate) assistant_messages: usize,

    /// 工具调用事件的总次数
    pub(crate) tool_events: usize,

    /// 输入 token 的累计使用量（来自 LLM API 响应）
    pub(crate) input_tokens: u64,

    /// 输出 token 的累计使用量（来自 LLM API 响应）
    pub(crate) output_tokens: u64,
}

/// 构建会话标题字符串
///
/// 根据统计数据和模型信息生成格式化的会话标题，
/// 用于在 CLI 界面中展示当前会话的概览信息。
///
/// # 参数
///
/// - `stats`: 包含会话统计信息的 `CliStats` 引用
/// - `provider_name`: LLM 提供商名称（如 "openai"、"anthropic"）
/// - `model_name`: 使用的模型名称（如 "gpt-4"、"claude-3"）
///
/// # 返回值
///
/// 返回一个格式化的多行字符串，包含以下信息：
/// ```text
/// Session
/// Context {total_tokens} tokens
/// {provider_name} / {model_name}
/// ```
///
/// 当总 token 数为零时，第二行显示为 "Context --"。
///
/// # 示例
///
/// ```ignore
/// use crate::stats::{CliStats, build_session_title};
///
/// let stats = CliStats {
///     user_messages: 2,
///     assistant_messages: 2,
///     tool_events: 3,
///     input_tokens: 500,
///     output_tokens: 1200,
/// };
///
/// let title = build_session_title(&stats, "openai", "gpt-4");
/// assert!(title.contains("Session"));
/// assert!(title.contains("1700 tokens"));
/// assert!(title.contains("openai / gpt-4"));
/// ```
pub(crate) fn build_session_title(
    stats: &CliStats,
    provider_name: &str,
    model_name: &str,
) -> String {
    // 使用 saturating_add 防止 token 计数溢出
    let total_tokens = stats.input_tokens.saturating_add(stats.output_tokens);

    // 根据是否有 token 使用数据，选择显示格式
    // - 有 token: 显示具体数值
    // - 无 token: 显示占位符 "--"
    let context_line = if total_tokens == 0 {
        "Context --".to_string()
    } else {
        format!("Context {total_tokens} tokens")
    };

    // 构建最终的多行标题字符串
    // 格式：Session / Context 信息 / 提供商和模型
    format!("Session\n{context_line}\n{provider_name} / {model_name}")
}
