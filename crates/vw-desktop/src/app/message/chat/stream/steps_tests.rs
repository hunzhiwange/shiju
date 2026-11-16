// 宿主契约：核对请求定位、活动/后台步骤投影及计时和费用字段的联动。
//! 验证会话步骤和工具生命周期事件的幂等合并与用量更新。
//!
//! 步骤夹具注入乱序、重复和缺字段事件，现有可选值不能被后到的空值覆盖。

use crate::app::models::TokenUsage;
use crate::app::state::{AcpHistoryReplayMode, AgentRequest};
use gateway_chat_stream::{GatewayChatToolEvent, GatewayChatToolEventKind, GatewayChatToolStatus};

fn usage(input_tokens: i64, output_tokens: i64) -> TokenUsage {
    TokenUsage { input_tokens, output_tokens, cached_tokens: 3, reasoning_tokens: 4 }
}

fn agent_request(id: u64, session_id: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id,
        session: session_id.to_string(),
        query: String::new(),
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
fn tool_timing_lifecycle_handler_updates_active_step() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.session_state.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).request_lifecycle.active_agent_request =
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

    let timing = &app.chat_view.active_session_view_state.steps[0].tool_timings[0];
    assert_eq!(timing.call_id, "call-1");
    assert_eq!(timing.started_ms, Some(100));
}

// 步骤开始：插入、排序并保留后续事件未提供的既有可选字段。

// 步骤结束：合并用量、结束原因与快照，并处理缺失的开始事件。

// 开始事件处理：校验请求归属后再同步活动视图和缓存会话。

#[test]
fn processing_timer_includes_gap_while_llm_timer_pauses_between_steps() {
    let mut runtime =
        crate::app::state::SessionRuntimeState::with_defaults("model".to_string(), false);
    let start = web_time::Instant::now();

    runtime.request_lifecycle.processing_timer.start_at(start);
    super::start_llm_timer_at(&mut runtime, start);
    super::pause_llm_timer_at(
        &mut runtime,
        start.checked_add(std::time::Duration::from_millis(1_250)).unwrap(),
    );
    let during_tools = start.checked_add(std::time::Duration::from_millis(9_000)).unwrap();
    assert_eq!(runtime.request_lifecycle.processing_timer.elapsed_ms_at(during_tools), 9_000);
    assert_eq!(runtime.request_lifecycle.llm_timer.elapsed_ms_at(during_tools), 1_250);

    super::start_llm_timer_at(&mut runtime, during_tools);
    assert_eq!(
        runtime.request_lifecycle.llm_timer.elapsed_ms_at(
            during_tools.checked_add(std::time::Duration::from_millis(750)).unwrap()
        ),
        2_000
    );
    assert_eq!(
        runtime.request_lifecycle.processing_timer.elapsed_ms_at(
            during_tools.checked_add(std::time::Duration::from_millis(750)).unwrap()
        ),
        9_750
    );
}

#[test]
fn handle_agent_step_start_returns_none_for_unknown_request_id() {
    let (mut app, _) = crate::app::App::new();
    let _task = super::handle_agent_step_start(&mut app, 999, "s1".to_string(), 1, 100, None);
    // 请求不存在时应直接返回，不能产生 panic 或修改应用状态。
    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_start_returns_none_for_empty_session_id() {
    let (mut app, _) = crate::app::App::new();
    app.get_session_runtime_mut("s1").request_lifecycle.active_agent_request =
        Some(agent_request(42, "s1"));
    let _task = super::handle_agent_step_start(&mut app, 42, "".to_string(), 1, 100, None);
    // 即使找到请求，空会话标识仍必须在写入任何状态前被拒绝。
    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_start_creates_step_in_active_session_view() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.session_state.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).request_lifecycle.active_agent_request =
        Some(agent_request(42, &session_id));

    let task = super::handle_agent_step_start(
        &mut app,
        42,
        session_id,
        1,
        100,
        Some("model-x".to_string()),
    );

    assert_eq!(task.units(), 0, "step projection must not submit a full UI snapshot");
    assert_eq!(app.chat_view.active_session_view_state.steps.len(), 1);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].started_ms, 100);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].model.as_deref(), Some("model-x"));
}

#[test]
fn handle_agent_step_start_keeps_inactive_session_out_of_active_view() {
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("active-session".to_string());
    app.get_session_runtime_mut("background-session").request_lifecycle.active_agent_request =
        Some(agent_request(42, "background-session"));

    let _task = super::handle_agent_step_start(
        &mut app,
        42,
        "background-session".to_string(),
        3,
        100,
        Some("model-x".to_string()),
    );

    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

// 结束事件处理：只更新匹配请求，并安排后续成本加载。

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
    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_finish_returns_none_for_empty_session_id() {
    let (mut app, _) = crate::app::App::new();
    app.get_session_runtime_mut("s1").request_lifecycle.active_agent_request =
        Some(agent_request(42, "s1"));
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
    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

#[test]
fn handle_agent_step_finish_updates_token_usage_for_active_session() {
    let (mut app, _) = crate::app::App::new();
    let session_id = "session-1".to_string();
    app.session_state.active_session_id = Some(session_id.clone());
    app.get_session_runtime_mut(&session_id).request_lifecycle.active_agent_request =
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

    assert_eq!(app.chat_view.usage.input_tokens, 10);
    assert_eq!(app.chat_view.usage.output_tokens, 20);
    assert_eq!(app.chat_view.usage.cached_tokens, 3);
    assert_eq!(app.chat_view.usage.reasoning_tokens, 4);
    assert_eq!(app.chat_view.active_session_view_state.steps.len(), 1);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].index, 1);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].usage.input_tokens, 10);
    assert_eq!(app.chat_view.active_session_view_state.steps[0].turn_id, None);
    assert!(app.chat_interaction.trajectory_raw_loading.is_empty());
}

#[test]
fn handle_agent_step_finish_does_not_charge_active_usage_for_inactive_session() {
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("active-session".to_string());
    app.get_session_runtime_mut("background-session").request_lifecycle.active_agent_request =
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

    assert_eq!(app.chat_view.usage, TokenUsage::default());
    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}

// 成本回填：结果只能写入原请求对应的步骤，过期结果应被忽略。

#[test]
fn handle_agent_step_cost_loaded_keeps_inactive_step_out_of_active_view() {
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("active-session".to_string());

    let _task = super::handle_agent_step_cost_loaded(
        &mut app,
        "background-session".to_string(),
        1,
        Some("openai/gpt-4".to_string()),
        Some(0.05),
    );

    assert!(app.chat_view.active_session_view_state.steps.is_empty());
}
