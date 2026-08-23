//! 会话运行时测试，覆盖工具清单归一化、代理限制交集和请求能力选择。
//! 空限制、静态回退与内置工具锁定均应维持默认拒绝的安全边界。

use super::*;
use crate::app::state::{
    AcpHistoryReplayMode, AgentRequest, ExploreSummaryAnimationState, SessionRuntimeState,
};

fn test_app() -> App {
    App::new().0
}

fn agent_request(id: u64, session: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id,
        session: session.to_string(),
        query: "query".to_string(),
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
    })
}

#[test]
fn runtime_tests_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("runtime_tests"));
}

#[test]
fn sorted_unique_tools_trims_drops_blanks_sorts_and_deduplicates() {
    let tools = vec![
        " write ".to_string(),
        String::new(),
        "read".to_string(),
        "  ".to_string(),
        "write".to_string(),
    ];

    assert_eq!(sorted_unique_tools(tools), vec!["read", "write"]);
}

#[test]
fn intersect_tools_preserves_left_order_and_duplicates() {
    let left = vec!["write".to_string(), "read".to_string(), "write".to_string()];
    let right = vec!["read".to_string(), "write".to_string()];

    assert_eq!(intersect_tools(&left, &right), left);
    assert!(intersect_tools(&left, &["shell".to_string()]).is_empty());
}

#[test]
fn request_allowed_tools_from_inventory_uses_filtered_tools() {
    let mut runtime = SessionRuntimeState::default();
    let inventory = crate::app::state::SessionToolInventory {
        base_tools: vec!["file_read".to_string(), "file_write".to_string()],
        restricts_tools: false,
    };

    assert_eq!(super::request_allowed_tools_from_inventory(&runtime, &inventory), None);

    runtime.tool_selector.toggle_tool(&inventory.base_tools, "file_write");

    assert_eq!(
        super::request_allowed_tools_from_inventory(&runtime, &inventory),
        Some(vec!["file_read".to_string()])
    );
}

#[test]
fn request_allowed_tools_from_inventory_preserves_agent_restriction() {
    let runtime = SessionRuntimeState::default();
    let inventory = crate::app::state::SessionToolInventory {
        base_tools: vec!["file_read".to_string()],
        restricts_tools: true,
    };

    assert_eq!(
        super::request_allowed_tools_from_inventory(&runtime, &inventory),
        Some(vec!["file_read".to_string()])
    );
}

#[test]
fn request_allowed_tools_from_inventory_rejects_empty_restricted_inventory() {
    let runtime = SessionRuntimeState::default();
    let inventory = crate::app::state::SessionToolInventory {
        base_tools: vec![String::new(), "  ".to_string()],
        restricts_tools: true,
    };

    assert_eq!(request_allowed_tools_from_inventory(&runtime, &inventory), None);
}

#[test]
fn selected_static_allowed_tools_uses_selected_agent_and_normalizes_entries() {
    let mut app = test_app();
    let mut runtime = SessionRuntimeState::default();
    let entry = app
        .agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key != MAIN_AGENT_KEY)
        .expect("a built-in delegate agent should exist");
    let selected_key = entry.key.clone();
    entry.allowed_tools =
        vec![" write ".to_string(), String::new(), "read".to_string(), "write".to_string()];
    runtime.agent = Some(selected_key);

    assert_eq!(
        app.selected_static_allowed_tools(&runtime),
        Some(vec!["read".to_string(), "write".to_string()])
    );

    runtime.agent = Some("missing-agent".to_string());
    assert_eq!(app.selected_static_allowed_tools(&runtime), None);
}

#[test]
fn selected_static_allowed_tools_treats_blank_main_configuration_as_empty_restriction() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    let main = app
        .agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist");
    main.allowed_tools = vec!["   ".to_string(), String::new()];

    assert_eq!(app.selected_static_allowed_tools(&runtime), Some(Vec::new()));
    assert!(app.session_tool_inventory(&runtime).restricts_tools);
}

