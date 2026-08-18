//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use super::{
    build_agents_settings_patch, bundled_workspace_identity_content,
    bundled_workspace_identity_path, handle_agents_config_saved, normalize_agent_key,
    summarize_models, summarize_providers, tool_in_preset, tools_for_preset, update,
};
use crate::app::{
    App,
    message::settings::{AgentsMessage, SettingsMessage},
    state::{
        AGENT_PROMPT_SYSTEM_TAB, DelegateAgentSettingsEntry, SkillsCatalogItem, SkillsCatalogKind,
        SkillsDirectoryScope,
    },
};
use iced::widget::text_editor;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use vw_config_types::agent::DelegateCapabilityPolicy;
use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelLimit, ProviderSource,
};

fn test_app() -> App {
    let mut app = App::new().0;
    app.agents_settings.loading = false;
    app.agents_settings.config_ready = true;
    app
}

fn entry<'a>(app: &'a App, key: &str) -> &'a DelegateAgentSettingsEntry {
    app.agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == key)
        .expect("missing agent settings entry")
}

fn entry_mut<'a>(app: &'a mut App, key: &str) -> &'a mut DelegateAgentSettingsEntry {
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == key)
        .expect("missing agent settings entry")
}

fn config_with_main_skills(skills: &[&str]) -> Box<vw_config_types::config::Config> {
    let mut config = vw_config_types::config::Config::default();
    let main = config.agents.entry("main".to_string()).or_insert_with(|| {
        vw_config_types::agent::builtin_agent_config("main").unwrap_or_default()
    });
    main.allowed_skills = skills.iter().map(|skill| (*skill).to_string()).collect();
    Box::new(config)
}

#[test]
fn project_config_loaded_applies_matching_project_skills() {
    let mut app = test_app();
    app.agents_settings.project_path = Some("/project-a".to_string());
    app.agents_settings.loading = true;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectConfigLoaded {
            project_path: "/project-a".to_string(),
            result: Ok(config_with_main_skills(&["project-a-skill"])),
        }),
    );

    assert_eq!(entry(&app, "main").allowed_skills, vec!["project-a-skill"]);
    assert!(!app.agents_settings.loading);
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn project_config_loaded_ignores_stale_project_response() {
    let mut app = test_app();
    app.agents_settings.project_path = Some("/project-b".to_string());
    entry_mut(&mut app, "main").allowed_skills = vec!["project-b-skill".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectConfigLoaded {
            project_path: "/project-a".to_string(),
            result: Ok(config_with_main_skills(&["project-a-skill"])),
        }),
    );

    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/project-b"));
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
    assert!(!app.agents_settings.loading);
    assert!(app.agents_settings.config_ready);
}

#[test]
fn config_loaded_ignores_stale_scope_after_project_changes() {
    let mut app = test_app();
    app.agents_settings.project_path = Some("/project-b".to_string());
    app.agents_settings.loading = true;
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
    assert!(app.agents_settings.loading);
}

#[test]
fn project_changed_tracks_save_scope_before_loading() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectChanged("/project-a".to_string())),
    );

    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/project-a"));
    assert!(app.agents_settings.loading);
    assert!(!app.agents_settings.config_ready);
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn agent_changes_do_not_save_before_the_gateway_config_is_ready() {
    let mut app = test_app();
    app.agents_settings.config_ready = false;

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::MaxIterationsChanged("main".to_string(), 321)),
    );

    assert!(!app.agents_settings.save_in_flight);
    assert!(
        app.agents_settings
            .save_error
            .as_deref()
            .is_some_and(|error| error.contains("拒绝保存默认快照"))
    );
}

