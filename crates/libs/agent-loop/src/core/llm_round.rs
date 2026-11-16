//! 执行工具循环中的单轮 LLM 请求并解析模型输出。
//!
//! 本模块负责 provider 调用、可观测性事件、取消处理、原生/文本工具调用解析，
//! 以及构造后续历史所需的助手消息内容。

use agent_multimodal as multimodal;
use agent_multimodal::ocr::ImageTextExtractor;
use agent_observability::{Observer, ObserverEvent, runtime_trace};
use model_types::{ChatMessage, ChatRequest, Provider, ProviderCapabilityError, ToolCall};
use agent_providers::sanitize_api_error;
use tool_types::ToolSpec;
use agent_util::truncate_with_ellipsis;
use anyhow::Result;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use super::errors::ToolLoopCancelled;

/// 单轮 LLM 响应的规范化结果。
///
/// 字段同时服务 UI 展示、工具执行、历史回放和诊断追踪，因此保留原始响应、
/// 展示文本、解析后的工具调用和原生工具调用等不同视角。
pub struct LlmRoundResult<T> {
    pub response_text: String,
    pub display_text: String,
    pub tool_calls: Vec<T>,
    pub assistant_history_content: String,
    pub native_tool_calls: Vec<ToolCall>,
    pub parse_issue_detected: bool,
    pub duration_secs: u64,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_llm_round_with_extractor<T>(
    services: &LlmServices<T>,
    provider: &dyn Provider,
    history: &[ChatMessage],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    channel_name: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    cancellation_token: Option<&CancellationToken>,
    hooks: Option<&agent_hooks::HookRunner>,
    tool_specs: &[ToolSpec],
    use_native_tools: bool,
    turn_id: &str,
    iteration: usize,
    extractor: &dyn ImageTextExtractor,
) -> Result<LlmRoundResult<T>> {
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
    (services.emit_model)(None, serde_json::Value::Null).await;
    let chat_future = provider.chat(
        ChatRequest { messages: &prepared_messages.messages, tools: request_tools },
        model,
        temperature,
    );
    let chat_result = if let Some(token) = cancellation_token {
        tokio::select! {
            () = token.cancelled() => {
                (services.emit_model)(Some("cancelled"), serde_json::Value::Null).await;
                return Err(ToolLoopCancelled.into());
            },
            result = chat_future => result,
        }
    } else {
        chat_future.await
    };

    let usage = chat_result
        .as_ref()
        .ok()
        .and_then(|response| response.usage.as_ref())
        .map(|usage| {
            (services.token_usage)(
                i64::try_from(usage.input_tokens.unwrap_or(0)).unwrap_or(i64::MAX),
                i64::try_from(usage.output_tokens.unwrap_or(0)).unwrap_or(i64::MAX),
                i64::try_from(usage.cached_tokens.unwrap_or(0)).unwrap_or(i64::MAX),
                i64::try_from(usage.reasoning_tokens.unwrap_or(0)).unwrap_or(i64::MAX),
            )
        })
        .unwrap_or(serde_json::Value::Null);
    (services.emit_model)(
        Some(if chat_result.is_ok() { "completed" } else { "failed" }),
        usage,
    )
    .await;
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

            if resp
                .tool_calls
                .iter()
                .any(|call| call.kind == model_types::ToolCallKind::Search)
            {
                let mut ids = std::collections::HashSet::new();
                anyhow::ensure!(
                    resp.tool_calls
                        .iter()
                        .all(|call| !call.id.trim().is_empty() && ids.insert(&call.id)),
                    "native search batch requires unique nonempty call ids"
                );
            }
            for call in &resp.tool_calls {
                if call.kind == model_types::ToolCallKind::Search {
                    anyhow::ensure!(
                        use_native_tools && provider.capabilities().native_tool_search,
                        "native tool search is not authorized for this provider request"
                    );
                    anyhow::ensure!(
                        call.name == "tool_search"
                            && call.namespace.is_none()
                            && !call.id.trim().is_empty(),
                        "invalid native search call identity"
                    );
                }
            }
            let response_text = resp.text_or_empty().to_string();
            let mut tool_calls = (services.parse_structured)(&resp.tool_calls);
            let mut parsed_text = String::new();

            if tool_calls.is_empty() {
                // 原生工具调用为空时才回退到文本解析，兼容旧模型输出的
                // <tool_call> 包裹格式。
                let (fallback_text, fallback_calls) = (services.parse_text)(&response_text);
                if !fallback_text.is_empty() {
                    parsed_text = fallback_text;
                }
                tool_calls = fallback_calls;
            }

            if use_native_tools && resp.tool_calls.is_empty() && !tool_calls.is_empty() {
                // 原生工具模式需要稳定的 tool_call_id 来关联工具结果；文本回退没有
                // provider 分配的 id，因此在本地生成可预测 id。
                assign_fallback_tool_call_ids(&mut tool_calls, iteration, services.call_id_mut);
            }

            let parse_issue = (services.detect_issue)(&response_text, &tool_calls);
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
                            &(services.scrub_credentials)(&response_text),
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
                    "raw_response": (services.scrub_credentials)(&response_text),
                    "native_tool_calls": resp.tool_calls.len(),
                    "parsed_tool_calls": tool_calls.len(),
                }),
            );

            let assistant_history_content = build_assistant_history(
                services,
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
pub fn assign_fallback_tool_call_ids<T>(tool_calls: &mut [T], iteration: usize, call_id_mut: fn(&mut T) -> &mut Option<String>) {
    for (index, tool_call) in tool_calls.iter_mut().enumerate() {
        let id = call_id_mut(tool_call);
        if id.is_none() {
            *id = Some(format!("fallback_{}_{}", iteration + 1, index + 1));
        }
    }
}

/// 构造写入会话历史的助手消息内容。
///
/// 返回值会保留 provider 原生工具调用或从文本解析出的工具调用信息，确保后续
/// tool result 能与模型上下文正确配对。
pub fn build_assistant_history<T>(
    services: &LlmServices<T>,
    response_text: &str,
    parsed_calls: &[T],
    native_tool_calls: &[ToolCall],
    reasoning_content: Option<&str>,
    use_native_tools: bool,
) -> String {
    if native_tool_calls.is_empty() {
        if use_native_tools {
            (services.history_parsed)(
                response_text,
                parsed_calls,
                reasoning_content,
            )
            .unwrap_or_else(|| response_text.to_string())
        } else {
            response_text.to_string()
        }
    } else {
        (services.history_native)(response_text, native_tool_calls, reasoning_content)
    }
}

pub fn image_markers_are_data_uris(history: &[ChatMessage]) -> bool {
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

/// 原解析类型通过函数签名贯穿，不复制解析AST，也不接管其它解析任务。
pub struct LlmServices<T> {
    pub parse_structured: fn(&[ToolCall]) -> Vec<T>,
    pub parse_text: fn(&str) -> (String, Vec<T>),
    pub detect_issue: fn(&str, &[T]) -> Option<String>,
    pub call_id_mut: fn(&mut T) -> &mut Option<String>,
    pub history_parsed: fn(&str, &[T], Option<&str>) -> Option<String>,
    pub history_native: fn(&str, &[ToolCall], Option<&str>) -> String,
    pub scrub_credentials: fn(&str) -> String,
    pub token_usage: fn(i64, i64, i64, i64) -> serde_json::Value,
    pub emit_model: for<'a> fn(Option<&'a str>, serde_json::Value) -> ModelEventFuture<'a>,
}
#[cfg(not(target_arch = "wasm32"))]
pub type ModelEventFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type ModelEventFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>;
