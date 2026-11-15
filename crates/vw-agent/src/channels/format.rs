//! 通道消息格式化模块
//!
//! 本模块提供用于处理输出到各通道的消息格式化功能，主要职责包括：
//!
//! - **工具调用标签清理**：从 LLM 响应中移除内部协议标签（如 `<function_calls>`、`<toolcall>` 等），
//!   防止将内部协议细节泄露给终端用户
//! - **通道投递指令**：为不同通道（如 Telegram、WhatsApp）提供格式化指导，
//!   确保 LLM 输出符合各通道的渲染特性
//! - **内部详情暴露判断**：根据用户消息内容智能判断是否应展示内部工具调用细节
//! - **工具上下文摘要提取**：从历史消息中提取已使用工具的摘要信息
//! - **通道响应净化**：清理响应中的工具相关 JSON 片段，确保输出干净友好
//!
//! # 设计原则
//!
//! - **安全优先**：默认隐藏内部工具调用细节，仅在用户明确请求时才暴露
//! - **语言感知**：支持中文和英文的意图识别，正确理解否定表达
//! - **通道适配**：针对不同通道的渲染能力提供定制化的格式化指导

use super::*;

pub(crate) use channel_support::format::strip_tool_call_tags;

pub(crate) use channel_support::format::channel_delivery_instructions;

pub(crate) use channel_support::format::should_expose_internal_tool_details;

/// 分离内部进度标记与实际内容
///
/// 检测消息增量（delta）是否以草稿进度标记开头，如果是则分离标记和剩余内容
///
/// # 参数
///
/// - `delta`: 消息增量字符串
///
/// # 返回值
///
/// 返回元组 `(is_draft_progress, content)`:
/// - `is_draft_progress`: 是否为草稿进度消息
/// - `content`: 移除标记后的内容（如果不是进度消息，返回原始 delta）
pub(crate) fn split_internal_progress_delta(delta: &str) -> (bool, &str) {
    channel_support::format::split_internal_progress_delta(
        delta,
        crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL,
        crate::app::agent::agent::loop_::DRAFT_WS_EVENT_SENTINEL,
    )
}

pub(crate) use channel_support::format::extract_tool_context_summary;

/// 净化通道响应，移除工具调用相关内容
///
/// 对响应字符串进行两阶段清理：首先移除工具调用标签，然后清理孤立的工具 JSON 片段
///
/// # 参数
///
/// - `response`: 原始响应字符串
/// - `tools`: 已注册的工具列表，用于识别工具名称
///
/// # 返回值
///
/// 返回净化后的响应字符串
pub(crate) fn sanitize_channel_response(response: &str, tools: &[Box<dyn Tool>]) -> String {
    let visible_response = crate::app::agent::session::child_agent::visible_answer_from_execution_output(response);
    channel_support::format::sanitize_visible_channel_response(&visible_response, tools)
}

#[cfg(test)]
pub(crate) use channel_support::format::is_tool_call_payload;

#[cfg(test)]
pub(crate) use channel_support::format::is_tool_result_payload;

#[cfg(test)]
pub(crate) use channel_support::format::sanitize_tool_json_value;

#[cfg(test)]
pub(crate) use channel_support::format::is_line_isolated_json_segment;

pub(crate) use channel_support::format::strip_isolated_tool_json_artifacts;

#[cfg(test)]
#[path = "format_tests.rs"]
mod format_tests;