#[test]
fn agents_settings_patch_uses_current_main_model_and_combines_runtime_fields() {
    let mut app = test_app();
    let main = entry_mut(&mut app, "main");
    main.provider = " openrouter ".to_string();
    main.model = " glm-5 ".to_string();
    main.temperature = 0.2;
    main.compact_context = true;
    main.max_tool_iterations = 50_002;
    main.max_history_messages = 456;
    main.max_iterations = 50_001;
    main.parallel_tools = true;
    main.allow_as_subagent = true;
    main.max_parallel_subagents = 3;
    let worker_key = app
        .agents_settings
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

    let patch =
        build_agents_settings_patch(&app.agents_settings.entries).expect("patch should serialize");

    assert_eq!(patch["agents"]["main"]["provider"], "openrouter");
    assert_eq!(patch["agents"]["main"]["model"], "glm-5");
    assert_eq!(patch["agents"]["main"]["allow_as_subagent"], true);
    assert_eq!(patch["agents"]["main"]["max_parallel_subagents"], 3);
    assert_eq!(patch["agents"]["main"]["max_iterations"], 50_001);
    assert_eq!(patch["default_provider"], "openrouter");
    assert_eq!(patch["default_model"], "openrouter/glm-5");
    assert_eq!(patch["default_temperature"], 0.2);
    assert!(patch.get("identity").is_none());
    assert_eq!(patch["agent"]["compact_context"], true);
    assert_eq!(patch["agent"]["max_tool_iterations"], 50_002);
    assert_eq!(patch["agent"]["max_history_messages"], 456);
    assert_eq!(patch["agent"]["parallel_tools"], true);
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
    let patch = build_agents_settings_patch(&app.agents_settings.entries)
        .expect("cleared model settings should serialize");
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
    let patch = build_agents_settings_patch(&app.agents_settings.entries)
        .expect("cleared model settings should serialize");
    assert_eq!(patch["default_provider"], "openrouter");
    assert_eq!(patch["default_model"], serde_json::Value::Null);
}

#[test]
fn force_model_selection_toggle_persists() {
    let mut app = test_app();
    assert!(!entry(&app, "main").force_model_selection);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ForceModelSelectionToggled(
            "main".to_string(),
            true,
        )),
    );

    assert!(entry(&app, "main").force_model_selection);
    let patch = build_agents_settings_patch(&app.agents_settings.entries)
        .expect("force selection should serialize");
    assert_eq!(patch["agents"]["main"]["force_model_selection"], true);
}

#[test]
fn agents_settings_patch_omits_fields_the_ui_does_not_own() {
    let app = test_app();

    let patch =
        build_agents_settings_patch(&app.agents_settings.entries).expect("patch should serialize");
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
    let config = vw_config_types::config::Config::default();
    let entries = crate::app::state::agent_settings_entries_from_config(&config);

    let patch = build_agents_settings_patch(&entries).expect("fresh settings should serialize");
    let researcher = &patch["agents"]["researcher"];

    for field in
        ["allow_as_subagent", "max_parallel_subagents", "max_depth", "agentic", "max_iterations"]
    {
        assert!(researcher.get(field).is_none(), "unexpected fixed boundary: {field}");
    }
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
    let mut config = vw_config_types::config::Config::default();
    config
        .agents
        .insert("general".to_string(), vw_config_types::agent::DelegateAgentConfig::default());
    let entries = crate::app::state::agent_settings_entries_from_config(&config);

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

    let patch =
        build_agents_settings_patch(&app.agents_settings.entries).expect("patch should serialize");

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
    app.agents_settings.selected_agent = "coder".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEnabledToggled {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            enabled: true,
        }),
    );
    assert!(app.agents_settings.save_in_flight);
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
fn capability_policy_controls_round_trip_without_rewriting_default_selections() {
    let mut app = test_app();
    let main = entry_mut(&mut app, "main");
    main.allowed_tools = vec!["read".to_string()];
    main.allowed_skills = vec!["skill-a".to_string()];

    assert_eq!(main.tool_policy, DelegateCapabilityPolicy::Unrestricted);
    assert_eq!(main.skill_policy, DelegateCapabilityPolicy::Unrestricted);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ToolPolicyChanged(
            "main".to_string(),
            DelegateCapabilityPolicy::Strict,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillPolicyChanged(
            "main".to_string(),
            DelegateCapabilityPolicy::UserOverride,
        )),
    );

    let main = entry(&app, "main");
    assert_eq!(main.tool_policy, DelegateCapabilityPolicy::Strict);
    assert_eq!(main.skill_policy, DelegateCapabilityPolicy::UserOverride);
    assert_eq!(main.allowed_tools, ["read"]);
    assert_eq!(main.allowed_skills, ["skill-a"]);

    let patch =
        build_agents_settings_patch(&app.agents_settings.entries).expect("patch should serialize");
    assert_eq!(patch["agents"]["main"]["tool_policy"], "strict");
    assert_eq!(patch["agents"]["main"]["skill_policy"], "user_override");
    assert_eq!(patch["agents"]["main"]["allowed_tools"], serde_json::json!(["read"]));
    assert_eq!(patch["agents"]["main"]["allowed_skills"], serde_json::json!(["skill-a"]));

    let reloaded: vw_config_types::config::Config =
        serde_json::from_value(patch).expect("patch should deserialize as config");
    let reloaded_entries = crate::app::state::agent_settings_entries_from_config(&reloaded);
    let reloaded_main = reloaded_entries
        .iter()
        .find(|entry| entry.key == "main")
        .expect("main entry should reload");
    assert_eq!(reloaded_main.tool_policy, DelegateCapabilityPolicy::Strict);
    assert_eq!(reloaded_main.skill_policy, DelegateCapabilityPolicy::UserOverride);
    assert_eq!(reloaded_main.allowed_tools, ["read"]);
    assert_eq!(reloaded_main.allowed_skills, ["skill-a"]);
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
fn bundled_workspace_identity_path_points_to_assets_agent() {
    assert_eq!(bundled_workspace_identity_path("SOUL.md"), "assets/agent/SOUL.md");
    assert_eq!(bundled_workspace_identity_path("MEMORY.md"), "assets/agent/MEMORY.md");
}

