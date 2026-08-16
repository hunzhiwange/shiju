use crate::app::models::{ChatMessage, ChatRole, ChatSession, ChatSessionStep, TokenUsage};
use crate::app::state::{AcpHistoryReplayMode, AgentRequest};
use vw_gateway_client::{GatewayChatToolEvent, GatewayChatToolEventKind, GatewayChatToolStatus};

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

fn agent_request(id: u64, session_id: &str) -> AgentRequest {
    AgentRequest {
        id,
        session: session_id.to_string(),
        query: String::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    }
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
fn tool_timing_lifecycle_handler_updates_active_step() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));
    let _task = super::handle_agent_step_start(
        &mut app,
        42,
        session_id,
        3,
        80,
        Some("provider/model".to_string()),
    );

    let _task = super::handle_agent_tool_lifecycle(
        &mut app,
        42,
        &tool_event(GatewayChatToolEventKind::Start, 100),
    );

    let timing = &app.active_session_view_state.steps[0].tool_timings[0];
    assert_eq!(timing.call_id, "call-1");
    assert_eq!(timing.started_ms, Some(100));
}

// ============ upsert_step_start ============

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
    // Update only model, keep snapshot path unchanged
    super::upsert_step_start(&mut session, 1, 20, Some("updated".to_string()), None);

    assert_eq!(session.steps[0].model.as_deref(), Some("updated"));
    assert_eq!(session.steps[0].start_snapshot_path.as_deref(), Some("snap"));
    assert_eq!(session.steps[0].started_ms, 20);
}

// ============ upsert_step_finish ============

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

// ============ handle_agent_step_start ============

#[test]
fn processing_timer_includes_gap_while_llm_timer_pauses_between_steps() {
    let mut runtime =
        crate::app::state::SessionRuntimeState::with_defaults("model".to_string(), false);
    let start = web_time::Instant::now();

    runtime.processing_timer.start_at(start);
    super::start_llm_timer_at(&mut runtime, start);
    super::pause_llm_timer_at(
        &mut runtime,
        start.checked_add(std::time::Duration::from_millis(1_250)).unwrap(),
    );
    let during_tools = start.checked_add(std::time::Duration::from_millis(9_000)).unwrap();
    assert_eq!(runtime.processing_timer.elapsed_ms_at(during_tools), 9_000);
    assert_eq!(runtime.llm_timer.elapsed_ms_at(during_tools), 1_250);

    super::start_llm_timer_at(&mut runtime, during_tools);
    assert_eq!(
        runtime.llm_timer.elapsed_ms_at(
            during_tools.checked_add(std::time::Duration::from_millis(750)).unwrap()
        ),
        2_000
    );
    assert_eq!(
        runtime.processing_timer.elapsed_ms_at(
            during_tools.checked_add(std::time::Duration::from_millis(750)).unwrap()
        ),
        9_750
    );
}

#[test]
fn handle_agent_step_start_returns_none_for_unknown_request_id() {
    let (mut app, _) = crate::app::App::new();
    let _task = super::handle_agent_step_start(&mut app, 999, "s1".to_string(), 1, 100, None);
    // Early return: no panic, no side effects on app
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_start_returns_none_for_empty_session_id() {
    let (mut app, _) = crate::app::App::new();
    app.get_session_runtime_mut("s1").active_agent_request = Some(agent_request(42, "s1"));
    let _task = super::handle_agent_step_start(&mut app, 42, "".to_string(), 1, 100, None);
    // Early return after request found but session_id empty
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_start_creates_step_in_active_session_view() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let _task = super::handle_agent_step_start(
        &mut app,
        42,
        session_id,
        1,
        100,
        Some("model-x".to_string()),
    );

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].started_ms, 100);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("model-x"));
}

#[test]
fn handle_agent_step_start_keeps_inactive_session_out_of_active_view() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("active-session".to_string());
    app.get_session_runtime_mut("background-session").active_agent_request =
        Some(agent_request(42, "background-session"));

    let _task = super::handle_agent_step_start(
        &mut app,
        42,
        "background-session".to_string(),
        3,
        100,
        Some("model-x".to_string()),
    );

    assert!(app.active_session_view_state.steps.is_empty());
}

// ============ handle_agent_step_finish ============

