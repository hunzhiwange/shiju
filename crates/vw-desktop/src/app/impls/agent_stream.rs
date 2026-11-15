//! Map session stream events into the desktop application message.
use super::{AgentRequest, Message, message};
use desktop_session::agent_stream_events::StreamMessage;
pub(crate) use desktop_session::agent_stream::{desktop_stream_request_id, task_plan_run_active};

pub(super) fn agent_stream(req: &AgentRequest) -> desktop_session::agent_stream::AgentBoxStream<Message> {
    use iced::futures::StreamExt;
    let stream = desktop_session::agent_stream::agent_stream(
        req, crate::app::config::config_access(), crate::app::config::gateway_client_endpoint(),
    ).map(|event| Message::Chat(match event {
        StreamMessage::AgentCompactionStatus(v0, v1) => message::ChatMessage::AgentCompactionStatus(v0, v1),
        StreamMessage::AgentContextLoaded(v0, v1) => message::ChatMessage::AgentContextLoaded(v0, v1),
        StreamMessage::AgentModelRecovery(v0, v1) => message::ChatMessage::AgentModelRecovery(v0, v1),
        StreamMessage::AgentPostToolRound(v0, v1, v2) => message::ChatMessage::AgentPostToolRound(v0, v1, v2),
        StreamMessage::AgentSessionDiffSummary(v0, v1, v2) => message::ChatMessage::AgentSessionDiffSummary(v0, v1, v2),
        StreamMessage::AgentStepFinish(v0, v1, v2, v3, v4, v5, v6) => message::ChatMessage::AgentStepFinish(v0, v1, v2, v3, v4, v5, v6),
        StreamMessage::AgentStepStart(v0, v1, v2, v3, v4) => message::ChatMessage::AgentStepStart(v0, v1, v2, v3, v4),
        StreamMessage::AgentStreamCancelled(v0) => message::ChatMessage::AgentStreamCancelled(v0),
        StreamMessage::AgentStreamDelta(v0, v1) => message::ChatMessage::AgentStreamDelta(v0, v1),
        StreamMessage::AgentStreamDone(v0, v1, v2, v3) => message::ChatMessage::AgentStreamDone(v0, v1, v2, v3),
        StreamMessage::AgentStreamError(v0, v1) => message::ChatMessage::AgentStreamError(v0, v1),
        StreamMessage::AgentTodoContinuation(v0) => message::ChatMessage::AgentTodoContinuation(v0),
        StreamMessage::AgentToolLifecycle(v0, v1) => message::ChatMessage::AgentToolLifecycle(v0, v1),
        StreamMessage::AgentWorkflowNodeUpdate(v0, v1) => message::ChatMessage::AgentWorkflowNodeUpdate(v0, v1),
        StreamMessage::PlanRunSessionCancelled(v0) => message::ChatMessage::PlanRunSessionCancelled(v0),
        StreamMessage::PlanRunSessionDelta{ session_id, delta } => message::ChatMessage::PlanRunSessionDelta{ session_id, delta },
        StreamMessage::PlanRunSessionFinished(v0) => message::ChatMessage::PlanRunSessionFinished(v0),
        StreamMessage::PlanRunSessionStarted{ title_feedback, parent_request_id, parent_session_id, session_id, title, directory, query } => message::ChatMessage::PlanRunSessionStarted{ title_feedback, parent_request_id, parent_session_id, session_id, title, directory, query },
        StreamMessage::PlanRunSessionWaiting(v0) => message::ChatMessage::PlanRunSessionWaiting(v0),
    }));
    #[cfg(not(target_arch = "wasm32"))]
    { stream.boxed() }
    #[cfg(target_arch = "wasm32")]
    { stream.boxed_local() }
}