#[test]
fn bundled_workspace_identity_content_is_embedded() {
    let soul = bundled_workspace_identity_content("SOUL.md").expect("missing embedded SOUL.md");
    let agents =
        bundled_workspace_identity_content("AGENTS.md").expect("missing embedded AGENTS.md");

    assert!(!soul.trim().is_empty());
    assert!(!agents.trim().is_empty());
    assert!(bundled_workspace_identity_content("UNKNOWN.md").is_none());
}

#[test]
fn summarize_models_keeps_builtin_provider_models_even_when_all_disabled() {
    let model = Model {
        id: "gpt-4.1".to_string(),
        provider_id: "openai".to_string(),
        api: ApiInfo {
            id: "gpt-4.1".to_string(),
            url: "https://api.example.com/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: "GPT-4.1".to_string(),
        family: None,
        capabilities: Capabilities {
            temperature: true,
            reasoning: false,
            attachment: false,
            toolcall: true,
            input: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(false),
        },
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache: ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 128_000, input: None, output: 16_384 },
        status: "disabled".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: String::new(),
        variants: HashMap::new(),
    };

    let mut models = HashMap::new();
    models.insert(model.id.clone(), model);

    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        Info {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            source: ProviderSource::Custom,
            env: vec!["OPENAI_API_KEY".to_string()],
            key: None,
            options: HashMap::new(),
            models,
        },
    );

    let summary = summarize_models(providers);

    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].id, "openai");
    assert_eq!(summary[0].models.len(), 1);
    assert_eq!(summary[0].models[0].id, "gpt-4.1");
    assert!(!summary[0].models[0].enabled);
}

#[test]
fn summarize_providers_labels_sources_and_sorts_by_name_then_id() {
    let providers = HashMap::from([
        (
            "z".to_string(),
            Info {
                id: "z".to_string(),
                name: "Zulu".to_string(),
                source: ProviderSource::Api,
                env: vec![],
                key: None,
                options: HashMap::new(),
                models: HashMap::new(),
            },
        ),
        (
            "a".to_string(),
            Info {
                id: "a".to_string(),
                name: "Alpha".to_string(),
                source: ProviderSource::Env,
                env: vec!["ALPHA_KEY".to_string()],
                key: None,
                options: HashMap::new(),
                models: HashMap::new(),
            },
        ),
        (
            "builtin".to_string(),
            Info {
                id: "builtin".to_string(),
                name: "Builtin".to_string(),
                source: ProviderSource::Custom,
                env: vec![],
                key: None,
                options: HashMap::new(),
                models: HashMap::new(),
            },
        ),
    ]);

    let summary = summarize_providers(providers);

    assert_eq!(
        summary.iter().map(|provider| provider.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "builtin", "z"]
    );
    assert_eq!(summary[0].source_label, "环境变量");
    assert!(summary[0].connected);
    assert_eq!(summary[1].source_label, "内置");
    assert!(!summary[1].connected);
    assert_eq!(summary[2].source_label, "API 密钥");
    assert!(summary[2].connected);
}

#[test]
fn normalize_agent_key_trims_and_removes_unsupported_characters() {
    assert_eq!(normalize_agent_key("  Agent-01_beta  "), "Agent-01_beta");
    assert_eq!(normalize_agent_key(" agent key!@#中文 "), "agentkey");
    assert_eq!(normalize_agent_key("   "), "");
}

