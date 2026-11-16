//! Native request inspection and real runner cancellation boundary.
use super::backend::{NativeAcpHost, VibeWindowBackend, start_prompt_with_runtime_runner};
use crate::session::processor::StreamEvent;
use crate::session::ui_types::TokenUsage;
use acp_server::server::{BackendPrompt, BackendError, PromptContent};
use agent_acp::host_migration_53_backend_contracts::{self as contracts, PromptCase};
struct NativeBackendPorts;
#[async_trait::async_trait]
impl contracts::BackendPorts for NativeBackendPorts {
    type Host = NativeAcpHost;
    async fn start(backend: &VibeWindowBackend, id: &str, case: PromptCase) -> Result<BackendPrompt, BackendError> {
        let query = match case { PromptCase::First | PromptCase::Native => "hello", PromptCase::Followup => "follow-up", PromptCase::Cancel => "wait" };
        start_prompt_with_runtime_runner(backend, id, vec![PromptContent::Text(query.into())], move |request, mut emit| {
            match case {
                PromptCase::First => {
                    assert_eq!(
                        request.reliability.provider_retries,
                        crate::app::agent::config::ReliabilityConfig::default().provider_retries
                    );
                    emit(StreamEvent::Delta("answer".to_string()));
                    emit(StreamEvent::Done(TokenUsage::default()));
                }
                PromptCase::Followup => {
                    assert_eq!(
                        request.history.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
                        ["hello", "answer"]
                    );
                    assert_eq!(request.history[0].role, crate::session::ui_types::ChatRole::User);
                    assert_eq!(request.history[1].role, crate::session::ui_types::ChatRole::Assistant);
                    emit(StreamEvent::Delta("second answer".into()));
                    emit(StreamEvent::Done(TokenUsage::default()));
                }
                PromptCase::Native => {
                    assert_eq!(request.channel_name.as_deref(), Some("acp"));
                    assert_eq!(request.model.as_deref(), Some("auto"));
                    assert!(crate::session::processor::should_execute_structured_tool_calls_locally(
                        &request.options
                    ));
                    emit(StreamEvent::Done(TokenUsage::default()));
                }
                PromptCase::Cancel => {
                    let abort = request.abort_token.expect("abort token");
                    while !abort.is_cancelled() {
                        std::thread::yield_now();
                    }
                    emit(StreamEvent::Delta("ignored".to_string()));
                }
            }
        }).await
    }
}

#[tokio::test]
async fn backend_reuses_native_session_identity_and_history() {
    contracts::backend_reuses_native_session_identity_and_history::<NativeBackendPorts>().await;
}

#[tokio::test]
async fn backend_prompt_persists_turn_once() {
    contracts::backend_prompt_persists_turn_once::<NativeBackendPorts>().await;
}

#[tokio::test]
async fn backend_prompt_uses_native_model_and_local_tools() {
    contracts::backend_prompt_uses_native_model_and_local_tools::<NativeBackendPorts>().await;
}

#[tokio::test]
async fn backend_cancel_propagates_to_runtime_turn() {
    contracts::backend_cancel_propagates_to_runtime_turn::<NativeBackendPorts>().await;
}
