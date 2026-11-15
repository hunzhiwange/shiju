//! Events emitted by gateway chat and plan-run streams.
#[derive(Debug, Clone)]
pub enum StreamMessage {
    AgentCompactionStatus(u64, bool),
    AgentContextLoaded(u64, Vec<String>),
    AgentModelRecovery(u64, gateway_chat_stream::GatewayModelRecoveryEvent),
    AgentPostToolRound(u64, String, u32),
    AgentSessionDiffSummary(u64, String, session_info::Summary),
    AgentStepFinish(u64, String, u32, u64, session_ui_types::TokenUsage, Option<String>, Option<String>),
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