#[test]
fn tool_presets_include_expected_tools_and_sort_results() {
    let available_tools = vec![
        "bash".to_string(),
        "read".to_string(),
        "browser".to_string(),
        "apply_patch".to_string(),
        "question".to_string(),
        "unknown".to_string(),
    ];

    assert!(tool_in_preset("read", "minimal"));
    assert!(tool_in_preset("apply_patch", "coding"));
    assert!(tool_in_preset("browser", "research"));
    assert!(tool_in_preset("question", "collab"));
    assert!(tool_in_preset("unknown", "full"));
    assert!(!tool_in_preset("unknown", "minimal"));
    assert!(!tool_in_preset("read", "unknown"));

    assert_eq!(
        tools_for_preset(&available_tools, "coding"),
        vec![
            "apply_patch".to_string(),
            "bash".to_string(),
            "question".to_string(),
            "read".to_string()
        ]
    );
}

#[test]
fn tool_presets_cover_aliases_and_deduplicate_available_tools() {
    let available_tools = vec![
        "web_search_tool".to_string(),
        "read".to_string(),
        "schedule".to_string(),
        "read".to_string(),
        "file_write".to_string(),
        "unrelated".to_string(),
    ];

    assert!(tool_in_preset("web_search_tool", "research"));
    assert!(tool_in_preset("schedule", "collab"));
    assert!(tool_in_preset("file_write", "coding"));
    assert_eq!(
        tools_for_preset(&available_tools, "full"),
        vec![
            "file_write".to_string(),
            "read".to_string(),
            "schedule".to_string(),
            "unrelated".to_string(),
            "web_search_tool".to_string(),
        ]
    );
    assert!(tools_for_preset(&available_tools, "unsupported").is_empty());
}

#[test]
fn summarize_providers_labels_config_source_and_sorts_equal_names_by_id() {
    let providers = HashMap::from([
        (
            "z-config".to_string(),
            Info {
                id: "z-config".to_string(),
                name: "Same".to_string(),
                source: ProviderSource::Config,
                env: vec![],
                key: None,
                options: HashMap::new(),
                models: HashMap::new(),
            },
        ),
        (
            "a-config".to_string(),
            Info {
                id: "a-config".to_string(),
                name: "Same".to_string(),
                source: ProviderSource::Config,
                env: vec![],
                key: Some("configured-secret".to_string()),
                options: HashMap::new(),
                models: HashMap::new(),
            },
        ),
    ]);

    let summary = summarize_providers(providers);

    assert_eq!(
        summary.iter().map(|provider| provider.id.as_str()).collect::<Vec<_>>(),
        vec!["a-config", "z-config"]
    );
    assert!(summary.iter().all(|provider| provider.source_label == "配置"));
    assert!(summary.iter().all(|provider| provider.connected));
}

#[test]
fn summarize_models_projects_active_capabilities_limits_and_detail() {
    let model = Model {
        id: "vision-model".to_string(),
        provider_id: "provider".to_string(),
        api: ApiInfo {
            id: "vision-model".to_string(),
            url: "https://api.example.com/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: "Vision Model".to_string(),
        family: None,
        capabilities: Capabilities {
            temperature: true,
            reasoning: true,
            attachment: false,
            toolcall: false,
            input: CapabilityIO { text: true, audio: false, image: true, video: false, pdf: true },
            output: CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: InterleavedCapability::Bool(true),
        },
        cost: ModelCost {
            input: 1.0,
            output: 2.0,
            cache: ModelCostCache { read: 0.1, write: 0.2 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 64_000, input: Some(48_000), output: 8_000 },
        status: "active".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    };
    let providers = HashMap::from([(
        "provider".to_string(),
        Info {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            source: ProviderSource::Api,
            env: vec![],
            key: None,
            options: HashMap::new(),
            models: HashMap::from([("vision-model".to_string(), model)]),
        },
    )]);

    let summary = summarize_models(providers);
    let model = &summary[0].models[0];

    assert_eq!(model.name, "Vision Model");
    assert!(model.enabled);
    assert!(!model.toolcall);
    assert!(model.attachment);
    assert_eq!(model.context_limit, 64_000);
    assert_eq!(model.detail["release_date"], "2026-01-01");
}

#[test]
fn refresh_marks_loading_and_clears_stale_error_before_task_runs() {
    let mut app = test_app();
    app.agents_settings.loading = false;
    app.agents_settings.project_path = Some("/stale-project".to_string());
    app.agents_settings.save_error = Some("stale error".to_string());

    let _task = update(&mut app, SettingsMessage::Agents(AgentsMessage::Refresh));

    assert!(app.agents_settings.loading);
    assert!(!app.agents_settings.config_ready);
    assert!(app.agents_settings.registered_tools.is_none());
    assert!(app.agents_settings.project_path.is_none());
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn select_agent_on_system_prompt_updates_selection_without_loading_file() {
    let mut app = test_app();
    app.agents_settings.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.to_string();
    app.agents_settings.save_error = Some("stale error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SelectAgent("researcher".to_string())),
    );

    assert_eq!(app.agents_settings.selected_agent, "researcher");
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn loaded_success_preserves_nonempty_tool_inventory_when_refresh_returns_empty() {
    let mut app = test_app();
    app.agents_settings.loading = true;
    app.agents_settings.available_tools = vec!["read".to_string()];
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((vec![], vec![], vec![])))),
    );

    assert!(app.agents_settings.loading);
    assert_eq!(app.agents_settings.available_tools, vec!["read".to_string()]);
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn loaded_error_preserves_existing_data_while_config_load_is_pending() {
    let mut app = test_app();
    app.agents_settings.loading = true;
    app.agents_settings.available_tools = vec!["read".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Err("provider unavailable".to_string()))),
    );

    assert!(app.agents_settings.loading);
    assert_eq!(app.agents_settings.available_tools, vec!["read".to_string()]);
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("provider unavailable"));
}