#[test]
fn session_tool_inventory_handles_unrestricted_intersection_and_static_fallbacks() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    let main = app
        .agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist");
    main.allowed_tools.clear();
    app.agents_settings.available_tools =
        vec![" write ".to_string(), "read".to_string(), "read".to_string(), String::new()];
    app.agents_settings.registered_tools = None;

    let restricted_empty = app.session_tool_inventory(&runtime);
    assert!(restricted_empty.base_tools.is_empty());
    assert!(restricted_empty.restricts_tools);

    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string(), " read ".to_string()];
    let intersected = app.session_tool_inventory(&runtime);
    assert_eq!(intersected.base_tools, vec!["read"]);
    assert!(intersected.restricts_tools);

    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string()];
    let disjoint = app.session_tool_inventory(&runtime);
    assert!(disjoint.base_tools.is_empty());
    assert!(disjoint.restricts_tools);

    app.agents_settings.available_tools.clear();
    assert_eq!(app.session_allowed_tools_for_request(&runtime), Some(vec!["shell".to_string()]));
}

#[test]
fn session_tool_inventory_includes_registered_builtin_tools_locked_by_settings() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    let main = app
        .agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist");
    main.allowed_tools = vec!["custom_tool".to_string()];
    app.agents_settings.available_tools = vec![
        "custom_tool".to_string(),
        "schedule".to_string(),
        "bash".to_string(),
        "file_read".to_string(),
    ];
    app.agents_settings.registered_tools = Some(app.agents_settings.available_tools.clone());

    let inventory = app.session_tool_inventory(&runtime);

    assert_eq!(inventory.base_tools, ["bash", "custom_tool", "file_read", "schedule"]);
    assert!(inventory.restricts_tools);
    assert_eq!(
        app.session_allowed_tools_for_request(&runtime),
        Some(vec![
            "bash".to_string(),
            "custom_tool".to_string(),
            "file_read".to_string(),
            "schedule".to_string(),
        ])
    );
}

#[test]
fn session_tool_inventory_preserves_selected_custom_registered_tools() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    let main = app
        .agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist");
    main.allowed_tools = vec!["custom_tool".to_string(), "mcp__custom__run".to_string()];
    app.agents_settings.available_tools = vec!["custom_tool".to_string()];
    app.agents_settings.registered_tools = Some(vec![
        "custom_tool".to_string(),
        "mcp__custom__run".to_string(),
        "unselected_custom_tool".to_string(),
    ]);

    let inventory = app.session_tool_inventory(&runtime);

    assert_eq!(inventory.base_tools, ["custom_tool", "mcp__custom__run"]);
    assert_eq!(
        app.session_allowed_tools_for_request(&runtime),
        Some(vec!["custom_tool".to_string(), "mcp__custom__run".to_string()])
    );
}

#[test]
fn get_session_runtime_prefers_existing_state_and_uses_global_defaults_for_missing_state() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.model = "app-model".to_string();
    app.auto_model = true;
    app.acp_agent = Some("app-acp".to_string());
    app.acp_history_mode = AcpHistoryReplayMode::Summary;
    app.acp_recent_count = 7;

    let app_default = app.get_session_runtime("missing");
    assert_eq!(app_default.model, "app-model");
    assert!(app_default.auto_model);
    assert_eq!(app_default.acp_agent.as_deref(), Some("app-acp"));
    assert_eq!(app_default.acp_history_mode, AcpHistoryReplayMode::Summary);
    assert_eq!(app_default.acp_recent_count, 7);

    let mut active = SessionRuntimeState::with_defaults("active-model".to_string(), false);
    active.acp_agent = Some("stored-acp".to_string());
    app.session_runtime_states.insert("active".to_string(), active.clone());
    app.active_session_id = Some("active".to_string());

    let global_default = app.get_session_runtime("another-missing");
    assert_eq!(global_default.model, "app-model");
    assert!(global_default.auto_model);
    assert_eq!(app.get_session_runtime("active").acp_agent, active.acp_agent);
}

