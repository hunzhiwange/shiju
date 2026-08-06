//! 执行工具循环中的单轮 LLM 请求并解析模型输出。
//!
//! 本模块负责 provider 调用、可观测性事件、取消处理、原生/文本工具调用解析，
//! 以及构造后续历史所需的助手消息内容。

use crate::app::agent::multimodal;
use crate::app::agent::multimodal::ocr::{ImageTextExtractor, SessionImageTextExtractor};
use crate::app::agent::observability::{Observer, ObserverEvent, runtime_trace};
use crate::app::agent::providers::{
    ChatMessage, ChatRequest, Provider, ProviderCapabilityError, ToolCall, sanitize_api_error,
};
use crate::app::agent::tools::ToolSpec;
use crate::app::agent::util::truncate_with_ellipsis;
use anyhow::Result;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use super::super::super::parsing::{
    ParsedToolCall, detect_tool_call_parse_issue, parse_structured_tool_calls, parse_tool_calls,
};
use super::super::super::utils::scrub_credentials;
use super::super::errors::ToolLoopCancelled;
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
pub(super) struct LlmRoundResult {
    pub(super) response_text: String,
    pub(super) display_text: String,
    pub(super) tool_calls: Vec<ParsedToolCall>,
    pub(super) assistant_history_content: String,
    pub(super) native_tool_calls: Vec<ToolCall>,
    pub(super) parse_issue_detected: bool,
    pub(super) duration_secs: u64,
}

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
    let image_marker_count = multimodal::count_image_markers(history);
    let has_asset_refs = history.iter().any(|message| message.content.contains("[VW_IMAGE:"));
    let pre_normalized = image_markers_are_data_uris(history);
    let prepared_messages = if image_marker_count == 0 {
        multimodal::prepare_messages_for_provider(history, multimodal_config).await?
    } else if (has_asset_refs || pre_normalized) && provider.supports_vision() {
        // 图片上下文已改写：归一化剩余 data URI，跳过逐步 OCR。
        multimodal::prepare_messages_for_provider(history, multimodal_config).await?
    } else if multimodal_config.ocr.enabled {
        multimodal::ocr::prepare_messages_with_ocr(history, multimodal_config, extractor).await?
    } else if provider.supports_vision() {
        multimodal::prepare_messages_for_provider(history, multimodal_config).await?
    } else {
        // 视觉能力必须在请求前显式拒绝，避免把含图片标记的历史发送给不支持
        // 该能力的 provider，造成不可预测的远端错误或内容丢失。
        return Err(ProviderCapabilityError {
            provider: provider_name.to_string(),
            capability: "vision".to_string(),
            message: format!(
                "received {image_marker_count} image marker(s), but this provider does not support vision input"
            ),
        }
        .into());
    };

    observer.record_event(&ObserverEvent::LlmRequest {
        provider: provider_name.to_string(),
        model: model.to_string(),
        messages_count: history.len(),
    });
    runtime_trace::record_event(
        "llm_request",
        Some(channel_name),
        Some(provider_name),
        Some(model),
        Some(turn_id),
        None,
        None,
        serde_json::json!({
            "iteration": iteration + 1,
            "messages_count": history.len(),
        }),
    );

    let llm_started_at = Instant::now();
    if let Some(hooks) = hooks {
        // hook 接收的是原始历史，便于调试用户看到的上下文，而 provider 请求则可能
        // 已经过多模态转换。
        hooks.fire_llm_input(history, model).await;
    }

    let request_tools = if use_native_tools { Some(tool_specs) } else { None };
    let chat_future = provider.chat(
        ChatRequest { messages: &prepared_messages.messages, tools: request_tools },
        model,
        temperature,
    );
    let chat_result = if let Some(token) = cancellation_token {
        tokio::select! {
            () = token.cancelled() => return Err(ToolLoopCancelled.into()),
            result = chat_future => result,
        }
    } else {
        chat_future.await
    };

    match chat_result {
        Ok(resp) => {
            let (resp_input_tokens, resp_output_tokens, resp_cached_tokens, resp_reasoning_tokens) =
                resp.usage
                    .as_ref()
                    .map(|usage| {
                        (
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.cached_tokens,
                            usage.reasoning_tokens,
                        )
                    })
                    .unwrap_or((None, None, None, None));

            observer.record_event(&ObserverEvent::LlmResponse {
                provider: provider_name.to_string(),
                model: model.to_string(),
                duration: llm_started_at.elapsed(),
                success: true,
                error_message: None,
                input_tokens: resp_input_tokens,
                output_tokens: resp_output_tokens,
                cached_tokens: resp_cached_tokens,
                reasoning_tokens: resp_reasoning_tokens,
            });

            let response_text = resp.text_or_empty().to_string();
            let mut tool_calls = parse_structured_tool_calls(&resp.tool_calls);
            let mut parsed_text = String::new();

            if tool_calls.is_empty() {
                // 原生工具调用为空时才回退到文本解析，兼容旧模型输出的
                // <tool_call> 包裹格式。
                let (fallback_text, fallback_calls) = parse_tool_calls(&response_text);
                if !fallback_text.is_empty() {
                    parsed_text = fallback_text;
                }
                tool_calls = fallback_calls;
            }

            if use_native_tools && resp.tool_calls.is_empty() && !tool_calls.is_empty() {
                // 原生工具模式需要稳定的 tool_call_id 来关联工具结果；文本回退没有
                // provider 分配的 id，因此在本地生成可预测 id。
                assign_fallback_tool_call_ids(&mut tool_calls, iteration);
            }

            let parse_issue = detect_tool_call_parse_issue(&response_text, &tool_calls);
            if let Some(issue) = parse_issue.as_ref() {
                runtime_trace::record_event(
                    "tool_call_parse_issue",
                    Some(channel_name),
                    Some(provider_name),
                    Some(model),
                    Some(turn_id),
                    Some(false),
                    Some(issue),
                    serde_json::json!({
                        "iteration": iteration + 1,
                        "response_excerpt": truncate_with_ellipsis(
                            &scrub_credentials(&response_text),
                            600
                        ),
                    }),
                );
            }

            runtime_trace::record_event(
                "llm_response",
                Some(channel_name),
                Some(provider_name),
                Some(model),
                Some(turn_id),
                Some(true),
                None,
                serde_json::json!({
                    "iteration": iteration + 1,
                    "duration_ms": llm_started_at.elapsed().as_millis(),
                    "input_tokens": resp_input_tokens,
                    "output_tokens": resp_output_tokens,
                    "cached_tokens": resp_cached_tokens,
                    "reasoning_tokens": resp_reasoning_tokens,
                    "raw_response": scrub_credentials(&response_text),
                    "native_tool_calls": resp.tool_calls.len(),
                    "parsed_tool_calls": tool_calls.len(),
                }),
            );

            let assistant_history_content = build_assistant_history(
                &response_text,
                &tool_calls,
                &resp.tool_calls,
                resp.reasoning_content.as_deref(),
                use_native_tools,
            );
            let display_text =
                if parsed_text.is_empty() { response_text.clone() } else { parsed_text };

            Ok(LlmRoundResult {
                response_text,
                display_text,
                tool_calls,
                assistant_history_content,
                native_tool_calls: resp.tool_calls,
                parse_issue_detected: parse_issue.is_some(),
                duration_secs: llm_started_at.elapsed().as_secs(),
            })
        }
        Err(error) => {
            let safe_error = sanitize_api_error(&error.to_string());
            observer.record_event(&ObserverEvent::LlmResponse {
                provider: provider_name.to_string(),
                model: model.to_string(),
                duration: llm_started_at.elapsed(),
                success: false,
                error_message: Some(safe_error.clone()),
                input_tokens: None,
                output_tokens: None,
                cached_tokens: None,
                reasoning_tokens: None,
            });
            runtime_trace::record_event(
                "llm_response",
                Some(channel_name),
                Some(provider_name),
                Some(model),
                Some(turn_id),
                Some(false),
                Some(&safe_error),
                serde_json::json!({
                    "iteration": iteration + 1,
                    "duration_ms": llm_started_at.elapsed().as_millis(),
                }),
            );
            Err(error)
        }
    }
}