#[test]
fn add_agent_key_changed_replaces_input_and_clears_error() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AddAgentKeyChanged("worker_2".to_string())),
    );

    assert_eq!(app.agents_settings.new_agent_key_input, "worker_2");
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn add_agent_requested_rejects_empty_normalized_key() {
    let mut app = test_app();
    let original_entries = app.agents_settings.entries.len();
    app.agents_settings.new_agent_key_input = " ! 中文 ".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(app.agents_settings.entries.len(), original_entries);
    assert_eq!(
        app.agents_settings.save_error.as_deref(),
        Some("智能体 key 不能为空，只允许字母、数字、-、_。")
    );
}

#[test]
fn add_agent_requested_rejects_duplicate_normalized_key() {
    let mut app = test_app();
    let original_entries = app.agents_settings.entries.len();
    app.agents_settings.new_agent_key_input = " main! ".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(app.agents_settings.entries.len(), original_entries);
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("智能体 main 已存在。"));
}

#[test]
fn tab_selection_updates_state_without_loading_system_prompt() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

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

    assert_eq!(app.agents_settings.active_detail_tab, "tools");
    assert_eq!(app.agents_settings.active_prompt_tab, AGENT_PROMPT_SYSTEM_TAB);
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn workspace_identity_loaded_ignores_result_for_unselected_agent() {
    let mut app = test_app();
    app.agents_settings.selected_agent = "main".to_string();
    app.agents_settings.workspace_identity_root_path = Some("original-root".to_string());
    app.agents_settings.save_error = Some("original error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
            agent_key: "other".to_string(),
            file_name: "SOUL.md".to_string(),
            workspace_root_path: Some("other-root".to_string()),
            result: Ok(("other content".to_string(), Some(5), Some(6))),
        }),
    );

    assert_eq!(app.agents_settings.workspace_identity_root_path.as_deref(), Some("original-root"));
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("original error"));
    let file = app
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "SOUL.md")
        .expect("missing SOUL.md state");
    assert!(file.editor.text().is_empty());
}

#[test]
fn workspace_identity_loaded_updates_matching_file_and_metadata() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityLoaded {
            agent_key: "main".to_string(),
            file_name: "SOUL.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Ok(("new soul".to_string(), Some(8), Some(13))),
        }),
    );

    assert_eq!(app.agents_settings.workspace_identity_root_path.as_deref(), Some("workspace-main"));
    assert!(app.agents_settings.save_error.is_none());
    let file = app
        .agents_settings
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
        .agents_settings
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

    assert_eq!(app.agents_settings.save_error.as_deref(), Some("read failed"));
    let file = app
        .agents_settings
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
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "AGENTS.md")
        .expect("missing AGENTS.md state");
    assert_eq!(file.editor.text(), "default agents");
    assert_eq!(file.size_bytes, Some(14));
    assert_eq!(file.modified_at_ms, Some(21));
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn workspace_identity_default_restored_ignores_stale_agent_response() {
    let mut app = test_app();
    app.agents_settings.selected_agent = "main".to_string();
    app.agents_settings.workspace_identity_root_path = Some("original-root".to_string());
    app.agents_settings.save_error = Some("original error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key: "other".to_string(),
            file_name: "AGENTS.md".to_string(),
            workspace_root_path: Some("other-root".to_string()),
            result: Ok(("other default".to_string(), Some(13), Some(21))),
        }),
    );

    assert_eq!(app.agents_settings.workspace_identity_root_path.as_deref(), Some("original-root"));
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("original error"));
    let file = app
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "AGENTS.md")
        .expect("missing AGENTS.md state");
    assert!(file.editor.text().is_empty());
}

