//! 原生提示事件、审批及 ACP 终态的桥接。
use std::sync::Arc;
use acp_server::server::{BackendCancelHandle, BackendError, BackendEvent, BackendPrompt};
use agent_approval::ApprovalManager;
use session_port::{PromptCancelHandle, PromptEvent, PromptTermination, PromptTurn, SessionScope};
use session_port::ui_types::TokenUsage;
use crate::{approval::ApprovalBridge, backend::map_runtime_event};

/// Persist the exact completed native turn inside its tenant/project context.
#[async_trait::async_trait]
pub trait TurnPersistence: Send + Sync + 'static {
    async fn persist_turn(&self, scope: SessionScope, session_id: String, query: String, assistant_text: String, usage: TokenUsage, error: Option<String>, cancelled: bool) -> Result<(), BackendError>;
}

struct RuntimeCancel(PromptCancelHandle);

impl BackendCancelHandle for RuntimeCancel {
    fn cancel(&self) -> bool {
        self.0.cancel()
    }
}

pub fn bridge_prompt<P: TurnPersistence>(
    persistence: P,
    cancellation: tokio_util::sync::CancellationToken,
    runtime: PromptTurn,
    operation: tokio::sync::OwnedMutexGuard<()>,
    runtime_handle: tokio::runtime::Handle,
    scope: SessionScope,
    session_id: String,
    query: String,
    approval: Arc<ApprovalManager>,
    approval_prompts: tokio::sync::mpsc::UnboundedReceiver<
        tool_context::approval::NonCliApprovalPrompt,
    >,
) -> BackendPrompt {
    let (events_tx, events_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permissions_tx, permissions_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = runtime.cancel.clone();
    runtime_handle.spawn(
        ApprovalBridge::new(
            approval,
            approval_prompts,
            permissions_tx,
            cancellation,
        )
        .run(),
    );
    std::thread::spawn(move || {
        let _operation = operation;
        let mut assistant_text = String::new();
        while let Ok(event) = runtime.events.recv() {
            if let PromptEvent::Text(text) = &event {
                assistant_text.push_str(text);
            }
            match &event {
                PromptEvent::Terminated(PromptTermination::Completed(usage)) => {
                    if let Err(error) = runtime_handle.block_on(persistence.persist_turn(
                        scope.clone(),
                        session_id.clone(),
                        query.clone(),
                        assistant_text.clone(),
                        usage.clone(),
                        None,
                        false,
                    )) {
                        let _ = events_tx.send(BackendEvent::Error(error.to_string()));
                        break;
                    }
                }
                PromptEvent::Terminated(PromptTermination::Failed(error)) => {
                    let stored_text = if assistant_text.trim().is_empty() {
                        error.clone()
                    } else {
                        assistant_text.clone()
                    };
                    if let Err(persist_error) = runtime_handle.block_on(persistence.persist_turn(
                        scope.clone(),
                        session_id.clone(),
                        query.clone(),
                        stored_text,
                        TokenUsage::default(),
                        Some(error.clone()),
                        false,
                    )) {
                        let _ = events_tx.send(BackendEvent::Error(persist_error.to_string()));
                        break;
                    }
                }
                PromptEvent::Terminated(PromptTermination::Cancelled) => {
                    if let Err(error) = runtime_handle.block_on(persistence.persist_turn(
                        scope.clone(),
                        session_id.clone(),
                        query.clone(),
                        assistant_text.clone(),
                        TokenUsage::default(),
                        None,
                        true,
                    )) {
                        let _ = events_tx.send(BackendEvent::Error(error.to_string()));
                        break;
                    }
                }
                _ => {}
            }
            if let Some(event) = map_runtime_event(event) {
                let terminal = matches!(event, BackendEvent::Terminal(_) | BackendEvent::Error(_));
                if events_tx.send(event).is_err() || terminal {
                    break;
                }
            }
        }
    });
    BackendPrompt::new(events_rx, std::sync::Arc::new(RuntimeCancel(cancel)))
        .with_permissions(permissions_rx)
}
