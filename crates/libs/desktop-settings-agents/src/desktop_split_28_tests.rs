//! Local agent message behavior; desktop effects are covered by host contracts.
use crate::agents_update::{self as domain, build_agents_settings_patch};
use crate::agents_update_message::{AgentsMessage, Message, SettingsMessage};
use crate::{agents_state::AgentsSettingsState, state::*};
use config_agent::DelegateCapabilityPolicy;
use desktop_settings_skills::types::SkillsCatalogKind;
use iced::{Task, widget::text_editor};
use serde_json::Value;
use std::sync::Arc;

fn input(state: &mut AgentsSettingsState) -> domain::Update<'_> {
    domain::Update {
        state,
        access: desktop_config::ConfigAccess::new(
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            None,
        ),
        client: Err("offline fixture".into()),
        acp_agents: &[],
        acp_loading: false,
        new_session_creating: false,
        project_path: None,
        validation_error: None,
        effects: Vec::new(),
    }
}
fn update(state: &mut AgentsSettingsState, message: SettingsMessage) -> Task<Message> {
    domain::update(&mut input(state), message)
}
fn handle_agents_config_saved(
    state: &mut AgentsSettingsState,
    result: Result<(), String>,
) -> Task<Message> {
    domain::handle_agents_config_saved(&mut input(state), result)
}
fn test_app() -> AgentsSettingsState {
    AgentsSettingsState { loading: false, config_ready: true, ..Default::default() }
}
fn entry<'a>(app: &'a AgentsSettingsState, key: &str) -> &'a DelegateAgentSettingsEntry {
    app.entries.iter().find(|entry| entry.key == key).expect("missing agent settings entry")
}

fn entry_mut<'a>(
    app: &'a mut AgentsSettingsState,
    key: &str,
) -> &'a mut DelegateAgentSettingsEntry {
    app.entries.iter_mut().find(|entry| entry.key == key).expect("missing agent settings entry")
}

fn config_with_main_skills(skills: &[&str]) -> Box<config_aggregate::Config> {
    let mut config = config_aggregate::Config::default();
    let main = config
        .agents
        .entry("main".to_string())
        .or_insert_with(|| config_agent::builtin_agent_config("main").unwrap_or_default());
    main.allowed_skills = skills.iter().map(|skill| (*skill).to_string()).collect();
    Box::new(config)
}

fn skill_item(id: &str, source: &str, enabled: bool) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: id.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed: true,
        enabled,
        source: source.to_string(),
        source_path: None,
    }
}

#[test]
fn project_config_loaded_does_not_replace_global_agents() {
    let mut app = test_app();
    app.project_path = Some("/project-a".to_string());
    app.loading = true;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectConfigLoaded {
            project_path: "/project-a".to_string(),
            result: Ok(config_with_main_skills(&["project-a-skill"])),
        }),
    );

    assert!(entry(&app, "main").allowed_skills.is_empty());
    assert!(app.loading);
    assert!(app.save_error.is_none());
}

#[test]
fn project_config_loaded_ignores_stale_project_response() {
    let mut app = test_app();
    app.project_path = Some("/project-b".to_string());
    entry_mut(&mut app, "main").allowed_skills = vec!["project-b-skill".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectConfigLoaded {
            project_path: "/project-a".to_string(),
            result: Ok(config_with_main_skills(&["project-a-skill"])),
        }),
    );

    assert_eq!(app.project_path.as_deref(), Some("/project-b"));
    assert_eq!(entry(&app, "main").allowed_skills, vec!["project-b-skill"]);
}

#[test]
fn config_loaded_restores_persisted_agent_values_for_matching_scope() {
    let mut app = test_app();
    let mut config = config_with_main_skills(&[]);
    config.agents.get_mut("main").expect("main agent").max_iterations = 123;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
            project_path: None,
            result: Ok(config),
        }),
    );

    assert_eq!(entry(&app, "main").max_iterations, 123);
    assert!(!app.loading);
    assert!(app.config_ready);
}

#[test]
fn config_loaded_ignores_stale_scope_after_project_changes() {
    let mut app = test_app();
    app.project_path = Some("/project-b".to_string());
    app.loading = true;
    entry_mut(&mut app, "main").max_iterations = 77;
    let mut config = config_with_main_skills(&[]);
    config.agents.get_mut("main").expect("main agent").max_iterations = 123;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
            project_path: Some("/project-a".to_string()),
            result: Ok(config),
        }),
    );

    assert_eq!(entry(&app, "main").max_iterations, 77);
    assert!(app.loading);
}

