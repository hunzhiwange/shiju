//! Transport scope adapter for session-engine's stream processor.
use crate::session::llm;
use crate::session::message::AssistantError;
use crate::tools::ToolSpec;
use serde_json::Value;
use session_engine::processor_events::StreamEvent;
pub(crate) use session_engine::llm_runner::{LlmStep, is_prompt_too_long_error, input_length_range_max, is_request_too_long_error};
#[cfg(test)]
use session_engine::llm_runner::{collect_llm_step_from_stream, emit_stream_event, is_acp_request, llm_step_retry_allowed, llm_error_message, is_acp_retryable_error, provider_retry_backoff_ms, STREAM_CONSUMER_STOPPED_ERR};
#[cfg(test)]
use super::types::{ToolCallStatus, ToolEventKind};
struct AgentLlmStreamHost;
impl session_engine::llm_runner::LlmStreamHost for AgentLlmStreamHost {
    fn spawn(session_id: String, messages: Vec<Value>, system: Vec<String>, model: Option<String>, options: Value, tools: std::collections::HashMap<String, ToolSpec>, abort_rx: tokio::sync::watch::Receiver<bool>, tx: std::sync::mpsc::Sender<llm::StreamEvent>) -> std::thread::JoinHandle<()> {
        spawn_llm_stream_thread(session_id, messages, system, model, options, tools, abort_rx, tx)
    }
}
#[cfg(test)]
fn spawn_llm_stream_thread(
    session_id: String,
    messages: Vec<serde_json::Value>,
    system: Vec<String>,
    model: Option<String>,
    options: serde_json::Value,
    tools: std::collections::HashMap<String, ToolSpec>,
    abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    if std::env::var_os("VW_RECOVERY_WINDOW_HTTP").is_some() {
        return spawn_real_llm_stream_thread(
            session_id, messages, system, model, options, tools, abort_rx, tx,
        );
    }
    llm_runner_tests::record_test_llm_messages(
        session_id.clone(),
        messages,
        options,
        system,
        model,
        tools.values().cloned().collect(),
    );
    llm_runner_tests::spawn_test_llm_stream_thread(&session_id, tools, abort_rx, tx)
}

#[cfg(not(test))]
use spawn_real_llm_stream_thread as spawn_llm_stream_thread;

fn spawn_real_llm_stream_thread(
    session_id: String,
    messages: Vec<serde_json::Value>,
    system: Vec<String>,
    model: Option<String>,
    options: serde_json::Value,
    tools: std::collections::HashMap<String, ToolSpec>,
    abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    let tool_context = crate::tools::current_tool_use_context();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project =
        crate::project::instance::project().map(|project| crate::project::instance::Context {
            directory: crate::project::instance::directory(),
            worktree: crate::project::instance::worktree(),
            project,
        });
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = tx.send(llm::StreamEvent::Error(AssistantError::Unknown {
                    message: error.to_string(),
                }));
                return;
            }
        };
        runtime.block_on(crate::session::ui_store::with_tenant_scope(tenant, async move {
            let usage_session = session_id.clone();
            let request = async move {
                llm::stream_chat_with_tools_for_session_with_abort(
                    &session_id,
                    messages,
                    system,
                    model,
                    options,
                    tools,
                    Some(abort_rx),
                    true,
                    move |event| {
                        let _ = tx.send(event);
                    },
                );
            };
            let request = crate::usage_bridge::native::with_session(usage_session, request);
            let request = async move {
                match tool_context {
                    Some(context) => crate::tools::scope_tool_use_context(context, request).await,
                    None => request.await,
                }
            };
            match project {
                Some(context) => crate::project::instance::provide_context(context, request).await,
                None => request.await,
            }
        }));
    })
}

pub(crate) fn run_llm_step(
    session_id: &str,
    messages: &[serde_json::Value],
    system: &[String],
    model: Option<String>,
    options: &serde_json::Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> { session_engine::llm_runner::run_llm_step::<AgentLlmStreamHost>(session_id, messages, system, model, options, available_tools, allowed_tools, on_event, cancellation) }
pub(crate) fn run_llm_step_with_retry(
    session_id: &str,
    messages: &[serde_json::Value],
    system: &[String],
    model: Option<String>,
    options: &serde_json::Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    additional_retries: u32,
    initial_backoff_ms: u64,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> { session_engine::llm_runner::run_llm_step_with_retry::<AgentLlmStreamHost>(session_id, messages, system, model, options, available_tools, allowed_tools, on_event, additional_retries, initial_backoff_ms, cancellation) }
pub(crate) fn run_llm_step_with_budget(
    session_id: &str,
    messages: &[Value],
    system: &[String],
    model: Option<String>,
    options: &Value,
    available_tools: &[ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    budget: &mut super::recovery::RetryBudget,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
) -> Result<LlmStep, String> { session_engine::llm_runner::run_llm_step_with_budget::<AgentLlmStreamHost>(session_id, messages, system, model, options, available_tools, allowed_tools, on_event, budget, cancellation) }
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/llm_runner_tests.rs"]
mod llm_runner_tests;
#[cfg(test)]
pub(crate) use llm_runner_tests::{
    ChannelCancelStream, enqueue_test_llm_stream, register_channel_cancel_stream,
    take_test_llm_messages, take_test_llm_stream_tool_ids, test_llm_stream_guard,
};
