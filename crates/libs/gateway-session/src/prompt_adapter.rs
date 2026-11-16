//! Typed desktop prompt request preparation and synchronous stream observation.
use crate::stream_host::PromptInput;
use serde_json::Value;
use std::sync::Arc;
use session_engine::processor_events::StreamEvent;

pub fn prepare_request<A, S: Default>(
    approval: Arc<A>, input: PromptInput,
) -> (session_engine::runner_types::Request<A, S>, Option<Vec<Value>>, tokio::sync::mpsc::UnboundedReceiver<tool_context::approval::NonCliApprovalPrompt>) {
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let request = session_engine::runner_types::Request {
        stream: 0,
        session: input.session,
        query: input.query,
        root: Some(input.root),
        model: input.model,
        reliability: input.reliability,
        options: input.options,
        approval: Some(approval),
        channel_name: Some("desktop".to_string()),
        non_cli_approval_context: (!input.background_acp).then(|| {
            tool_context::approval::NonCliApprovalContext {
                sender: "desktop".to_string(),
                reply_target: input.approval_reply_target,
                prompt_tx,
            }
        }),
        assistant_message_id: input.assistant_message_id,
        history: input.history,
        persist_app_session_artifacts: input.persist_app_session_artifacts,
        app_session_scope: input.app_session_scope,
        abort_token: input.abort_token,
        session_tools: tool_assembly::runtime::SessionToolset::default(),
        session_skills: Default::default(),
    };
    (request, input.persisted_history, prompt_rx)
}

/// Observe before forwarding, so snapshot recording finishes before processor progress.
pub fn observe_event(event: &StreamEvent, before_emit: &mut impl FnMut(&session_port::PromptEvent)) {
    match event {
        StreamEvent::StepStart {
            step_index,
            created_ms,
            model,
        } => before_emit(&session_port::PromptEvent::StepStart {
            step_index: *step_index,
            created_ms: *created_ms,
            model: model.clone(),
        }),
        StreamEvent::PostToolRound {
            step_index,
        } => before_emit(&session_port::PromptEvent::PostToolRound {
            step_index: *step_index,
        }),
        StreamEvent::StepFinish {
            step_index,
            finished_ms,
            usage,
            finish_reason,
            model,
        } => before_emit(&session_port::PromptEvent::StepFinish {
            step_index: *step_index,
            finished_ms: *finished_ms,
            usage: usage.clone(),
            finish_reason: finish_reason.clone(),
            model: model.clone(),
        }),
        _ => {}
    }
}