/// 为文本回退解析出的工具调用补齐稳定 id。
///
/// 参数 `iteration` 用于保证同一轮内 id 可读且跨轮不冲突。
fn assign_fallback_tool_call_ids(tool_calls: &mut [ParsedToolCall], iteration: usize) {
    for (index, tool_call) in tool_calls.iter_mut().enumerate() {
        if tool_call.tool_call_id.is_none() {
            tool_call.tool_call_id = Some(format!("fallback_{}_{}", iteration + 1, index + 1));
        }
    }
}

/// 构造写入会话历史的助手消息内容。
///
/// 返回值会保留 provider 原生工具调用或从文本解析出的工具调用信息，确保后续
/// tool result 能与模型上下文正确配对。
fn build_assistant_history(
    response_text: &str,
    parsed_calls: &[ParsedToolCall],
    native_tool_calls: &[ToolCall],
    reasoning_content: Option<&str>,
    use_native_tools: bool,
) -> String {
    if native_tool_calls.is_empty() {
        if use_native_tools {
            build_native_assistant_history_from_parsed_calls(
                response_text,
                parsed_calls,
                reasoning_content,
            )
            .unwrap_or_else(|| response_text.to_string())
        } else {
            response_text.to_string()
        }
    } else {
        build_native_assistant_history(response_text, native_tool_calls, reasoning_content)
    }
}

fn image_markers_are_data_uris(history: &[ChatMessage]) -> bool {
    let mut found = false;
    for message in history.iter().filter(|message| message.role == "user") {
        for reference in multimodal::parse_image_markers(&message.content).1 {
            found = true;
            if !reference.starts_with("data:") {
                return false;
            }
        }
    }
    found
}
