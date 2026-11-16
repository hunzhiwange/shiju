use crate::message::chat::request::{AcpHistoryReplayMode, AgentRequest};
use crate::session::{agent::SessionRuntimeState as RuntimeState, runtime::*};
use desktop_settings_agents::{agents_state::AgentsSettingsState, state::MAIN_AGENT_KEY};
use std::collections::HashMap;
type SessionRuntimeState = RuntimeState<()>;
struct Fixture {
    agents_settings: AgentsSettingsState,
    session_runtime_states: HashMap<String, SessionRuntimeState>,
    active_session_id: Option<String>,
    model: String,
    auto_model: bool,
    acp_agent: Option<String>,
    acp_history_mode: AcpHistoryReplayMode,
    acp_recent_count: usize,
    chat: Vec<session_ui_types::ChatMessage>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            agents_settings: Default::default(),
            session_runtime_states: Default::default(),
            active_session_id: None,
            model: String::new(),
            auto_model: false,
            acp_agent: None,
            acp_history_mode: AcpHistoryReplayMode::Discard,
            acp_recent_count: 3,
            chat: Vec::new(),
        }
    }
}
fn test_app() -> Fixture {
    Fixture::default()
}
impl Fixture {
    fn defaults(&self) -> RuntimeDefaults {
        RuntimeDefaults {
            model: self.model.clone(),
            auto_model: self.auto_model,
            reasoning_effort: None,
            acp_history_mode: self.acp_history_mode,
            acp_recent_count: self.acp_recent_count,
            full_access: false,
            config_ready: self.agents_settings.config_ready,
        }
    }
    fn selected_static_allowed_tools(&self, runtime: &SessionRuntimeState) -> Option<Vec<String>> {
        selected_static_allowed_tools(&self.agents_settings, runtime)
    }
    fn session_tool_inventory(&self, runtime: &SessionRuntimeState) -> SessionToolInventory {
        tool_inventory(&self.agents_settings, runtime)
    }
    fn session_allowed_tools_for_request(
        &self,
        runtime: &SessionRuntimeState,
    ) -> Option<Vec<String>> {
        request_tools(&self.agents_settings, runtime)
    }
    fn get_session_runtime(&self, id: &str) -> SessionRuntimeState {
        read_runtime(
            self.session_runtime_states.get(id),
            self.agents_settings.config_ready,
            false,
            || self.defaults(),
        )
    }
    fn get_session_runtime_mut(&mut self, id: &str) -> &mut SessionRuntimeState {
        self.defaults().initialize(&mut self.session_runtime_states, id)
    }
    fn current_session_runtime(&self) -> SessionRuntimeState {
        self.get_session_runtime(self.active_session_id.as_deref().unwrap_or("__empty__"))
    }
    fn current_session_runtime_mut(&mut self) -> &mut SessionRuntimeState {
        self.get_session_runtime_mut(
            &self.active_session_id.clone().unwrap_or_else(|| "__empty__".into()),
        )
    }
    fn current_session_runtime_ref(&self) -> Option<&SessionRuntimeState> {
        self.session_runtime_states.get(self.active_session_id.as_deref().unwrap_or("__empty__"))
    }
    fn current_session_has_started(&self) -> bool {
        current_session_has_started(
            self.current_session_runtime_ref(),
            self.active_session_id.is_some(),
            !self.chat.is_empty(),
        )
    }
    fn current_session_is_requesting(&self) -> bool {
        self.current_session_runtime_ref().is_some_and(|r| r.request_lifecycle.is_requesting)
    }
    fn session_is_requesting(&self, id: &str) -> bool {
        self.session_runtime_states.get(id).is_some_and(|r| r.request_lifecycle.is_requesting)
    }
    fn any_session_requesting(&self) -> bool {
        self.session_runtime_states.values().any(|r| r.request_lifecycle.is_requesting)
    }
    fn mark_active_session_viewed(&mut self) {
        if let Some(id) = self.active_session_id.clone() {
            mark_viewed(self.get_session_runtime_mut(&id));
        }
    }
    fn find_session_by_request_id(&self, id: u64) -> Option<String> {
        find_session_by_request_id(&self.session_runtime_states, id)
    }
    fn prepare_historical_session_model(&mut self, id: &str) {
        prepare_historical_session_model(self.defaults(), &mut self.session_runtime_states, id);
    }
    fn restore_session_model_from_steps(
        &mut self,
        id: &str,
        steps: &[session_ui_types::ChatSessionStep],
    ) -> bool {
        let Some(model) = latest_history_model(steps) else {
            return false;
        };
        let Some(runtime) = self.session_runtime_states.get_mut(id) else {
            return false;
        };
        restore_session_model_from_history(runtime, &model)
    }
    fn mark_session_acp_rebuild_required(&mut self, id: &str) {
        mark_session_acp_rebuild_required(self.get_session_runtime_mut(id));
    }
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
fn runtime_tests_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("runtime_tests"));
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
fn restricted_request_preserves_registered_agent_tool() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string()];
    app.agents_settings.registered_tools =
        Some(vec!["shell".to_string(), "AgentTool".to_string(), "unselected_tool".to_string()]);

    assert_eq!(
        app.session_allowed_tools_for_request(&runtime),
        Some(vec!["AgentTool".to_string(), "shell".to_string()])
    );
}

