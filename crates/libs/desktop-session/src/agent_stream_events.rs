//! Events emitted by gateway chat and plan-run streams.
#[derive(Debug, Clone)]
pub enum StreamMessage {
    AgentCompactionStatus(u64, bool),
    AgentContextLoaded(u64, Vec<String>),
    AgentModelRecovery(u64, gateway_chat_stream::GatewayModelRecoveryEvent),
    AgentPostToolRound(u64, String, u32),
    AgentSessionDiffSummary(u64, String, session_info::Summary),
    AgentStepFinish(
        u64,
        String,
        u32,
        u64,
        session_ui_types::TokenUsage,
        Option<String>,
        Option<String>,
    ),
    AgentStepStart(u64, String, u32, u64, Option<String>),
    AgentStreamCancelled(u64),
    AgentStreamDelta(u64, String),
    AgentStreamDone(u64, session_ui_types::TokenUsage, Option<String>, Option<String>),
    AgentStreamError(u64, String),
    AgentTodoContinuation(u64),
    AgentToolLifecycle(u64, gateway_chat_stream::GatewayChatToolEvent),
    AgentWorkflowNodeUpdate(u64, String),
    PlanRunSessionCancelled(String),
    PlanRunSessionDelta {
        session_id: String,
        delta: String,
    },
    PlanRunSessionFinished(String),
    PlanRunSessionStarted {
        title_feedback: Option<desktop_i18n::input::Feedback>,
        parent_request_id: u64,
        parent_session_id: String,
        session_id: String,
        title: String,
        directory: String,
        query: String,
    },
    PlanRunSessionWaiting(String),
}

impl<M, P, V, D, T, G> From<StreamMessage>
    for crate::message::chat::events::ChatMessage<M, P, V, D, T, G>
{
    fn from(event: StreamMessage) -> Self {
        use crate::message::chat::events::ChatMessage;
        match event {
            StreamMessage::AgentCompactionStatus(v0, v1) => {
                ChatMessage::AgentCompactionStatus(v0, v1)
            }
            StreamMessage::AgentContextLoaded(v0, v1) => ChatMessage::AgentContextLoaded(v0, v1),
            StreamMessage::AgentModelRecovery(v0, v1) => ChatMessage::AgentModelRecovery(v0, v1),
            StreamMessage::AgentPostToolRound(v0, v1, v2) => {
                ChatMessage::AgentPostToolRound(v0, v1, v2)
            }
            StreamMessage::AgentSessionDiffSummary(v0, v1, v2) => {
                ChatMessage::AgentSessionDiffSummary(v0, v1, v2)
            }
            StreamMessage::AgentStepFinish(v0, v1, v2, v3, v4, v5, v6) => {
                ChatMessage::AgentStepFinish(v0, v1, v2, v3, v4, v5, v6)
            }
            StreamMessage::AgentStepStart(v0, v1, v2, v3, v4) => {
                ChatMessage::AgentStepStart(v0, v1, v2, v3, v4)
            }
            StreamMessage::AgentStreamCancelled(v0) => ChatMessage::AgentStreamCancelled(v0),
            StreamMessage::AgentStreamDelta(v0, v1) => ChatMessage::AgentStreamDelta(v0, v1),
            StreamMessage::AgentStreamDone(v0, v1, v2, v3) => {
                ChatMessage::AgentStreamDone(v0, v1, v2, v3)
            }
            StreamMessage::AgentStreamError(v0, v1) => ChatMessage::AgentStreamError(v0, v1),
            StreamMessage::AgentTodoContinuation(v0) => ChatMessage::AgentTodoContinuation(v0),
            StreamMessage::AgentToolLifecycle(v0, v1) => ChatMessage::AgentToolLifecycle(v0, v1),
            StreamMessage::AgentWorkflowNodeUpdate(v0, v1) => {
                ChatMessage::AgentWorkflowNodeUpdate(v0, v1)
            }
            StreamMessage::PlanRunSessionCancelled(v0) => ChatMessage::PlanRunSessionCancelled(v0),
            StreamMessage::PlanRunSessionDelta { session_id, delta } => {
                ChatMessage::PlanRunSessionDelta { session_id, delta }
            }
            StreamMessage::PlanRunSessionFinished(v0) => ChatMessage::PlanRunSessionFinished(v0),
            StreamMessage::PlanRunSessionStarted {
                title_feedback,
                parent_request_id,
                parent_session_id,
                session_id,
                title,
                directory,
                query,
            } => ChatMessage::PlanRunSessionStarted {
                title_feedback,
                parent_request_id,
                parent_session_id,
                session_id,
                title,
                directory,
                query,
            },
            StreamMessage::PlanRunSessionWaiting(v0) => ChatMessage::PlanRunSessionWaiting(v0),
        }
    }
}
