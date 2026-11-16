//! Host composition for the canonical AI SDK implementation.
#[cfg(test)]
use llm_runtime::aisdk::request::*;
use super::host::NativeHost;
use crate::session::llm::types::{Error, StreamEvent};
use crate::provider::provider;
use crate::tools;
use serde_json::Value;
use std::collections::HashMap;
use agent_auth as auth;
pub async fn do_stream_request_aisdk(
    provider_info: &provider::Info,
    auth_info: Option<&auth::Info>,
    headers: &HashMap<String, String>,
    merged_options: &Value,
    model: &provider::Model,
    chat_messages: &Value,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (impl FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    llm_runtime::aisdk::request::do_stream_request_aisdk(&NativeHost, provider_info, auth_info, headers, merged_options, model, chat_messages, tools, temperature, top_p, max_output_tokens, retries, abort, on_event).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/request_tests.rs"]
mod request_tests;
