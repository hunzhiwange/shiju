//! Prompt entrypoint composition; algorithms are owned by llm-runtime.
pub(crate) fn ensure_host() {
    crate::provider::provider::ensure_host();
    llm_runtime::host::install(llm_runtime::host::Host {
        #[cfg(not(target_arch = "wasm32"))]
        annotate_model_source: super::acp::controls::annotate_model_source,
        stream: |input, callback| Box::pin(super::stream(input, callback)),
        catalog_error: super::types::catalog_error_string,
        system_prompt: |model, tools| crate::session::system::provider_with_context(model, None, tools),
        unified_prompt_marker: crate::session::system::UNIFIED_PROMPT_MARKER,
    });
}
use crate::provider::provider;
use crate::tools;
use crate::session::ui_types as models;
use super::types::{PromptStreamEvent, StreamEvent};
use serde_json::Value;
use std::collections::HashMap;
#[cfg(test)]
use llm_runtime::prompt::*;
pub(super) async fn resolve_model(model: Option<String>) -> Result<provider::Model, String> {
    ensure_host();
    llm_runtime::prompt::resolve_model(model).await
}
pub(crate) fn acp_model_source(
    options: &Value,
) -> Result<vw_api_acp_model::acp_model_source::AcpModelSource, String> {
    ensure_host();
    llm_runtime::prompt::acp_model_source(options)
}
pub(crate) fn is_acp_model(model: &provider::Model) -> bool {
    ensure_host();
    llm_runtime::prompt::is_acp_model(model)
}
pub(crate) async fn resolve_auto_model(
    message: &str,
    options: &Value,
    needs_tools: bool,
) -> Result<provider::Model, String> {
    ensure_host();
    llm_runtime::prompt::resolve_auto_model(message, options, needs_tools).await
}
pub(crate) fn model_limit_for_request(
    model: Option<String>,
    options: &Value,
) -> provider::ModelLimit {
    ensure_host();
    llm_runtime::prompt::model_limit_for_request(model, options)
}
pub fn stream_prompt(
    prompt: &str,
    model: Option<String>,
    on_event: impl FnMut(PromptStreamEvent) + Send + 'static,
) {
    ensure_host();
    llm_runtime::prompt::stream_prompt(prompt, model, on_event)
}
pub fn stream_prompt_with_tools(
    prompt: &str,
    model: Option<String>,
    tools: HashMap<String, tools::ToolSpec>,
    on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    ensure_host();
    llm_runtime::prompt::stream_prompt_with_tools(prompt, model, tools, on_event)
}
pub fn stream_chat_with_tools(
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    ensure_host();
    llm_runtime::prompt::stream_chat_with_tools(messages, system, model, options, tools, on_event)
}
pub fn stream_chat_with_tools_for_session(
    session_id: &str,
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    ensure_host();
    llm_runtime::prompt::stream_chat_with_tools_for_session(session_id, messages, system, model, options, tools, on_event)
}
pub(crate) fn stream_chat_with_tools_for_session_with_abort(
    session_id: &str,
    messages: Vec<Value>,
    system: Vec<String>,
    model: Option<String>,
    options: Value,
    tools: HashMap<String, tools::ToolSpec>,
    abort: Option<tokio::sync::watch::Receiver<bool>>,
    retry_in_caller: bool,
    on_event: impl FnMut(StreamEvent) + Send + 'static,
) {
    ensure_host();
    llm_runtime::prompt::stream_chat_with_tools_for_session_with_abort(session_id, messages, system, model, options, tools, abort, retry_in_caller, on_event)
}
pub fn send_prompt(
    prompt: &str,
    model: Option<String>,
) -> Result<(models::TokenUsage, String), String> {
    ensure_host();
    llm_runtime::prompt::send_prompt(prompt, model)
}
#[cfg(test)]
#[path = "../../../../libs/llm-runtime/src/prompt_tests.rs"]
mod prompt_tests;

#[cfg(test)]
async fn resolve_model_for_options(model: Option<String>, options: &Value) -> Result<provider::Model, String> {
    ensure_host();
    llm_runtime::prompt::resolve_model_for_options(model, options).await
}
