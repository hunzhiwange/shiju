use session_ui_types::{ChatMessage, ChatRole, ChatSession, ChatSessionStep, TokenUsage};
use gateway_chat_stream::{GatewayChatToolEvent, GatewayChatToolEventKind, GatewayChatToolStatus};
fn empty_session() -> ChatSession {
    ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 1,
    }
}
fn usage(input_tokens: i64, output_tokens: i64) -> TokenUsage {
    TokenUsage { input_tokens, output_tokens, cached_tokens: 3, reasoning_tokens: 4 }
}
fn tool_event(kind: GatewayChatToolEventKind, occurred_ms: u64) -> GatewayChatToolEvent {
    GatewayChatToolEvent {
        step_index: 3,
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        kind,
        status: GatewayChatToolStatus::Running,
        occurred_ms,
        payload: serde_json::json!({}),
    }
}
#[test]
fn tool_lifecycle_merges_idempotently() {
    let mut session = empty_session();
    super::upsert_step_start(&mut session, 3, 80, None, None);

    super::merge_tool_lifecycle(&mut session, &tool_event(GatewayChatToolEventKind::Update, 90));
    super::merge_tool_lifecycle(&mut session, &tool_event(GatewayChatToolEventKind::Complete, 160));
    super::merge_tool_lifecycle(&mut session, &tool_event(GatewayChatToolEventKind::Start, 100));
    super::merge_tool_lifecycle(&mut session, &tool_event(GatewayChatToolEventKind::Start, 110));
    super::merge_tool_lifecycle(&mut session, &tool_event(GatewayChatToolEventKind::Complete, 170));

    let timings = &session.steps[0].tool_timings;
    assert_eq!(timings.len(), 1);
    assert_eq!(timings[0].call_id, "call-1");
    assert_eq!(timings[0].name, "shell");
    assert_eq!(timings[0].started_ms, Some(100));
    assert_eq!(timings[0].finished_ms, Some(160));
    assert_eq!(session.updated_ms, 170);
}
#[test]
fn upsert_step_start_inserts_steps_sorted_and_updates_session_time() {
    let mut session = empty_session();

    super::upsert_step_start(&mut session, 2, 20, Some("model-b".to_string()), None);
    super::upsert_step_start(
        &mut session,
        1,
        10,
        Some("model-a".to_string()),
        Some("start.snap".to_string()),
    );

    assert_eq!(session.steps.iter().map(|step| step.index).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(session.steps[0].started_ms, 10);
    assert_eq!(session.steps[0].start_snapshot_path.as_deref(), Some("start.snap"));
    assert_eq!(session.steps[0].model.as_deref(), Some("model-a"));
    assert_eq!(session.updated_ms, 20);
}
#[test]
fn upsert_step_start_snapshots_model_onto_step_messages() {
    let mut session = empty_session();
    session.messages = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    super::upsert_step_start(&mut session, 1, 10, Some("provider/model-a".to_string()), None);

    assert_eq!(session.messages[0].model.as_deref(), Some("provider/model-a"));
    assert_eq!(session.messages[1].model.as_deref(), Some("provider/model-a"));
}
#[test]
fn upsert_step_start_preserves_existing_optional_fields_when_none() {
    let mut session = empty_session();
    session.steps.push(ChatSessionStep {
        index: 7,
        turn_id: None,
        started_ms: 1,
        finished_ms: None,
        start_snapshot_path: Some("old.snap".to_string()),
        finish_snapshot_path: None,
        usage: TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some("old-model".to_string()),
        tool_timings: Vec::new(),
    });

    super::upsert_step_start(&mut session, 7, 30, None, None);

    let step = &session.steps[0];
    assert_eq!(step.started_ms, 30);
    assert_eq!(step.start_snapshot_path.as_deref(), Some("old.snap"));
    assert_eq!(step.model.as_deref(), Some("old-model"));
}
#[test]
fn upsert_step_start_replaces_model_when_some() {
    let mut session = empty_session();

    super::upsert_step_start(&mut session, 1, 10, Some("original".to_string()), None);
    super::upsert_step_start(&mut session, 1, 20, Some("replaced".to_string()), None);

    assert_eq!(session.steps.len(), 1);
    assert_eq!(session.steps[0].model.as_deref(), Some("replaced"));
    assert_eq!(session.updated_ms, 20);
}
#[test]
fn upsert_step_start_replaces_snapshot_path_when_some() {
    let mut session = empty_session();

    super::upsert_step_start(&mut session, 1, 10, None, Some("old.snap".to_string()));
    super::upsert_step_start(&mut session, 1, 20, None, Some("new.snap".to_string()));

    assert_eq!(session.steps.len(), 1);
    assert_eq!(session.steps[0].start_snapshot_path.as_deref(), Some("new.snap"));
}
#[test]
fn upsert_step_start_replaces_model_only_keeps_snapshot_path() {
    let mut session = empty_session();

    super::upsert_step_start(
        &mut session,
        1,
        10,
        Some("original".to_string()),
        Some("snap".to_string()),
    );
    // 后续事件只更新模型时，不应清除先前记录的快照路径。
    super::upsert_step_start(&mut session, 1, 20, Some("updated".to_string()), None);

    assert_eq!(session.steps[0].model.as_deref(), Some("updated"));
    assert_eq!(session.steps[0].start_snapshot_path.as_deref(), Some("snap"));
    assert_eq!(session.steps[0].started_ms, 20);
}
#[test]
fn upsert_step_finish_updates_existing_step_with_usage_and_reason() {
    let mut session = empty_session();
    super::upsert_step_start(&mut session, 1, 10, Some("draft-model".to_string()), None);

    super::upsert_step_finish(
        &mut session,
        1,
        40,
        usage(11, 22),
        Some("stop".to_string()),
        Some("final-model".to_string()),
    );

    let step = &session.steps[0];
    assert_eq!(step.finished_ms, Some(40));
    assert_eq!(step.usage.input_tokens, 11);
    assert_eq!(step.usage.output_tokens, 22);
    assert_eq!(step.finish_reason.as_deref(), Some("stop"));
    assert_eq!(step.model.as_deref(), Some("final-model"));
    assert_eq!(session.updated_ms, 40);
}
#[test]
fn upsert_step_finish_creates_missing_step_from_finish_time() {
    let mut session = empty_session();

    super::upsert_step_finish(&mut session, 5, 50, usage(1, 2), None, None);

    assert_eq!(session.steps.len(), 1);
    assert_eq!(session.steps[0].index, 5);
    assert_eq!(session.steps[0].started_ms, 50);
    assert_eq!(session.steps[0].finished_ms, Some(50));
}
#[test]
fn upsert_step_finish_preserves_model_when_none() {
    let mut session = empty_session();
    super::upsert_step_start(&mut session, 1, 10, Some("original-model".to_string()), None);

    super::upsert_step_finish(&mut session, 1, 40, usage(1, 2), Some("stop".to_string()), None);

    assert_eq!(session.steps[0].model.as_deref(), Some("original-model"));
    assert_eq!(session.steps[0].finish_reason.as_deref(), Some("stop"));
}
#[test]
fn upsert_step_finish_preserves_existing_reason_and_monotonic_session_time_when_none() {
    let mut session = empty_session();
    session.updated_ms = 100;
    super::upsert_step_finish(
        &mut session,
        1,
        80,
        usage(1, 2),
        Some("length".to_string()),
        Some("model-a".to_string()),
    );

    super::upsert_step_finish(&mut session, 1, 90, usage(3, 4), None, None);

    assert_eq!(session.steps[0].finish_reason.as_deref(), Some("length"));
    assert_eq!(session.steps[0].model.as_deref(), Some("model-a"));
    assert_eq!(session.updated_ms, 100);
}
#[test]
fn upsert_step_finish_updates_model_only_on_existing_step() {
    let mut session = empty_session();
    super::upsert_step_start(&mut session, 1, 10, None, None);

    super::upsert_step_finish(
        &mut session,
        1,
        40,
        usage(1, 2),
        None,
        Some("new-model".to_string()),
    );

    assert_eq!(session.steps[0].model.as_deref(), Some("new-model"));
    assert_eq!(session.steps[0].finish_reason, None);
}
#[test]
fn upsert_step_finish_maintains_sort_order_across_multiple_steps() {
    let mut session = empty_session();
    super::upsert_step_start(&mut session, 3, 30, None, None);
    super::upsert_step_start(&mut session, 1, 10, None, None);
    super::upsert_step_start(&mut session, 2, 20, None, None);

    super::upsert_step_finish(&mut session, 2, 200, usage(1, 2), None, None);
    super::upsert_step_finish(&mut session, 1, 100, usage(3, 4), Some("stop".to_string()), None);
    super::upsert_step_finish(&mut session, 3, 300, usage(5, 6), None, None);

    let indices: Vec<u32> = session.steps.iter().map(|s| s.index).collect();
    assert_eq!(indices, vec![1, 2, 3]);
    assert_eq!(session.steps[0].finished_ms, Some(100));
    assert_eq!(session.steps[1].finished_ms, Some(200));
    assert_eq!(session.steps[2].finished_ms, Some(300));
}
