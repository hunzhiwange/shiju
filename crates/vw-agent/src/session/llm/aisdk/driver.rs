//! Host composition for the canonical AI SDK implementation.
#[cfg(test)]
use llm_runtime::aisdk::driver::*;
use super::host::NativeHost;
use crate::session::llm::types::{Error, StreamEvent};
use crate::provider::provider;
use crate::tools;
use serde_json::Value;
use std::collections::HashMap;
use super::convert::AisdkRequestInfo;
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
    llm_runtime::aisdk::driver::dispatch_stream_request(&NativeHost, provider_id, model, bearer, request_info, tools, temperature, top_p, max_output_tokens, merged_options, retries, abort, on_event).await
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../../libs/llm-runtime/src/aisdk/host_tests/driver_tests.rs"]
mod driver_tests;