#[test]
fn project_changed_preserves_global_config_readiness() {
    let mut app = test_app();
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectChanged("/project-a".to_string())),
    );

    assert!(app.project_path.is_none());
    assert!(!app.loading);
    assert!(app.config_ready);
    assert_eq!(app.save_error.as_deref(), Some("stale error"));
}

#[test]
fn agent_changes_do_not_save_before_the_gateway_config_is_ready() {
    let mut app = test_app();
    app.config_ready = false;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxIterationsChanged("main".to_string(), 321)),
    );

    assert!(!app.save_in_flight);
    assert!(app.save_error.as_deref().is_some_and(|error| error.contains("拒绝保存默认快照")));
}

#[test]
fn agents_settings_patch_uses_current_main_model_and_combines_runtime_fields() {
    let mut app = test_app();
    let main = entry_mut(&mut app, "main");
    main.provider = " openrouter ".to_string();
    main.model = " glm-5 ".to_string();
    main.temperature = 0.2;
    main.compact_context = true;
    main.max_history_messages = 456;
    main.max_iterations = 50_002;
    main.parallel_tools = true;
    main.allow_as_subagent = false;
    main.max_parallel_subagents = 3;
    let worker_key = app
        .entries
        .iter()
        .find(|entry| entry.key != "main")
        .expect("builtin worker exists")
        .key
        .clone();
    let worker = entry_mut(&mut app, &worker_key);
    worker.compact_context = true;
    worker.max_history_messages = 234;
    worker.max_iterations = 777;
    worker.parallel_tools = true;

    let patch = build_agents_settings_patch(&app.entries).expect("patch should serialize");

    assert_eq!(patch["agents"]["main"]["provider"], "openrouter");
    assert_eq!(patch["agents"]["main"]["model"], "glm-5");
    assert!(patch["agents"]["main"].get("allow_as_subagent").is_none());
    assert_eq!(patch["agents"]["main"]["max_parallel_subagents"], 3);
    assert_eq!(patch["agents"]["main"]["max_iterations"], 50_002);
    assert_eq!(patch["agents"]["main"]["compact_context"], true);
    assert_eq!(patch["agents"]["main"]["max_history_messages"], 456);
    assert_eq!(patch["agents"]["main"]["parallel_tools"], true);
    assert_eq!(patch["default_provider"], "openrouter");
    assert_eq!(patch["default_model"], "openrouter/glm-5");
    assert!((patch["default_temperature"].as_f64().unwrap() - 0.2).abs() < f32::EPSILON as f64);
    assert!(patch.get("identity").is_none());
    assert!(patch.get("agent").is_none());
    assert_eq!(patch["agents"][&worker_key]["compact_context"], true);
    assert_eq!(patch["agents"][&worker_key]["max_history_messages"], 234);
    assert_eq!(patch["agents"][&worker_key]["max_iterations"], 777);
    assert_eq!(patch["agents"][&worker_key]["parallel_tools"], true);
}

#[test]
fn agent_provider_clear_clears_model_and_persists_null_defaults() {
    let mut app = test_app();
    entry_mut(&mut app, "main").provider = "openrouter".to_string();
    entry_mut(&mut app, "main").model = "glm-5".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProviderCleared("main".to_string())),
    );

    assert!(entry(&app, "main").provider.is_empty());
    assert!(entry(&app, "main").model.is_empty());
    let patch =
        build_agents_settings_patch(&app.entries).expect("cleared model settings should serialize");
    assert_eq!(patch["default_provider"], serde_json::Value::Null);
    assert_eq!(patch["default_model"], serde_json::Value::Null);
}

#[test]
fn agent_model_clear_keeps_provider() {
    let mut app = test_app();
    entry_mut(&mut app, "main").provider = "openrouter".to_string();
    entry_mut(&mut app, "main").model = "glm-5".to_string();

    let _ =
        update(&mut app, SettingsMessage::Agents(AgentsMessage::ModelCleared("main".to_string())));

    assert_eq!(entry(&app, "main").provider, "openrouter");
    assert!(entry(&app, "main").model.is_empty());
    let patch =
        build_agents_settings_patch(&app.entries).expect("cleared model settings should serialize");
    assert_eq!(patch["default_provider"], "openrouter");
    assert_eq!(patch["default_model"], serde_json::Value::Null);
}