#[test]
fn restricted_request_does_not_invent_unregistered_agent_tool() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools = vec!["shell".to_string()];
    app.agents_settings.registered_tools = Some(vec!["shell".to_string()]);

    assert_eq!(app.session_allowed_tools_for_request(&runtime), Some(vec!["shell".to_string()]));
}

#[test]
fn empty_restricted_request_keeps_only_registered_agent_tool() {
    let mut app = test_app();
    let runtime = SessionRuntimeState::default();
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == MAIN_AGENT_KEY)
        .expect("main agent should exist")
        .allowed_tools
        .clear();
    app.agents_settings.registered_tools = Some(vec!["AgentTool".to_string()]);

    assert_eq!(
        app.session_allowed_tools_for_request(&runtime),
        Some(vec!["AgentTool".to_string()])
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
fn current_session_started_covers_chat_active_request_and_queue() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());
    app.session_runtime_states.clear();

    assert!(app.current_session_has_started());
    app.current_session_runtime_mut().draft = Some(crate::session::agent::SessionDraft {
        project_path: "/repo".into(),
        directory: "/repo".into(),
        history_len: 0,
    });
    assert!(!app.current_session_has_started());

    app.chat.push(session_ui_types::ChatMessage {
        role: session_ui_types::ChatRole::User,
        content: "started".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    assert!(app.current_session_has_started());

    app.chat.clear();
    app.current_session_runtime_mut().request_lifecycle.active_agent_request =
        Some(agent_request(41, "active"));
    assert!(app.current_session_has_started());

    let runtime = app.current_session_runtime_mut();
    runtime.request_lifecycle.active_agent_request = None;
    runtime.request_lifecycle.queue.push(crate::message::chat::request::QueueItem {
        created_ms: 10,
        query: "queued".to_string(),
        attachments: Vec::new(),
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
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior: crate::message::chat::request::ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
    });
    assert!(app.current_session_has_started());
}

#[test]
fn viewed_and_requesting_helpers_update_and_query_per_session_state() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.active_session_id = Some("active".to_string());
    let mut active = SessionRuntimeState::default();
    active.has_unseen_success = true;
    active.request_lifecycle.is_requesting = true;
    app.session_runtime_states.insert("active".to_string(), active);

    assert!(app.current_session_is_requesting());
    assert!(app.session_is_requesting("active"));
    assert!(app.any_session_requesting());
    assert!(!app.session_is_requesting("missing"));

    app.mark_active_session_viewed();
    assert!(!app.get_session_runtime("active").has_unseen_success);
    app.session_runtime_states
        .get_mut("active")
        .expect("active state")
        .request_lifecycle
        .is_requesting = false;
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
    first.request_lifecycle.active_agent_request = Some(agent_request(41, "first"));
    let mut second = SessionRuntimeState::default();
    second.request_lifecycle.active_agent_request = Some(agent_request(42, "second"));
    app.session_runtime_states.insert("first".to_string(), first);
    app.session_runtime_states.insert("second".to_string(), second);

    assert_eq!(app.find_session_by_request_id(42).as_deref(), Some("second"));
    assert_eq!(app.find_session_by_request_id(99), None);
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
    let steps = vec![session_ui_types::ChatSessionStep {
        index: 1,
        turn_id: None,
        started_ms: 1,
        finished_ms: Some(2),
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage: session_ui_types::TokenUsage::default(),
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
fn fork_selection_runtime_inherited_history_does_not_start_fork() {
    let mut app = test_app();
    app.active_session_id = Some("fork".into());
    app.chat = (0..6)
        .map(|index| session_ui_types::ChatMessage {
            role: if index % 2 == 0 {
                session_ui_types::ChatRole::User
            } else {
                session_ui_types::ChatRole::Assistant
            },
            content: format!("继承历史 {index}"),
            model: None,
            think_timing: vec![],
            processing_timing: None,
        })
        .collect();
    app.current_session_runtime_mut().fork_execution_started = Some(false);
    assert!(!app.current_session_has_started());
    app.current_session_runtime_mut().request_lifecycle.is_requesting = true;
    assert!(app.current_session_has_started());
    app.current_session_runtime_mut().request_lifecycle.is_requesting = false;
    app.current_session_runtime_mut().fork_execution_started = Some(true);
    assert!(app.current_session_has_started());
    app.current_session_runtime_mut().fork_execution_started = None;
    assert!(app.current_session_has_started());
}
