#[cfg(not(target_arch = "wasm32"))]
use aisdk::core::DynamicModel;
#[cfg(not(target_arch = "wasm32"))]
use aisdk::providers::{AlibabaCn, OpenAICompatible};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::provider::provider;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::logging::LOGGER;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::session::llm::types::{Error, StreamEvent};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::tools;

#[cfg(not(target_arch = "wasm32"))]
use super::convert::AisdkRequestInfo;
#[cfg(not(target_arch = "wasm32"))]
use super::error::{aisdk_assistant_error_log_fields, assistant_error_from_aisdk};
#[cfg(not(target_arch = "wasm32"))]
use super::stream::do_stream_request_aisdk_with_model;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverKind {
    AlibabaCn,
    OpenAICompatible,
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_driver_kind(provider_id: &str, adapter: &str) -> DriverKind {
    if provider_id.eq_ignore_ascii_case("alibaba-cn")
        && adapter.trim().eq_ignore_ascii_case("openai-compatible")
    {
        DriverKind::AlibabaCn
    } else {
        DriverKind::OpenAICompatible
    }
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
    match resolve_driver_kind(provider_id, &model.api.adapter) {
        DriverKind::AlibabaCn => {
            let mut builder = AlibabaCn::<DynamicModel>::builder()
                .provider_name(provider_id.to_string())
                .api_key(bearer.to_string())
                .model_name(model.api.id.clone())
                .base_url(request_info.base_url.clone());

            if let Some(path) = request_info.path_override.clone() {
                builder = builder.path(path);
            }

            let lm = builder.build().map_err(|error| {
                log_build_failed(provider_id, model, &request_info.request_url, error)
            })?;

            do_stream_request_aisdk_with_model(
                provider_id.to_string(),
                lm,
                request_info.request_url,
                request_info.enforce_strict_tool_schema,
                request_info.messages,
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
        }
        DriverKind::OpenAICompatible => {
            let mut builder = OpenAICompatible::<DynamicModel>::builder()
                .provider_name(provider_id.to_string())
                .api_key(bearer.to_string())
                .model_name(model.api.id.clone())
                .base_url(request_info.base_url.clone());

            if let Some(path) = request_info.path_override.clone() {
                builder = builder.path(path);
            }

            let lm = builder.build().map_err(|error| {
                log_build_failed(provider_id, model, &request_info.request_url, error)
            })?;

            do_stream_request_aisdk_with_model(
                provider_id.to_string(),
                lm,
                request_info.request_url,
                request_info.enforce_strict_tool_schema,
                request_info.messages,
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
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "driver_tests.rs"]
mod driver_tests;