#[test]
fn force_model_selection_toggle_persists() {
    let mut app = test_app();
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ForceModelSelectionToggled("main".into(), true)),
    );
    assert!(!app.profile_modal.as_ref().unwrap().force_model_selection);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProfileModelChanged(
            config_agent_profile::AgentDefault::Fixed(config_agent_profile::AgentModelReference {
                provider: "provider".into(),
                model: "model".into(),
            }),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ForceModelSelectionToggled("main".into(), true)),
    );
    let draft = app.profile_modal.as_ref().unwrap();
    assert!(draft.force_model_selection);
    let patch = build_agents_settings_patch(std::slice::from_ref(draft)).unwrap();
    assert_eq!(patch["agents"]["main"]["force_model_selection"], true);
    assert_eq!(patch["agents"]["main"]["provider"], "provider");
    assert_eq!(patch["agents"]["main"]["model"], "model");
}

#[test]
fn agents_settings_patch_omits_fields_the_ui_does_not_own() {
    let app = test_app();

    let patch = build_agents_settings_patch(&app.entries).expect("patch should serialize");
    let main = &patch["agents"]["main"];

    for field in [
        "label",
        "description",
        "builtin",
        "mode",
        "top_p",
        "identity_format",
        "hidden",
        "options",
        "permission",
        "steps",
    ] {
        assert!(main.get(field).is_none(), "unexpected destructive field: {field}");
    }
}

#[test]
fn fresh_agents_settings_patch_preserves_builtin_role_boundaries() {
    let config = config_aggregate::Config::default();
    let entries = crate::state::agent_settings_entries_from_config(&config);

    let patch = build_agents_settings_patch(&entries).expect("fresh settings should serialize");
    let researcher = &patch["agents"]["researcher"];

    for field in ["allow_as_subagent", "max_parallel_subagents", "max_depth", "agentic"] {
        assert!(researcher.get(field).is_none(), "unexpected fixed boundary: {field}");
    }
    // 最大迭代次数是所有角色均可编辑的运行参数，角色资格与递归边界仍固定。
    assert_eq!(researcher["max_iterations"], 50_000);
    assert_eq!(
        researcher["allowed_tools"],
        serde_json::json!([
            "file_read",
            "glob",
            "grep",
            "ls",
            "pdf_read",
            "web_fetch",
            "web_search_tool"
        ])
    );
}

#[test]
fn agents_settings_patch_removes_legacy_general_entry() {
    let mut config = config_aggregate::Config::default();
    config.agents.insert("general".to_string(), config_agent::DelegateAgentConfig::default());
    let entries = crate::state::agent_settings_entries_from_config(&config);

    assert!(entries.iter().all(|entry| entry.key != "general"));
    let patch = build_agents_settings_patch(&entries).expect("settings patch should serialize");

    assert_eq!(patch["agents"].get("general"), Some(&Value::Null));
}

#[test]
fn agents_settings_patch_normalizes_enabled_identity_files() {
    let mut app = test_app();
    entry_mut(&mut app, "main").enabled_identity_files = vec![
        " SOUL.md ".to_string(),
        String::new(),
        "AGENTS.md".to_string(),
        "SOUL.md".to_string(),
        " \t ".to_string(),
    ];

    let patch = build_agents_settings_patch(&app.entries).expect("patch should serialize");

    assert_eq!(
        patch["agents"]["main"]["enabled_identity_files"],
        serde_json::json!(["AGENTS.md", "SOUL.md"])
    );
}

#[test]
fn workspace_identity_enabled_toggle_adds_removes_deduplicates_and_isolates_agents() {
    let mut app = test_app();

    assert!(entry(&app, "main").enabled_identity_files.is_empty());
    assert!(entry(&app, "coder").enabled_identity_files.is_empty());
    app.selected_agent = "coder".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEnabledToggled {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            enabled: true,
        }),
    );
    assert!(app.dirty);
    assert!(!app.save_in_flight);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEnabledToggled {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            enabled: true,
        }),
    );

    assert_eq!(entry(&app, "main").enabled_identity_files, vec!["SOUL.md"]);
    assert!(entry(&app, "coder").enabled_identity_files.is_empty());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEnabledToggled {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            enabled: false,
        }),
    );

    assert!(entry(&app, "main").enabled_identity_files.is_empty());
    assert!(entry(&app, "coder").enabled_identity_files.is_empty());
}

#[test]
fn agent_subagent_messages_update_and_clamp() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowAsSubagentToggled("main".to_string(), true)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxParallelSubagentsChanged("main".to_string(), 99)),
    );

    let main = entry(&app, "main");
    assert!(main.allow_as_subagent);
    assert_eq!(main.max_parallel_subagents, 32);
}