#[test]
fn handle_agent_step_finish_returns_none_for_unknown_request_id() {
    let (mut app, _) = crate::app::App::new();
    let _task = super::handle_agent_step_finish(
        &mut app,
        999,
        "s1".to_string(),
        1,
        200,
        usage(10, 20),
        None,
        None,
    );
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_finish_returns_none_for_empty_session_id() {
    let (mut app, _) = crate::app::App::new();
    app.get_session_runtime_mut("s1").active_agent_request = Some(agent_request(42, "s1"));
    let _task = super::handle_agent_step_finish(
        &mut app,
        42,
        "".to_string(),
        1,
        200,
        usage(10, 20),
        Some("stop".to_string()),
        Some("final".to_string()),
    );
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_finish_updates_token_usage_for_active_session() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).active_agent_request =
        Some(agent_request(42, &session_id));

    let _task = super::handle_agent_step_finish(
        &mut app,
        42,
        session_id,
        1,
        200,
        usage(10, 20),
        Some("stop".to_string()),
        Some("final-model".to_string()),
    );

    assert_eq!(app.usage.input_tokens, 10);
    assert_eq!(app.usage.output_tokens, 20);
    assert_eq!(app.usage.cached_tokens, 3);
    assert_eq!(app.usage.reasoning_tokens, 4);
    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].usage.input_tokens, 10);
    assert_eq!(app.active_session_view_state.steps[0].turn_id, None);
    assert!(app.trajectory_raw_loading.is_empty());
}

#[test]
fn handle_agent_step_finish_does_not_charge_active_usage_for_inactive_session() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("active-session".to_string());
    app.get_session_runtime_mut("background-session").active_agent_request =
        Some(agent_request(42, "background-session"));

    let _task = super::handle_agent_step_finish(
        &mut app,
        42,
        "background-session".to_string(),
        1,
        200,
        usage(10, 20),
        Some("stop".to_string()),
        None,
    );

    assert_eq!(app.usage, TokenUsage::default());
    assert!(app.active_session_view_state.steps.is_empty());
}

// ============ handle_agent_step_cost_loaded ============

#[test]
fn handle_agent_step_cost_loaded_returns_none_for_empty_session_id() {
    let (mut app, _) = crate::app::App::new();
    let _task = super::handle_agent_step_cost_loaded(&mut app, "".to_string(), 1, None, None);
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_cost_loaded_noop_when_step_missing_and_both_fields_none() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    let _task = super::handle_agent_step_cost_loaded(&mut app, session_id, 1, None, None);
    // No step to update, both fields None — nothing changes
    assert!(app.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_cost_loaded_creates_step_when_model_provided() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    let _task = super::handle_agent_step_cost_loaded(
        &mut app,
        session_id,
        1,
        Some("openai/gpt-4".to_string()),
        None,
    );

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, None);
}

#[test]
fn handle_agent_step_cost_loaded_creates_step_when_cost_provided() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    let _task = super::handle_agent_step_cost_loaded(&mut app, session_id, 1, None, Some(0.05));

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.05));
    assert_eq!(app.active_session_view_state.steps[0].model, None);
}

#[test]
fn handle_agent_step_cost_loaded_updates_existing_step_model_and_cost() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    // First call creates the step
    let _task1 = super::handle_agent_step_cost_loaded(
        &mut app,
        session_id.clone(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.03),
    );

    // Second call updates model and cost
    let _task2 = super::handle_agent_step_cost_loaded(
        &mut app,
        session_id,
        1,
        Some("openai/gpt-4-turbo".to_string()),
        Some(0.05),
    );

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4-turbo"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.05));
}

#[test]
fn handle_agent_step_cost_loaded_does_not_overwrite_with_none_on_existing_step() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.active_session_id = Some(session_id.clone());

    // Create step with model and cost
    let _task1 = super::handle_agent_step_cost_loaded(
        &mut app,
        session_id.clone(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.03),
    );

    // Call with both None — existing values should be preserved
    let _task2 = super::handle_agent_step_cost_loaded(&mut app, session_id, 1, None, None);

    assert_eq!(app.active_session_view_state.steps.len(), 1);
    assert_eq!(app.active_session_view_state.steps[0].model.as_deref(), Some("openai/gpt-4"));
    assert_eq!(app.active_session_view_state.steps[0].cost_usd, Some(0.03));
}

#[test]
fn handle_agent_step_cost_loaded_keeps_inactive_step_out_of_active_view() {
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("active-session".to_string());

    let _task = super::handle_agent_step_cost_loaded(
        &mut app,
        "background-session".to_string(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.05),
    );

    assert!(app.active_session_view_state.steps.is_empty());
}
