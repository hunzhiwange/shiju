//! Typed result of one model step.
use session_port::ui_types as models;
use crate::processor_events as llm;

/// LLM 步骤执行结果
///
/// 封装单次 LLM 调用的完整输出结果，包括令牌使用统计、完成原因、推理内容、
/// 文本输出、工具调用以及完整的消息历史。
///
/// # 字段说明
///
/// * `usage` - 令牌使用统计，包括输入和输出令牌数量
/// * `finish_reason` - 模型完成响应的原因（如 "stop"、"length" 等）
/// * `reasoning_content` - 模型的推理/思考过程内容
/// * `text` - 模型生成的文本内容
/// * `tool_calls` - 模型请求执行的工具调用列表
/// * `full_messages` - 完整的消息历史记录，用于上下文追溯
///
/// # 示例
///
/// ```ignore
/// let step = run_llm_step(&messages, &system, model, &tools, &mut on_event)?;
/// println!("生成的文本: {}", step.text);
/// println!("使用了 {} 个令牌", step.usage.total_tokens);
/// ```
#[derive(Debug, Clone)]
pub struct LlmStep {
    /// 令牌使用统计信息
    pub usage: models::TokenUsage,
    /// 响应完成的原因（可选）
    pub finish_reason: Option<String>,
    /// 模型的推理/思考内容
    pub reasoning_content: String,
    /// 模型生成的主要文本内容
    pub text: String,
    /// 模型请求执行的工具调用列表
    pub tool_calls: Vec<llm::ToolCall>,
    /// 完整的消息历史，包含所有上下文信息
    pub full_messages: Vec<serde_json::Value>,
}