#[test]
fn skill_policy_round_trips_without_rewriting_tool_selection() {
    let mut app = test_app();
    let main = entry_mut(&mut app, "main");
    main.allowed_tools = vec!["read".to_string()];
    main.allowed_skills = vec!["skill-a".to_string()];

    assert_eq!(main.skill_policy, DelegateCapabilityPolicy::Unrestricted);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillPolicyChanged(
            "main".to_string(),
            DelegateCapabilityPolicy::Strict,
        )),
    );

    let main = entry(&app, "main");
    assert_eq!(main.skill_policy, DelegateCapabilityPolicy::Strict);
    assert_eq!(main.allowed_tools, ["read"]);
    assert_eq!(main.allowed_skills, ["skill-a"]);

    let patch = build_agents_settings_patch(&app.entries).expect("patch should serialize");
    assert!(patch["agents"]["main"].get("tool_policy").is_none());
    assert_eq!(patch["agents"]["main"]["skill_policy"], "strict");
    assert_eq!(patch["agents"]["main"]["allowed_tools"], serde_json::json!(["read"]));
    assert_eq!(patch["agents"]["main"]["allowed_skills"], serde_json::json!(["skill-a"]));

    let reloaded_config: config_agent::DelegateAgentConfig =
        serde_json::from_value(patch["agents"]["main"].clone())
            .expect("main agent patch should deserialize");
    let reloaded_main =
        crate::state::DelegateAgentSettingsEntry::from_config("main", Some(reloaded_config));
    assert_eq!(reloaded_main.skill_policy, DelegateCapabilityPolicy::Strict);
    assert_eq!(reloaded_main.allowed_tools, ["read"]);
    assert_eq!(reloaded_main.allowed_skills, ["skill-a"]);
}

#[test]
fn refresh_marks_loading_and_clears_stale_error_before_task_runs() {
    let mut app = test_app();
    app.loading = false;
    app.project_path = Some("/stale-project".to_string());
    app.save_error = Some("stale error".to_string());

    let _task = update(&mut app, SettingsMessage::Agents(AgentsMessage::Refresh));

    assert!(app.loading);
    assert!(!app.config_ready);
    assert!(app.registered_tools.is_none());
    assert!(app.project_path.is_none());
    assert!(app.save_error.is_none());
}

#[test]
fn select_agent_on_system_prompt_updates_selection_without_loading_file() {
    let mut app = test_app();
    app.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.to_string();
    app.save_error = Some("stale error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SelectAgent("researcher".to_string())),
    );

    assert_eq!(app.selected_agent, "researcher");
    assert!(app.save_error.is_none());
}

#[test]
fn loaded_success_preserves_nonempty_tool_inventory_when_refresh_returns_empty() {
    let mut app = test_app();
    app.loading = true;
    app.available_tools = vec!["read".to_string()];
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((vec![], vec![], vec![])))),
    );

    assert!(app.loading);
    assert_eq!(app.available_tools, vec!["read".to_string()]);
    assert!(app.save_error.is_none());
}

#[test]
fn loaded_error_preserves_existing_data_while_config_load_is_pending() {
    let mut app = test_app();
    app.loading = true;
    app.available_tools = vec!["read".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Err("provider unavailable".to_string()))),
    );

    assert!(app.loading);
    assert_eq!(app.available_tools, vec!["read".to_string()]);
    assert_eq!(app.save_error.as_deref(), Some("provider unavailable"));
}

#[test]
fn add_agent_key_changed_replaces_input_and_clears_error() {
    let mut app = test_app();
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AddAgentKeyChanged("worker_2".to_string())),
    );

    assert_eq!(app.new_agent_key_input, "worker_2");
    assert!(app.save_error.is_none());
}

#[test]
fn add_agent_requested_rejects_empty_normalized_key() {
    let mut app = test_app();
    let original_entries = app.entries.len();
    app.new_agent_key_input = " ! 中文 ".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(app.entries.len(), original_entries);
    assert_eq!(app.save_error.as_deref(), Some("智能体 key 不能为空，只允许字母、数字、-、_。"));
}

#[test]
fn add_agent_requested_rejects_duplicate_normalized_key() {
    let mut app = test_app();
    let original_entries = app.entries.len();
    app.new_agent_key_input = " main! ".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(app.entries.len(), original_entries);
    assert_eq!(app.save_error.as_deref(), Some("智能体 main 已存在。"));
}

#[test]
fn tab_selection_updates_state_without_loading_system_prompt() {
    let mut app = test_app();
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::DetailTabSelected("tools".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::PromptTabSelected(
            AGENT_PROMPT_SYSTEM_TAB.to_string(),
        )),
    );

    assert_eq!(app.active_detail_tab, "tools");
    assert_eq!(app.active_prompt_tab, AGENT_PROMPT_SYSTEM_TAB);
    assert!(app.save_error.is_none());
}

