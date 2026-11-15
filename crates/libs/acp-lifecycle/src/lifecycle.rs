//! Agent 生命周期快照与会话状态对齐逻辑。

use acp_types::{SessionConversation, SessionMessage, SessionRecord, normalize_runtime_session_id};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentLifecycleExit {
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub exited_at: Option<String>,
    pub reason: Option<String>,
    pub unexpected_during_prompt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentLifecycleSnapshot {
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub last_exit: Option<AgentLifecycleExit>,
}

pub fn apply_lifecycle_snapshot_to_record(
    record: &mut SessionRecord,
    snapshot: &AgentLifecycleSnapshot,
) {
    record.pid = snapshot.pid;
    record.agent_started_at = snapshot.started_at.clone();

    if let Some(last_exit) = snapshot.last_exit.as_ref() {
        record.last_agent_exit_code = last_exit.exit_code;
        record.last_agent_exit_signal = last_exit.signal.clone();
        record.last_agent_exit_at = last_exit.exited_at.clone();
        record.last_agent_disconnect_reason = last_exit.reason.clone();
        return;
    }

    record.last_agent_exit_code = None;
    record.last_agent_exit_signal = None;
    record.last_agent_exit_at = None;
    record.last_agent_disconnect_reason = None;
}

pub fn reconcile_agent_session_id(record: &mut SessionRecord, agent_session_id: Option<&str>) {
    let Some(normalized) =
        agent_session_id.and_then(|value| normalize_runtime_session_id(&value.into()))
    else {
        return;
    };

    record.agent_session_id = Some(normalized);
}

pub fn session_has_agent_messages(record: &SessionRecord) -> bool {
    record.messages.iter().any(|message| matches!(message, SessionMessage::Agent(_)))
}

pub fn apply_conversation(record: &mut SessionRecord, conversation: &SessionConversation) {
    record.title = conversation.title.clone();
    record.messages = conversation.messages.clone();
    record.updated_at = conversation.updated_at.clone();
    record.cumulative_token_usage = conversation.cumulative_token_usage.clone();
    record.request_token_usage = conversation.request_token_usage.clone();
}
