//! Application-owned stream execution, catalog errors and system prompt context.
use std::{future::Future, pin::Pin, sync::OnceLock};
use crate::{types::StreamInput, StreamEvent, Error};
pub type StreamFuture = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;
#[derive(Clone, Copy)]
pub struct Host {
    #[cfg(not(target_arch = "wasm32"))]
    pub annotate_model_source: fn(&mut vw_api_acp_model::acp_session::AcpSessionOptions, &str),
    pub stream: fn(StreamInput, Box<dyn FnMut(StreamEvent) + Send>) -> StreamFuture,
    pub catalog_error: fn(agent_provider::provider::CatalogError) -> String,
    pub system_prompt: fn(&model_types::Model, &[String]) -> Vec<String>,
    pub unified_prompt_marker: &'static str,
}
static HOST: OnceLock<Host> = OnceLock::new();
pub fn install(host: Host) { let _ = HOST.set(host); }
pub fn get() -> &'static Host { HOST.get().expect("LLM runtime host must be installed by composition") }
pub async fn stream(input: StreamInput, callback: impl FnMut(StreamEvent) + Send + 'static) -> Result<(), Error> {
    (get().stream)(input, Box::new(callback)).await
}