#[test]
fn workspace_identity_loaded_ignores_result_for_unselected_agent() {
    let mut app = test_app();
    app.selected_agent = "main".to_string();
    app.workspace_identity_root_path = Some("original-root".to_string());
    app.save_error = Some("original error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
            agent_key: "other".to_string(),
            file_name: "SOUL.md".to_string(),
            workspace_root_path: Some("other-root".to_string()),
            result: Ok(("other content".to_string(), Some(5), Some(6))),
        }),
    );

    assert_eq!(app.workspace_identity_root_path.as_deref(), Some("original-root"));
    assert_eq!(app.save_error.as_deref(), Some("original error"));
    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "SOUL.md")
        .expect("missing SOUL.md state");
    assert!(file.editor.text().is_empty());
}

#[test]
fn workspace_identity_loaded_updates_matching_file_and_metadata() {
    let mut app = test_app();
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Ok(("new soul".to_string(), Some(8), Some(13))),
        }),
    );

    assert_eq!(app.workspace_identity_root_path.as_deref(), Some("workspace-main"));
    assert!(app.save_error.is_none());
    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "SOUL.md")
        .expect("missing SOUL.md state");
    assert_eq!(file.editor.text(), "new soul");
    assert_eq!(file.size_bytes, Some(8));
    assert_eq!(file.modified_at_ms, Some(13));
}

#[test]
fn workspace_identity_loaded_records_error_without_replacing_file() {
    let mut app = test_app();
    let file = app
        .workspace_identity_files
        .iter_mut()
        .find(|file| file.file_name == "SOUL.md")
        .expect("missing SOUL.md state");
    file.editor = iced::widget::text_editor::Content::with_text("original soul");

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Err("read failed".to_string()),
        }),
    );

    assert_eq!(app.save_error.as_deref(), Some("read failed"));
    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "SOUL.md")
        .expect("missing SOUL.md state");
    assert_eq!(file.editor.text(), "original soul");
}

#[test]
fn workspace_identity_default_restored_updates_matching_file() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key: "main".to_string(),
            file_name: "AGENTS.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Ok(("default agents".to_string(), Some(14), Some(21))),
        }),
    );

    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "AGENTS.md")
        .expect("missing AGENTS.md state");
    assert_eq!(file.editor.text(), "default agents");
    assert_eq!(file.size_bytes, Some(14));
    assert_eq!(file.modified_at_ms, Some(21));
    assert!(app.save_error.is_none());
}

#[test]
fn workspace_identity_default_restored_ignores_stale_agent_response() {
    let mut app = test_app();
    app.selected_agent = "main".to_string();
    app.workspace_identity_root_path = Some("original-root".to_string());
    app.save_error = Some("original error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key: "other".to_string(),
            file_name: "AGENTS.md".to_string(),
            workspace_root_path: Some("other-root".to_string()),
            result: Ok(("other default".to_string(), Some(13), Some(21))),
        }),
    );

    assert_eq!(app.workspace_identity_root_path.as_deref(), Some("original-root"));
    assert_eq!(app.save_error.as_deref(), Some("original error"));
    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "AGENTS.md")
        .expect("missing AGENTS.md state");
    assert!(file.editor.text().is_empty());
}

#[test]
fn workspace_identity_default_restored_records_selected_agent_error() {
    let mut app = test_app();
    app.workspace_identity_root_path = Some("original-root".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key: "main".to_string(),
            file_name: "AGENTS.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Err("restore failed".to_string()),
        }),
    );

    assert_eq!(app.workspace_identity_root_path.as_deref(), Some("workspace-main"));
    assert_eq!(app.save_error.as_deref(), Some("restore failed"));
}

#[test]
fn workspace_identity_saved_updates_metadata_and_clears_error() {
    let mut app = test_app();
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentitySaved {
            file_name: "MEMORY.md".to_string(),
            size_bytes: Some(34),
            modified_at_ms: Some(55),
            result: Ok(()),
        }),
    );

    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "MEMORY.md")
        .expect("missing MEMORY.md state");
    assert_eq!(file.size_bytes, Some(34));
    assert_eq!(file.modified_at_ms, Some(55));
    assert!(app.save_error.is_none());
}

