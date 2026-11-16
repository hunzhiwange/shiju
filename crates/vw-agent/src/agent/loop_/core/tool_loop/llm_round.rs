//! 执行工具循环中的单轮 LLM 请求并解析模型输出。
//!
//! 本模块负责 provider 调用、可观测性事件、取消处理、原生/文本工具调用解析，
//! 以及构造后续历史所需的助手消息内容。

use crate::app::agent::multimodal::ocr::{ImageTextExtractor, SessionImageTextExtractor};
use crate::app::agent::observability::Observer;
use model_types::{ChatMessage, Provider};
use crate::app::agent::tools::ToolSpec;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use super::super::super::parsing::{
    ParsedToolCall, detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls,
};
use super::super::super::utils::scrub_credentials;
use super::super::history::{
    build_native_assistant_history, build_native_assistant_history_from_parsed_calls,
};

#[cfg(test)]
#[path = "llm_round_tests.rs"]
mod llm_round_tests;

/// 单轮 LLM 响应的规范化结果。
///
/// 字段同时服务 UI 展示、工具执行、历史回放和诊断追踪，因此保留原始响应、
/// 展示文本、解析后的工具调用和原生工具调用等不同视角。
pub(super) type LlmRoundResult = agent_loop::core::llm_round::LlmRoundResult<ParsedToolCall>;

/// 执行一次 LLM 请求并将响应转换为工具循环可消费的结构。
///
/// 参数包含 provider、对话历史、观测器、模型配置、工具声明和取消令牌。
/// 返回值是标准化后的 `LlmRoundResult`；当 provider 不支持当前输入能力、
/// 请求失败或取消令牌触发时返回错误。
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_llm_round(
    provider: &dyn Provider,
    history: &[ChatMessage],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    channel_name: &str,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    cancellation_token: Option<&CancellationToken>,
    hooks: Option<&crate::app::agent::hooks::HookRunner>,
    tool_specs: &[ToolSpec],
    use_native_tools: bool,
    turn_id: &str,
    iteration: usize,
) -> Result<LlmRoundResult> {
    let extractor = SessionImageTextExtractor::new();
    run_llm_round_with_extractor(
        provider,
        history,
        observer,
        provider_name,
        model,
        temperature,
        channel_name,
        multimodal_config,
        cancellation_token,
        hooks,
        tool_specs,
        use_native_tools,
        turn_id,
        iteration,
        &extractor,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_llm_round_with_extractor(
    provider: &dyn Provider,
    history: &[ChatMessage],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    channel_name: &str,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    cancellation_token: Option<&CancellationToken>,
    hooks: Option<&crate::app::agent::hooks::HookRunner>,
    tool_specs: &[ToolSpec],
    use_native_tools: bool,
    turn_id: &str,
    iteration: usize,
    extractor: &dyn ImageTextExtractor,
) -> Result<LlmRoundResult> {
    agent_loop::core::llm_round::run_llm_round_with_extractor(&services(), provider, history, observer, provider_name, model, temperature, channel_name, multimodal_config, cancellation_token, hooks, tool_specs, use_native_tools, turn_id, iteration, extractor).await
}

fn services() -> agent_loop::core::llm_round::LlmServices<ParsedToolCall> {
    agent_loop::core::llm_round::LlmServices {
        parse_structured: parse_structured_tool_calls, parse_text: parse_tool_calls,
        detect_issue: detect_tool_call_parse_issue, call_id_mut: |call| &mut call.tool_call_id,
        history_parsed: build_native_assistant_history_from_parsed_calls,
        history_native: build_native_assistant_history, scrub_credentials,
        token_usage: crate::plugins::hook_model::token_usage,
        emit_model: |status, usage| Box::pin(crate::plugins::hook_model::emit(status, usage)),
    }
}