#[test]
fn get_session_runtime_mut_initializes_and_refreshes_acp_preferences_without_overwriting_agent() {
    let mut app = test_app();
    app.active_session_id = None;
    app.session_runtime_states.clear();
    app.model = "model-a".to_string();
    app.auto_model = true;
    app.acp_agent = Some("default-acp".to_string());
    app.acp_history_mode = AcpHistoryReplayMode::Summary;
    app.acp_recent_count = 4;

    let created = app.get_session_runtime_mut("session");
    assert_eq!(created.model, "model-a");
    assert!(created.auto_model);
    assert_eq!(created.acp_agent.as_deref(), Some("default-acp"));
    assert_eq!(created.acp_history_mode, AcpHistoryReplayMode::Summary);
    assert_eq!(created.acp_recent_count, 4);
    created.acp_agent = Some("session-acp".to_string());

    app.acp_agent = Some("new-default".to_string());
    app.acp_history_mode = AcpHistoryReplayMode::Full;
    app.acp_recent_count = 9;
    let refreshed = app.get_session_runtime_mut("session");
    assert_eq!(refreshed.acp_agent.as_deref(), Some("session-acp"));
    assert_eq!(refreshed.acp_history_mode, AcpHistoryReplayMode::Full);
    assert_eq!(refreshed.acp_recent_count, 9);
}

#[test]
fn current_session_accessors_switch_between_empty_and_active_runtime() {
    let mut app = test_app();
    app.active_session_id = None;
    app.session_runtime_states.clear();

    app.current_session_runtime_mut().model = "empty-model".to_string();
    assert_eq!(app.current_session_runtime().model, "empty-model");
    assert_eq!(
        app.current_session_runtime_ref().map(|runtime| runtime.model.as_str()),
        Some("empty-model")
    );

    app.active_session_id = Some("active".to_string());
    app.current_session_runtime_mut().model = "active-model".to_string();
    assert_eq!(app.current_session_runtime().model, "active-model");
    assert_eq!(
        app.current_session_runtime_ref().map(|runtime| runtime.model.as_str()),
        Some("active-model")
    );
}

#[test]
fn viewed_and_requesting_helpers_update_and_query_per_session_state() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.active_session_id = Some("active".to_string());
    let mut active = SessionRuntimeState::default();
    active.has_unseen_success = true;
    active.is_requesting = true;
    app.session_runtime_states.insert("active".to_string(), active);

    assert!(app.current_session_is_requesting());
    assert!(app.session_is_requesting("active"));
    assert!(app.any_session_requesting());
    assert!(!app.session_is_requesting("missing"));

    app.mark_active_session_viewed();
    assert!(!app.get_session_runtime("active").has_unseen_success);
    app.session_runtime_states.get_mut("active").expect("active state").is_requesting = false;
    assert!(!app.current_session_is_requesting());
    assert!(!app.any_session_requesting());

    app.active_session_id = None;
    app.mark_active_session_viewed();
}

#[test]
fn find_session_by_request_id_returns_matching_session_only() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    let mut first = SessionRuntimeState::default();
    first.active_agent_request = Some(agent_request(41, "first"));
    let mut second = SessionRuntimeState::default();
    second.active_agent_request = Some(agent_request(42, "second"));
    app.session_runtime_states.insert("first".to_string(), first);
    app.session_runtime_states.insert("second".to_string(), second);

    assert_eq!(app.find_session_by_request_id(42).as_deref(), Some("second"));
    assert_eq!(app.find_session_by_request_id(99), None);
}

#[test]
fn sync_active_session_preferences_preserves_global_model_and_syncs_other_values() {
    let mut app = test_app();
    app.active_session_id = None;
    app.model = "unchanged".to_string();
    app.auto_model = false;
    app.sync_active_session_preferences();
    assert_eq!(app.model, "unchanged");
    assert!(!app.auto_model);

    let mut runtime = SessionRuntimeState::with_defaults("session-model".to_string(), true);
    runtime.reasoning_effort = Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High);
    runtime.acp_agent = Some("session-acp".to_string());
    runtime.acp_history_mode = AcpHistoryReplayMode::Recent;
    runtime.acp_recent_count = 12;
    app.session_runtime_states.insert("active".to_string(), runtime);
    app.active_session_id = Some("active".to_string());

    app.sync_active_session_preferences();

    assert_eq!(app.model, "unchanged");
    assert!(!app.auto_model);
    assert_eq!(
        app.reasoning_effort,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::High)
    );
    assert_eq!(app.acp_agent.as_deref(), Some("session-acp"));
    assert_eq!(app.acp_history_mode, AcpHistoryReplayMode::Recent);
    assert_eq!(app.acp_recent_count, 12);
}