#[test]
fn workspace_identity_saved_records_error_and_preserves_metadata() {
    let mut app = test_app();
    let file = app
        .workspace_identity_files
        .iter_mut()
        .find(|file| file.file_name == "MEMORY.md")
        .expect("missing MEMORY.md state");
    file.size_bytes = Some(89);
    file.modified_at_ms = Some(144);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentitySaved {
            file_name: "MEMORY.md".to_string(),
            size_bytes: None,
            modified_at_ms: None,
            result: Err("write failed".to_string()),
        }),
    );

    let file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "MEMORY.md")
        .expect("missing MEMORY.md state");
    assert_eq!(file.size_bytes, Some(89));
    assert_eq!(file.modified_at_ms, Some(144));
    assert_eq!(app.save_error.as_deref(), Some("write failed"));
}

#[test]
fn agent_core_field_messages_update_entries_and_clamp_numeric_bounds() {
    let mut app = test_app();
    entry_mut(&mut app, "main").provider = "old-provider".to_string();
    entry_mut(&mut app, "main").model = "old-model".to_string();
    entry_mut(&mut app, "main").enabled = true;
    app.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProviderChanged(
            "main".to_string(),
            "new-provider".to_string(),
        )),
    );
    assert_eq!(entry(&app, "main").provider, "new-provider");
    assert!(entry(&app, "main").model.is_empty());
    assert!(app.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged(
            "main".to_string(),
            "model-a".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::EnabledToggled("main".to_string(), false)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::TemperatureChanged("main".to_string(), 9.0)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxHistoryMessagesChanged(
            "main".to_string(),
            2_000,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxDepthChanged("main".to_string(), 99)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxIterationsChanged("main".to_string(), 50_001)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::CompactContextToggled("main".to_string(), true)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ParallelToolsToggled("main".to_string(), true)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AgenticToggled("main".to_string(), true)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ApiKeyChanged(
            "main".to_string(),
            "sk-test".to_string(),
        )),
    );

    let main = entry(&app, "main");
    assert_eq!(main.model, "model-a");
    assert!(main.enabled);
    assert_eq!(main.temperature, 2.0);
    assert_eq!(main.max_history_messages, 1_000);
    assert_eq!(main.max_depth, 32);
    assert_eq!(main.max_iterations, 50_001);
    assert!(main.compact_context);
    assert!(main.parallel_tools);
    assert!(main.agentic);
    assert_eq!(main.api_key_input, "sk-test");
}

#[test]
fn available_tools_exclude_mcp_dynamic_ids() {
    let mut app = test_app();
    app.available_tools.clear();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((
            vec![],
            vec![],
            vec![
                "bash".to_string(),
                "mcp__rustcodegraph__search".to_string(),
                "read".to_string(),
                "mcp__github__list_issues".to_string(),
            ],
        )))),
    );

    assert_eq!(app.available_tools, vec!["bash".to_string(), "read".to_string()]);
    assert!(!app.available_tools.iter().any(|tool| tool.starts_with("mcp__")));
}

#[test]
fn loaded_tool_catalog_removes_stale_selections_without_creating_user_edits() {
    let mut app = test_app();
    entry_mut(&mut app, "main").allowed_tools =
        vec!["removed_a".to_string(), "read".to_string(), "removed_b".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((
            vec![],
            vec![],
            vec!["read".to_string()],
        )))),
    );

    assert_eq!(app.registered_tools.as_deref().expect("tool catalog"), ["read".to_string()]);
    assert_eq!(entry(&app, "main").allowed_tools, ["read"]);
    assert!(!app.dirty);
    assert!(!app.save_in_flight);
}

#[test]
fn config_loaded_after_tool_catalog_removes_stale_selections_without_creating_user_edits() {
    let mut app = test_app();
    app.config_ready = false;
    app.available_tools = vec!["read".to_string()];
    app.registered_tools = Some(app.available_tools.clone());

    let mut config = config_aggregate::Config::default();
    let mut custom = config_agent::DelegateAgentConfig::default();
    custom.allowed_tools = vec!["removed_a".to_string(), "removed_b".to_string()];
    config.agents.insert("custom".to_string(), custom);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
            project_path: None,
            result: Ok(Box::new(config)),
        }),
    );

    assert!(entry(&app, "custom").allowed_tools.is_empty());
    assert!(!app.dirty);
    assert!(!app.save_in_flight);
}