#[test]
fn workspace_identity_default_restored_records_selected_agent_error() {
    let mut app = test_app();
    app.agents_settings.workspace_identity_root_path = Some("original-root".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityDefaultRestored {
            agent_key: "main".to_string(),
            file_name: "AGENTS.md".to_string(),
            workspace_root_path: Some("workspace-main".to_string()),
            result: Err("restore failed".to_string()),
        }),
    );

    assert_eq!(app.agents_settings.workspace_identity_root_path.as_deref(), Some("workspace-main"));
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("restore failed"));
}

#[test]
fn workspace_identity_saved_updates_metadata_and_clears_error() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

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
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "MEMORY.md")
        .expect("missing MEMORY.md state");
    assert_eq!(file.size_bytes, Some(34));
    assert_eq!(file.modified_at_ms, Some(55));
    assert!(app.agents_settings.save_error.is_none());
}

#[test]
fn workspace_identity_saved_records_error_and_preserves_metadata() {
    let mut app = test_app();
    let file = app
        .agents_settings
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
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "MEMORY.md")
        .expect("missing MEMORY.md state");
    assert_eq!(file.size_bytes, Some(89));
    assert_eq!(file.modified_at_ms, Some(144));
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("write failed"));
}

#[test]
fn agent_core_field_messages_update_entries_and_clamp_numeric_bounds() {
    let mut app = test_app();
    entry_mut(&mut app, "main").provider = "old-provider".to_string();
    entry_mut(&mut app, "main").model = "old-model".to_string();
    entry_mut(&mut app, "main").enabled = true;
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProviderChanged(
            "main".to_string(),
            "new-provider".to_string(),
        )),
    );
    assert_eq!(entry(&app, "main").provider, "new-provider");
    assert!(entry(&app, "main").model.is_empty());
    assert!(app.agents_settings.save_error.is_none());

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
        SettingsMessage::Agents(AgentsMessage::MaxToolIterationsChanged(
            "main".to_string(),
            50_002,
        )),
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
    assert_eq!(main.max_tool_iterations, 50_002);
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
    app.agents_settings.available_tools.clear();

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

    assert_eq!(
        app.agents_settings.available_tools,
        vec!["bash".to_string(), "read".to_string(), "rustcodegraph".to_string()]
    );
    assert!(!app.agents_settings.available_tools.iter().any(|tool| tool.starts_with("mcp__")));
}

#[test]
fn loaded_tool_catalog_removes_all_stale_selections_and_schedules_save() {
    let mut app = test_app();
    entry_mut(&mut app, "main").allowed_tools = vec![
        "removed_a".to_string(),
        "read".to_string(),
        "removed_b".to_string(),
    ];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((
            vec![],
            vec![],
            vec!["read".to_string()],
        )))),
    );

    assert_eq!(
        app.agents_settings.registered_tools.as_deref().expect("tool catalog"),
        ["read".to_string(), "rustcodegraph".to_string()]
    );
    assert_eq!(entry(&app, "main").allowed_tools, ["read"]);
    assert!(app.agents_settings.save_in_flight);
}

#[test]
fn config_loaded_after_tool_catalog_removes_stale_selections_and_schedules_save() {
    let mut app = test_app();
    app.agents_settings.config_ready = false;
    app.agents_settings.available_tools = vec!["read".to_string(), "rustcodegraph".to_string()];
    app.agents_settings.registered_tools = Some(app.agents_settings.available_tools.clone());

    let mut config = vw_config_types::config::Config::default();
    let mut custom = vw_config_types::agent::DelegateAgentConfig::default();
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
    assert!(app.agents_settings.save_in_flight);
}

