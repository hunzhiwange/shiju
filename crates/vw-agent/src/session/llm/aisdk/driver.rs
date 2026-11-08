//! AI SDK 模型解析与流式请求驱动。
//!
//! 非 WASM 平台在此把统一会话请求转换为供应商模型调用，并归一化错误与输出。

#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::DynamicModel;
#[cfg(not(target_arch = "wasm32"))]
use aisdk::providers::{
    OpenAICompatible, OpenAICompatibleResolveOptions, resolve_openai_compatible_model,
};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::provider::provider;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::logging::LOGGER;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::messages::has_tool_calls;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::types::{Error, StreamEvent};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::message::AssistantError;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::tools;

#[cfg(not(target_arch = "wasm32"))]
use super::convert::AisdkRequestInfo;
#[cfg(not(target_arch = "wasm32"))]
use super::error::{aisdk_assistant_error_log_fields, assistant_error_from_aisdk};
#[cfg(not(target_arch = "wasm32"))]
use super::stream::do_stream_request_aisdk_with_model;

#[cfg(not(target_arch = "wasm32"))]
fn resolve_model(
    provider_id: &str,
    model: &provider::Model,
    bearer: &str,
    request_info: &AisdkRequestInfo,
) -> aisdk::Result<OpenAICompatible<DynamicModel>> {
    resolve_openai_compatible_model(
        &format!("{provider_id}/{}", model.api.id),
        OpenAICompatibleResolveOptions {
            api_key: Some(bearer.to_string()),
            base_url: Some(request_info.base_url.clone()),
            path: request_info.path_override.clone(),
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn log_build_failed(
    provider_id: &str,
    model: &provider::Model,
    request_url: &str,
    error: aisdk::Error,
) -> Error {
    let assistant_error = assistant_error_from_aisdk(provider_id, error);
    LOGGER.clone_logger().tag("providerID", provider_id).tag("modelID", &model.api.id).error(
        "aisdk.model.build_failed",
        Some({
            let mut m = aisdk_assistant_error_log_fields(&assistant_error);
            m.insert("requestURL".to_string(), Value::String(request_url.to_string()));
            m
        }),
    );
    Error::Api(assistant_error)
}

/// Check the selected catalog row before message conversion or image preparation.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn validate_catalog_request(
    model: &provider::Model,
    messages: &[Value],
    tools: &HashMap<String, tools::ToolSpec>,
    options: &Value,
) -> Result<(), AssistantError> {
    let Some(declared) = &model.capabilities.catalog_execution else {
        return Ok(());
    };
    let capability_error = |capability| {
        assistant_error_from_aisdk(
            &model.provider_id,
            aisdk::Error::InvalidInput(format!(
                "model {}/{} does not support {capability}",
                model.provider_id, model.api.id
            )),
        )
    };

    if declared.tool_calls_unsupported
        && (!tools.is_empty()
            || has_tool_calls(messages)
            || messages
                .iter()
                .any(|message| matches!(message["role"].as_str(), Some("tool" | "native"))))
    {
        return Err(capability_error("tool calls"));
    }
    if let Some(input) = &declared.input {
        for message in messages.iter().filter(|message| message["role"] == "user") {
            if let Some(modality) = unsupported_input(input, &message["content"]) {
                return Err(capability_error(modality));
            }
        }
    }
    super::reasoning::validate_reasoning(model, options)
        .map_err(|error| assistant_error_from_aisdk(&model.provider_id, error))
}

#[cfg(not(target_arch = "wasm32"))]
fn unsupported_input(input: &provider::CapabilityIO, content: &Value) -> Option<&'static str> {
    if let Some(content) = content.as_str() {
        if input.image && input.text {
            return None;
        }
        let (text, images) = crate::multimodal::parse_image_markers(content);
        if !input.image && !images.is_empty() {
            return Some("image input");
        }
        return (!input.text && !text.is_empty()).then_some("text input");
    }

    content.as_array()?.iter().find_map(|part| match part["type"].as_str()? {
        "image_url" | "image" | "input_image" if !input.image => Some("image input"),
        "audio" | "input_audio" if !input.audio => Some("audio input"),
        "video" | "video_url" | "input_video" if !input.video => Some("video input"),
        "pdf" | "file" | "input_file" | "document" if !input.pdf => Some("PDF input"),
        "text" | "input_text"
            if !input.text && part["text"].as_str().is_some_and(|text| !text.trim().is_empty()) =>
        {
            Some("text input")
        }
        _ => None,
    })
}

/// 解析提供方模型并分派 AI SDK 流式请求。
///
/// 模型构建和流处理错误会转换为统一助手错误，同时避免在日志中输出认证令牌。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn dispatch_stream_request(
    provider_id: &str,
    model: &provider::Model,
    bearer: &str,
    request_info: AisdkRequestInfo,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    merged_options: &Value,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (impl FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    if model.capabilities.catalog_execution.is_none() {
        super::reasoning::validate_reasoning(model, merged_options).map_err(|error| {
            log_build_failed(provider_id, model, &request_info.request_url, error)
        })?;
    }
    macro_rules! send_model {
        ($resolved:expr) => {{
            let lm = $resolved.map_err(|error| {
                log_build_failed(provider_id, model, &request_info.request_url, error)
            })?;
            do_stream_request_aisdk_with_model(
                provider_id.to_string(),
                lm,
                request_info.request_url,
                request_info.enforce_strict_tool_schema,
                provider::resolve_native_search_capabilities(
                    &model.capabilities,
                    &model.api.adapter,
                    request_info.path_override.as_deref().unwrap_or("chat/completions"),
                )
                .native_tool_search,
                request_info.messages,
                request_info.wire_messages,
                tools,
                temperature,
                top_p,
                max_output_tokens,
                merged_options,
                retries,
                abort,
                on_event,
            )
            .await
        }};
    }
    if request_info.request_url.ends_with("/messages") {
        return send_model!(
            aisdk::providers::Anthropic::<DynamicModel>::builder()
                .model_name(model.api.id.clone())
                .api_key(bearer)
                .base_url(request_info.base_url.clone())
                .build()
        );
    }
    if request_info.request_url.ends_with("/responses") {
        return send_model!(
            aisdk::providers::OpenAI::<DynamicModel>::builder()
                .path("/responses")
                .model_name(model.api.id.clone())
                .api_key(bearer)
                .base_url(request_info.base_url.clone())
                .build()
        );
    }
    send_model!(resolve_model(provider_id, model, bearer, &request_info))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "driver_tests.rs"]
mod driver_tests;