#[test]
fn allowed_tool_messages_keep_builtin_required_tools() {
    let mut app = test_app();
    app.available_tools = vec![
        "bash".to_string(),
        "read".to_string(),
        "write".to_string(),
        "web_search_tool".to_string(),
    ];
    entry_mut(&mut app, "main").allowed_tools =
        vec!["bash".to_string(), "read".to_string(), "web_search_tool".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "bash".to_string(),
            false,
        )),
    );
    assert!(entry(&app, "main").allowed_tools.iter().any(|tool| tool == "bash"));
    assert!(entry(&app, "main").allowed_tools.iter().any(|tool| tool == "web_search_tool"));

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsInvertSelection("main".to_string())),
    );
    let inverted = &entry(&app, "main").allowed_tools;
    assert!(inverted.iter().any(|tool| tool == "bash"));
    assert!(!inverted.iter().any(|tool| tool == "web_search_tool"));
    assert!(inverted.iter().any(|tool| tool == "write"));
    assert!(inverted.iter().any(|tool| tool == "read"));

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsApplyPreset(
            "main".to_string(),
            "research".to_string(),
        )),
    );
    let preset = &entry(&app, "main").allowed_tools;
    assert!(preset.iter().any(|tool| tool == "bash"));
    assert!(preset.iter().any(|tool| tool == "web_search_tool"));
    assert!(preset.iter().any(|tool| tool == "read"));
    assert!(preset.iter().any(|tool| tool == "write"));
}

#[test]
fn allowed_tool_messages_sort_select_invert_and_apply_presets() {
    let mut app = test_app();
    app.available_tools = vec![
        "write".to_string(),
        "read".to_string(),
        "bash".to_string(),
        "web_search_tool".to_string(),
    ];
    entry_mut(&mut app, "main").allowed_tools = vec!["read".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "bash".to_string(),
            true,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "bash".to_string(),
            true,
        )),
    );
    assert_eq!(entry(&app, "main").allowed_tools, vec!["bash", "read"]);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "read".to_string(),
            false,
        )),
    );
    // 读取文件属于必需能力，不能取消。
    assert_eq!(entry(&app, "main").allowed_tools, vec!["bash".to_string(), "read".to_string()]);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsSelectAll("main".to_string())),
    );
    let mut expected_all = app.available_tools.clone();
    expected_all.sort();
    assert_eq!(entry(&app, "main").allowed_tools, expected_all);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsInvertSelection("main".to_string())),
    );
    // 反选后只保留必需能力及兼容入口。
    assert_eq!(
        entry(&app, "main").allowed_tools,
        vec!["bash".to_string(), "read".to_string(), "write".to_string()]
    );

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsApplyPreset(
            "main".to_string(),
            "research".to_string(),
        )),
    );
    assert_eq!(
        entry(&app, "main").allowed_tools,
        vec![
            "bash".to_string(),
            "read".to_string(),
            "web_search_tool".to_string(),
            "write".to_string()
        ]
    );
}

#[test]
fn agent_config_saves_only_on_click_and_retains_edits_during_save() {
    let mut app = test_app();
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "first".into())),
    );
    assert!(app.dirty);
    assert!(!app.save_in_flight);

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.save_in_flight);
    assert!(!app.dirty);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "second".into())),
    );
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.dirty, "duplicate save must not consume later edits");
    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(!app.save_in_flight);
    assert!(app.dirty);
    assert_eq!(entry(&app, "main").model, "second");

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(!app.save_in_flight);
    assert!(!app.dirty);
}

#[test]
fn agent_config_failed_save_keeps_draft_for_manual_retry() {
    let mut app = test_app();
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "saved-later".into())),
    );
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    let _ = handle_agents_config_saved(&mut app, Err("offline".into()));
    assert!(app.dirty);
    assert!(!app.save_in_flight);
    assert!(app.save_error.as_deref().unwrap().contains("offline"));
    assert_eq!(entry(&app, "main").model, "saved-later");
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.save_in_flight);
    assert!(app.save_error.is_none());
}

#[test]
fn agent_config_draft_survives_refresh_scope_change_and_stale_load() {
    let mut app = test_app();
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "draft".into())),
    );
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Refresh));
    let _ =
        update(&mut app, SettingsMessage::Agents(AgentsMessage::ProjectChanged("/other".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ConfigLoaded {
            project_path: None,
            result: Ok(Box::default()),
        }),
    );
    assert_eq!(entry(&app, "main").model, "draft");
    assert!(app.dirty);
    assert!(app.project_path.is_none());
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::DiscardChanges));
    assert!(!app.dirty);
    assert!(!app.config_ready);
    assert!(app.loading);
}

#[test]
fn add_agent_defaults_exclude_scheduled_tasks() {
    let mut app = test_app();
    app.available_tools = vec![
        "bash".to_string(),
        "read".to_string(),
        "schedule".to_string(),
        "cron_list".to_string(),
        "pdf_read".to_string(),
        "memory_recall".to_string(),
        "Config".to_string(),
        "screenshot".to_string(),
        "image_asset_read".to_string(),
        "create_thread".to_string(),
    ];
    app.new_agent_key_input = "worker_2".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(
        app.profile_modal.as_ref().unwrap().allowed_tools,
        vec![
            "Config",
            "bash",
            "create_thread",
            "image_asset_read",
            "memory_recall",
            "pdf_read",
            "read",
            "screenshot"
        ]
    );
}