#[test]
fn allowed_tool_messages_keep_builtin_required_tools() {
    let mut app = test_app();
    app.agents_settings.available_tools = vec![
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
    assert!(inverted.iter().any(|tool| tool == "web_search_tool"));
    assert!(inverted.iter().any(|tool| tool == "write"));
    assert!(!inverted.iter().any(|tool| tool == "read"));

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
    assert!(!preset.iter().any(|tool| tool == "write"));
}

#[test]
fn allowed_tool_messages_sort_select_invert_and_apply_presets() {
    let mut app = test_app();
    app.agents_settings.available_tools = vec![
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
    // 关闭可选工具时仍强制并回 available 中的内置必选。
    assert_eq!(
        entry(&app, "main").allowed_tools,
        vec!["bash".to_string(), "web_search_tool".to_string()]
    );

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsSelectAll("main".to_string())),
    );
    let mut expected_all = app.agents_settings.available_tools.clone();
    expected_all.sort();
    assert_eq!(entry(&app, "main").allowed_tools, expected_all);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolsInvertSelection("main".to_string())),
    );
    // 内置必选（bash / web_search_tool）不可被反选清掉。
    assert_eq!(
        entry(&app, "main").allowed_tools,
        vec!["bash".to_string(), "web_search_tool".to_string()]
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
        vec!["bash".to_string(), "read".to_string(), "web_search_tool".to_string()]
    );
}

#[test]
fn agent_config_save_coalesces_changes_while_request_is_in_flight() {
    let mut app = test_app();
    app.agents_settings.available_tools = vec!["bash".to_string(), "read".to_string()];
    entry_mut(&mut app, "main").allowed_tools.clear();

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "read".to_string(),
            true,
        )),
    );
    assert!(app.agents_settings.save_in_flight);
    assert!(!app.agents_settings.save_pending);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedToolToggled(
            "main".to_string(),
            "bash".to_string(),
            true,
        )),
    );
    assert!(app.agents_settings.save_in_flight);
    assert!(app.agents_settings.save_pending);

    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(app.agents_settings.save_in_flight);
    assert!(!app.agents_settings.save_pending);
    assert_eq!(entry(&app, "main").allowed_tools, vec!["bash", "read"]);

    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(!app.agents_settings.save_in_flight);
    assert!(!app.agents_settings.save_pending);
}

#[test]
fn add_agent_defaults_to_all_loaded_tools() {
    let mut app = test_app();
    app.agents_settings.available_tools = vec!["bash".to_string(), "read".to_string()];
    app.agents_settings.new_agent_key_input = "worker_2".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(entry(&app, "worker_2").allowed_tools, vec!["bash", "read"]);
}

#[test]
fn allowed_skill_messages_filter_by_scope_and_toggle_membership() {
    let mut app = test_app();
    app.skills_settings.catalog = vec![
        skill_item("project-a", "workspace", true),
        skill_item("project-disabled", "workspace", false),
        skill_item("global-a", "global", true),
        skill_item("bundled-a", "bundled", true),
    ];
    entry_mut(&mut app, "main").allowed_skills = vec!["global-a".to_string()];

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsSelectAll(
            "main".to_string(),
            SkillsDirectoryScope::Project,
        )),
    );
    assert_eq!(entry(&app, "main").allowed_skills, vec!["global-a", "project-a"]);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsInvertSelection(
            "main".to_string(),
            SkillsDirectoryScope::All,
        )),
    );
    assert_eq!(entry(&app, "main").allowed_skills, vec!["bundled-a"]);

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
    assert_eq!(entry(&app, "main").allowed_skills, vec!["project-a"]);
}

#[test]
fn editor_actions_persist_only_for_text_edits_and_leave_unknown_files_unchanged() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("keep".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SystemPromptAction(
            "main".to_string(),
            text_editor::Action::Move(text_editor::Motion::DocumentEnd),
        )),
    );
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("keep"));

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
    assert!(app.agents_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEditorAction(
            "USER.md".to_string(),
            text_editor::Action::Edit(text_editor::Edit::Insert('u')),
        )),
    );
    let user_file = app
        .agents_settings
        .workspace_identity_files
        .iter()
        .find(|file| file.file_name == "USER.md")
        .expect("missing USER.md state");
    assert_eq!(user_file.editor.text(), "u");

    app.agents_settings.save_error = Some("unknown remains".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::WorkspaceIdentityEditorAction(
            "UNKNOWN.md".to_string(),
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        )),
    );
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("unknown remains"));
}