#[test]
fn historical_session_model_restore_uses_latest_recorded_model() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.model = "previous-session-model".to_string();
    app.auto_model = false;

    app.prepare_historical_session_model("history");
    app.active_session_id = Some("history".to_string());
    app.sync_active_session_preferences();

    assert_eq!(app.model, "previous-session-model");
    assert!(!app.auto_model);
    assert_eq!(app.get_session_runtime("history").model, "previous-session-model");

    let steps = vec![
        crate::app::models::ChatSessionStep {
            index: 1,
            turn_id: None,
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: crate::app::models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: Some("provider/old-model".to_string()),
            tool_timings: Vec::new(),
        },
        crate::app::models::ChatSessionStep {
            index: 2,
            turn_id: None,
            started_ms: 3,
            finished_ms: Some(4),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: crate::app::models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: Some(" provider/history-model ".to_string()),
            tool_timings: Vec::new(),
        },
    ];

    assert!(app.restore_session_model_from_steps("history", &steps));
    app.sync_active_session_preferences();

    assert_eq!(app.model, "previous-session-model");
    assert!(!app.auto_model);
    let runtime = app.session_runtime_states.get("history").expect("history runtime");
    assert_eq!(runtime.model, "provider/history-model");
    assert_eq!(runtime.task_mode_model, "provider/history-model");
    assert!(!runtime.restore_model_from_history);
}

#[test]
fn historical_session_model_restore_preserves_existing_session_selection() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.session_runtime_states.insert(
        "history".to_string(),
        SessionRuntimeState::with_defaults("session-selection".to_string(), false),
    );

    app.prepare_historical_session_model("history");
    let steps = vec![crate::app::models::ChatSessionStep {
        index: 1,
        turn_id: None,
        started_ms: 1,
        finished_ms: Some(2),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: crate::app::models::TokenUsage::default(),
        cost_usd: None,
        finish_reason: None,
        model: Some("provider/history-model".to_string()),
        tool_timings: Vec::new(),
    }];

    assert!(!app.restore_session_model_from_steps("history", &steps));
    assert_eq!(app.get_session_runtime("history").model, "session-selection");
}

#[test]
fn mark_session_acp_rebuild_required_sets_flag_and_clears_effective_agent() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    let mut runtime = SessionRuntimeState::default();
    runtime.last_effective_acp_agent = Some("old-agent".to_string());
    app.session_runtime_states.insert("session".to_string(), runtime);

    app.mark_session_acp_rebuild_required("session");

    let runtime = app.session_runtime_states.get("session").expect("session state");
    assert!(runtime.acp_rebuild_required);
    assert_eq!(runtime.last_effective_acp_agent, None);
}

#[test]
fn animation_helpers_detect_future_change_and_wrap_frame_without_expiring_idle_state() {
    let mut app = test_app();
    app.chat_explore_summary_animations.clear();
    assert!(!app.has_active_explore_summary_animation());

    app.chat_explore_summary_animations.insert(
        1,
        ExploreSummaryAnimationState {
            previous_summary_text: "previous".to_string(),
            current_summary_text: "current".to_string(),
            changed_at_ms: Some(u64::MAX),
        },
    );
    assert!(app.has_active_explore_summary_animation());

    app.chat_explore_summary_animations.insert(
        2,
        ExploreSummaryAnimationState {
            previous_summary_text: "stable".to_string(),
            current_summary_text: "stable".to_string(),
            changed_at_ms: None,
        },
    );
    app.status_animation_frame = usize::MAX;
    app.advance_status_animation_frame();

    assert_eq!(app.status_animation_frame, 0);
    let stable = app.chat_explore_summary_animations.get(&2).expect("stable animation state");
    assert_eq!(stable.previous_summary_text, "stable");
    assert_eq!(stable.changed_at_ms, None);
}

#[test]
fn advance_status_animation_frame_expires_old_summary_animation() {
    let mut app = test_app();
    app.chat_explore_summary_animations.clear();
    app.chat_explore_summary_animations.insert(
        7,
        ExploreSummaryAnimationState {
            previous_summary_text: "old".to_string(),
            current_summary_text: "new".to_string(),
            changed_at_ms: Some(0),
        },
    );

    app.advance_status_animation_frame();

    let state = app.chat_explore_summary_animations.get(&7).expect("animation state");
    assert_eq!(state.previous_summary_text, "new");
    assert_eq!(state.current_summary_text, "new");
    assert_eq!(state.changed_at_ms, None);
}
