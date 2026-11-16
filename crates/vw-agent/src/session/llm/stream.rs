//! Stream composition with the application's existing transport and hook contexts.
use super::types::{Error, StreamEvent, StreamInput};
struct StreamHost;
impl llm_runtime::stream::Host for StreamHost {
    #[cfg(not(target_arch = "wasm32"))]
    type Acp = super::acp::request::RuntimeHost;
    #[cfg(not(target_arch = "wasm32"))]
    type Sdk = super::aisdk::host::NativeHost;
    #[cfg(not(target_arch = "wasm32"))]
    fn sdk_host() -> Self::Sdk { super::aisdk::host::NativeHost }
    fn project_id() -> Option<String> { crate::project::instance::project().map(|p| p.id) }
    fn client_id() -> String { crate::flag::vibewindow_client() }
    fn emit_model(status: Option<&'static str>, usage: serde_json::Value)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(crate::plugins::hook_model::emit::<crate::tools::context::AgentContextHost>(status, usage))
    }
}
pub async fn stream(input: StreamInput, callback: impl FnMut(StreamEvent) + Send + 'static) -> Result<(), Error> {
    super::prompt::ensure_host();
    llm_runtime::stream::stream::<StreamHost>(input, callback).await
}
#[cfg(test)]
use llm_runtime::stream::*;
#[cfg(test)]
use crate::provider::provider;
#[cfg(test)]
use crate::session::message;
#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
#[path = "../../../../libs/llm-runtime/tests/host_suites/stream_tests.rs"]
mod stream_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../libs/llm-runtime/tests/host_suites/transport_tests.rs"]
mod transport_tests;
