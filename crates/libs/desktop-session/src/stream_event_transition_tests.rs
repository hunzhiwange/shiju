//! Stream event state transitions; host tests retain cross-domain effect wiring.
use crate::message::chat::request::{AcpHistoryReplayMode, AgentRequest};
use crate::message::chat::stream::stream_events::*;
use crate::message::chat::stream::tail::StreamTailState;
use crate::session::agent::SessionRuntimeState;
use session_ui_types::{ChatMessage, ChatRole};
use std::sync::Arc;
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
fn agent_request(id: u64, session_id: &str) -> Arc<AgentRequest> {
    Arc::new(AgentRequest {
        id,
        session: session_id.to_string(),
        query: "query".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_model_source: Default::default(),
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: Some(false),
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
}

#[test]
// 思考标签可能跨多个增量拆分，尾部状态机需要在不重扫完整消息的情况下拼接。
fn stream_tail_state_handles_split_think_tags_from_deltas() {
    let cases = [
        vec!["plain response"],
        vec!["<thi", "nk>working", "</thi", "nk>done"],
        vec!["<thinking", ">first</think", "ing><think>", "second</think>", "tail"],
        vec!["<think>reasoning", "\ntool read\n{\"status\":\"completed\"}\n", "</think>answer"],
    ];

    for (deltas, (expected_count, expected_open)) in
        cases.into_iter().zip([(0, false), (1, false), (2, false), (1, false)])
    {
        let mut state = StreamTailState::default();
        let mut full_message = String::new();
        for delta in deltas {
            state.apply_delta(delta);
            full_message.push_str(delta);
        }

        assert_eq!(state.think_count(), expected_count, "message: {full_message:?}");
        assert_eq!(state.thinking_open(), expected_open, "message: {full_message:?}");
    }
}

#[test]
fn compaction_status_started_then_completed_updates_one_message() {
    let mut runtime = SessionRuntimeState::<()>::new();
    let mut chat = Vec::new();
    let mut ids = Vec::new();
    apply_runtime_compaction(&mut runtime, &mut chat, &mut ids, false, "started", "completed");
    assert_eq!(chat.len(), 1);
    assert_eq!(runtime.pending_compaction_status_index, Some(0));
    apply_runtime_compaction(&mut runtime, &mut chat, &mut ids, true, "started", "completed");
    assert_eq!(chat.len(), 1);
    assert_eq!(runtime.pending_compaction_status_index, None);
    assert_eq!(ids, vec![None]);
}

fn apply_delta(
    runtime: &mut SessionRuntimeState<()>,
    chat: &mut Vec<ChatMessage>,
    ids: &mut Vec<Option<String>>,
    delta: &str,
) -> DeltaEffect {
    let effect = queue_runtime_delta(runtime, true, delta.into());
    if let DeltaEffect::Immediate(ref delta) = effect {
        append_stream_delta_to_tail(chat, ids, delta.clone(), 10, &mut runtime.stream_tail_state);
    }
    effect
}

#[test]
fn active_stream_followup_deltas_are_coalesced_until_flush() {
    let mut runtime = SessionRuntimeState::<()>::new();
    let mut chat = vec![message(ChatRole::User, "question")];
    let mut ids = vec![None];
    assert!(matches!(
        apply_delta(&mut runtime, &mut chat, &mut ids, "hello"),
        DeltaEffect::Immediate(_)
    ));
    assert!(matches!(
        apply_delta(&mut runtime, &mut chat, &mut ids, " world"),
        DeltaEffect::Schedule
    ));
    assert!(matches!(apply_delta(&mut runtime, &mut chat, &mut ids, "!"), DeltaEffect::Buffered));
    assert_eq!(chat.len(), 2);
    assert_eq!(chat.last().unwrap().content, "hello");
    assert_eq!(runtime.pending_stream_delta, " world!");
    let delta = flush_runtime_delta(&mut runtime);
    append_stream_delta_to_tail(&mut chat, &mut ids, delta, 11, &mut runtime.stream_tail_state);
    assert_eq!(chat.last().unwrap().content, "hello world!");
    assert!(runtime.pending_stream_delta.is_empty());
    assert!(!runtime.stream_delta_flush_scheduled);
}

#[test]
fn active_stream_applies_first_delta_without_flush_timer() {
    let mut runtime = SessionRuntimeState::<()>::new();
    let mut chat = vec![message(ChatRole::User, "question")];
    let mut ids = vec![None];
    apply_delta(&mut runtime, &mut chat, &mut ids, "hello");
    assert_eq!(chat.last().unwrap().content, "hello");
    assert!(runtime.pending_stream_delta.is_empty());
    assert!(!runtime.stream_delta_flush_scheduled);
    apply_delta(&mut runtime, &mut chat, &mut ids, " world");
    assert_eq!(chat.last().unwrap().content, "hello");
    assert_eq!(runtime.pending_stream_delta, " world");
    assert!(runtime.stream_delta_flush_scheduled);
}

#[test]
fn active_workflow_update_appends_tool_block_and_preserves_message_id() {
    let mut chat = vec![message(ChatRole::Assistant, "answer")];
    let mut ids = vec![Some("assistant-id".into())];
    apply_tool_blocks(
        &mut chat,
        &mut ids,
        &mut StreamTailState::default(),
        vec!["tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n".into()],
    );
    assert_eq!(chat.len(), 1);
    assert_eq!(ids, vec![Some("assistant-id".into())]);
}

#[test]
fn stream_done_clears_active_request_and_applies_ids_and_usage() {
    let mut runtime = SessionRuntimeState::<()>::new();
    runtime.request_lifecycle.is_requesting = true;
    runtime.request_lifecycle.active_agent_request = Some(agent_request(42, "session-1"));
    let chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    let mut ids = vec![None, None];
    apply_stream_done_message_ids(
        &chat,
        &mut ids,
        Some("assistant-id".into()),
        Some("parent-id".into()),
    );
    finish_runtime(&mut runtime, false);
    assert_eq!(ids[0].as_deref(), Some("parent-id"));
    assert_eq!(ids[1].as_deref(), Some("assistant-id"));
    assert!(!runtime.request_lifecycle.is_requesting);
    assert!(!runtime.has_unseen_success);
    assert!(runtime.request_lifecycle.active_agent_request.is_none());
}

#[test]
fn missing_request_stream_error_clears_active_requesting_and_shows_toast() {
    let mut runtime = SessionRuntimeState::<()>::new();
    runtime.request_lifecycle.is_requesting = true;
    runtime.has_unseen_success = true;
    assert!(settle_unmatched_error(&mut runtime));
    assert!(!runtime.request_lifecycle.is_requesting);
    assert!(!runtime.has_unseen_success);
    assert!(runtime.request_lifecycle.active_agent_request.is_none());
}

#[test]
fn recovery_notice_cancel_terminal_and_old_error_keep_new_request() {
    let mut runtime = SessionRuntimeState::<()>::new();
    runtime.request_lifecycle.active_agent_request = Some(agent_request(43, "notice"));
    runtime.request_lifecycle.is_requesting = true;
    assert!(!settle_unmatched_error(&mut runtime));
    assert!(runtime.request_lifecycle.is_requesting);
    assert_eq!(runtime.request_lifecycle.active_agent_request.as_ref().unwrap().id, 43);
}

#[test]
fn recovery_notice_waiting_deduplicates_and_keeps_request_busy() {
    use gateway_chat_stream::{GatewayModelRecoveryEvent, GatewayModelRecoveryPhase::Waiting};
    let mut runtime = SessionRuntimeState::<()>::new();
    runtime.request_lifecycle.active_agent_request = Some(agent_request(42, "notice"));
    runtime.request_lifecycle.is_requesting = true;
    let event = GatewayModelRecoveryEvent {
        session_id: "notice".into(),
        attempt: 1,
        max_attempts: 3,
        wait_ms: 500,
        phase: Waiting,
        reason: "配置暂时不可读".into(),
        next_action: "检查模型配置".into(),
    };
    assert!(model_recovery_matches(&runtime, 42, "notice", &event));
    runtime.model_recovery = Some((42, event.clone()));
    assert!(!model_recovery_matches(&runtime, 42, "notice", &event));
    assert!(runtime.request_lifecycle.is_requesting);
}
