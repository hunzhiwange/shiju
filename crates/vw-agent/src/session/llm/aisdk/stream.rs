//! Host composition for the canonical AI SDK implementation.
#[cfg(test)]
use llm_runtime::aisdk::stream::*;
use super::host::NativeHost;
use crate::session::llm::types::{Error, StreamEvent};
use crate::tools;
use serde_json::Value;
use std::collections::HashMap;
use aisdk::core::capabilities::ToolCallSupport;
pub(crate) async fn do_stream_request_aisdk_with_model<
    M: aisdk::core::language_model::LanguageModel + ToolCallSupport,
>(
    provider_id: String,
    model: M,
    request_url: String,
    enforce_strict_tool_schema: bool,
    native_tool_search: bool,
    messages: Vec<aisdk::core::Message>,
    wire_messages: Option<Value>,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    merged_options: &Value,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    llm_runtime::aisdk::stream::do_stream_request_aisdk_with_model(&NativeHost, provider_id, model, request_url, enforce_strict_tool_schema, native_tool_search, messages, wire_messages, tools, temperature, top_p, max_output_tokens, merged_options, retries, abort, on_event).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/stream_tests.rs"]
mod stream_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/truncation_tests.rs"]
mod truncation_tests;
