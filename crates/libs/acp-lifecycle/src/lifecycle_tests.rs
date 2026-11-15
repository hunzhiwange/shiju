//! 生命周期快照与状态对齐逻辑的单元测试。

use std::collections::HashMap;

use serde_json::Value;

use crate::{
    AgentLifecycleExit, AgentLifecycleSnapshot, apply_conversation,
    apply_lifecycle_snapshot_to_record, reconcile_agent_session_id, session_has_agent_messages,
};
use acp_types::{
    SessionAgentContent, SessionAgentMessage, SessionConversation, SessionEventLog, SessionMessage,
    SessionRecord, SessionTokenUsage, SessionUserContent, SessionUserMessage,
};

fn test_record() -> SessionRecord {
    SessionRecord {
        schema: "vwacp.session.v1".to_string(),
        vwacp_record_id: "record-1".to_string(),
        acp_session_id: "session-1".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/project/active.jsonl".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 3,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

fn test_conversation() -> SessionConversation {
    let mut request_token_usage = HashMap::new();
    request_token_usage.insert(
        "user-1".to_string(),
        SessionTokenUsage {
            input_tokens: Some(10),
            output_tokens: Some(20),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        },
    );

    SessionConversation {
        title: Some("new title".to_string()),
        messages: vec![
            SessionMessage::User(SessionUserMessage {
                id: "user-1".to_string(),
                content: vec![SessionUserContent::Text("hello".to_string())],
            }),
            SessionMessage::Agent(SessionAgentMessage {
                content: vec![SessionAgentContent::Text("world".to_string())],
                tool_results: HashMap::new(),
                reasoning_details: Some(Value::String("details".to_string())),
            }),
        ],
        updated_at: "2026-01-02T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage {
            input_tokens: Some(30),
            output_tokens: Some(40),
            cache_creation_input_tokens: Some(5),
            cache_read_input_tokens: Some(6),
        },
        request_token_usage,
    }
}

#[test]
fn apply_lifecycle_snapshot_to_record_sets_exit_fields() {
    let mut record = test_record();
    let snapshot = AgentLifecycleSnapshot {
        pid: Some(42),
        started_at: Some("2026-01-01T12:00:00Z".to_string()),
        last_exit: Some(AgentLifecycleExit {
            exit_code: Some(1),
            signal: Some("SIGTERM".to_string()),
            exited_at: Some("2026-01-01T12:30:00Z".to_string()),
            reason: Some("agent_disconnect".to_string()),
            unexpected_during_prompt: true,
        }),
    };

    apply_lifecycle_snapshot_to_record(&mut record, &snapshot);

    assert_eq!(record.pid, Some(42));
    assert_eq!(record.agent_started_at.as_deref(), Some("2026-01-01T12:00:00Z"));
    assert_eq!(record.last_agent_exit_code, Some(1));
    assert_eq!(record.last_agent_exit_signal.as_deref(), Some("SIGTERM"));
    assert_eq!(record.last_agent_exit_at.as_deref(), Some("2026-01-01T12:30:00Z"));
    assert_eq!(record.last_agent_disconnect_reason.as_deref(), Some("agent_disconnect"));
}

#[test]
fn apply_lifecycle_snapshot_to_record_clears_exit_fields_without_last_exit() {
    let mut record = test_record();
    record.last_agent_exit_code = Some(9);
    record.last_agent_exit_signal = Some("SIGKILL".to_string());
    record.last_agent_exit_at = Some("2026-01-01T10:00:00Z".to_string());
    record.last_agent_disconnect_reason = Some("previous".to_string());

    let snapshot = AgentLifecycleSnapshot {
        pid: Some(7),
        started_at: Some("2026-01-01T11:00:00Z".to_string()),
        last_exit: None,
    };

    apply_lifecycle_snapshot_to_record(&mut record, &snapshot);

    assert_eq!(record.pid, Some(7));
    assert_eq!(record.agent_started_at.as_deref(), Some("2026-01-01T11:00:00Z"));
    assert_eq!(record.last_agent_exit_code, None);
    assert_eq!(record.last_agent_exit_signal, None);
    assert_eq!(record.last_agent_exit_at, None);
    assert_eq!(record.last_agent_disconnect_reason, None);
}

#[test]
fn reconcile_agent_session_id_ignores_blank_values() {
    let mut record = test_record();

    reconcile_agent_session_id(&mut record, Some("   "));

    assert_eq!(record.agent_session_id, None);
}

#[test]
fn reconcile_agent_session_id_updates_normalized_value() {
    let mut record = test_record();

    reconcile_agent_session_id(&mut record, Some(" runtime-session "));

    assert_eq!(record.agent_session_id.as_deref(), Some("runtime-session"));
}

#[test]
fn session_has_agent_messages_detects_agent_entries() {
    let mut record = test_record();
    record.messages.push(SessionMessage::User(SessionUserMessage {
        id: "user-1".to_string(),
        content: vec![SessionUserContent::Text("hello".to_string())],
    }));
    assert!(!session_has_agent_messages(&record));

    record.messages.push(SessionMessage::Agent(SessionAgentMessage {
        content: vec![SessionAgentContent::Text("world".to_string())],
        tool_results: HashMap::new(),
        reasoning_details: None,
    }));

    assert!(session_has_agent_messages(&record));
}

#[test]
fn apply_conversation_replaces_record_conversation_fields() {
    let mut record = test_record();
    let conversation = test_conversation();

    apply_conversation(&mut record, &conversation);

    assert_eq!(record.title, conversation.title);
    assert_eq!(record.messages, conversation.messages);
    assert_eq!(record.updated_at, conversation.updated_at);
    assert_eq!(record.cumulative_token_usage, conversation.cumulative_token_usage);
    assert_eq!(record.request_token_usage, conversation.request_token_usage);
}