#[test]
fn allowed_skill_messages_filter_by_scope_and_toggle_membership() {
    let mut app = test_app();
    app.skill_catalog = vec![
        skill_item("project-a", "workspace", true),
        skill_item("project-disabled", "workspace", false),
        skill_item("global-a", "global", true),
        skill_item("bundled-a", "bundled", true),
    ];
    entry_mut(&mut app, "main").defaults.skills =
        config_agent_profile::AgentDefault::Fixed(vec!["global-a".into()]);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsSelectAll(
            "main".to_string(),
            SkillsDirectoryScope::Project,
        )),
    );
    assert_eq!(
        entry(&app, "main").defaults.skills,
        config_agent_profile::AgentDefault::Fixed(vec!["global-a".into(), "project-a".into()])
    );

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsInvertSelection(
            "main".to_string(),
            SkillsDirectoryScope::All,
        )),
    );
    assert_eq!(
        entry(&app, "main").defaults.skills,
        config_agent_profile::AgentDefault::Fixed(vec!["bundled-a".into()])
    );

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled(
            "main".to_string(),
            "project-a".to_string(),
            true,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled(
            "main".to_string(),
            "bundled-a".to_string(),
            false,
        )),
    );
    assert_eq!(
        entry(&app, "main").defaults.skills,
        config_agent_profile::AgentDefault::Fixed(vec!["project-a".into()])
    );
}

#[test]
fn editor_actions_persist_only_for_text_edits_and_leave_unknown_files_unchanged() {
    let mut app = test_app();
    app.save_error = Some("keep".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SystemPromptAction(
            "main".to_string(),
            text_editor::Action::Move(text_editor::Motion::DocumentEnd),
        )),
    );
    assert_eq!(app.save_error.as_deref(), Some("keep"));

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SystemPromptAction(
            "main".to_string(),
            text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(
                "system prompt".to_string(),
            ))),
        )),
    );
    assert_eq!(entry(&app, "main").system_prompt_editor.text(), "system prompt");
    assert!(app.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEditorAction(
            "USER.md".to_string(),
            text_editor::Action::Edit(text_editor::Edit::Insert('u')),
        )),
    );
    let user_file = app
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "USER.md")
        .expect("missing USER.md state");
    assert_eq!(user_file.editor.text(), "u");

    app.save_error = Some("unknown remains".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEditorAction(
            "UNKNOWN.md".to_string(),
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        )),
    );
    assert_eq!(app.save_error.as_deref(), Some("unknown remains"));
}

#[test]
fn successful_catalog_refresh_preserves_unsaved_config_error() {
    let mut app = test_app();
    app.dirty = true;
    app.save_error = Some("save failed".into());
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((vec![], vec![], vec![])))),
    );
    assert_eq!(app.save_error.as_deref(), Some("save failed"));
    assert!(app.dirty);
}

#[test]
fn optional_tools_can_be_cancelled_and_remain_cancelled_after_reload() {
    let mut app = test_app();
    let ids = ["bash", "bash_wait", "bash_cancel", "pdf_read", "memory_store", "AgentTool"];
    app.available_tools = ids.iter().map(|id| (*id).to_string()).collect();
    entry_mut(&mut app, "main").allowed_tools = app.available_tools.clone();
    for id in ["pdf_read", "memory_store", "AgentTool", "bash_wait", "bash_cancel"] {
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
                "main".into(),
                id.into(),
                false,
            )),
        );
    }
    let selected = &entry(&app, "main").allowed_tools;
    assert_eq!(selected, &["bash", "bash_cancel", "bash_wait"]);
    let patch = build_agents_settings_patch(&app.entries).unwrap();
    let main = serde_json::from_value(patch["agents"]["main"].clone()).unwrap();
    let configured = std::collections::HashMap::from([("main".to_string(), main)]);
    let merged = config_agent::merged_agent_configs(&configured);
    assert_eq!(&merged["main"].allowed_tools, selected);
    let mut restored =
        crate::state::DelegateAgentSettingsEntry::from_config("main", Some(merged["main"].clone()));
    crate::state::apply_default_allowed_tools(
        std::slice::from_mut(&mut restored),
        &app.available_tools,
    );
    assert_eq!(&restored.allowed_tools, selected);
}
