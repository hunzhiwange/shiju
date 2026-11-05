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
fn project_config_loaded_does_not_replace_global_agents() {
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

    assert!(entry(&app, "main").allowed_skills.is_empty());
    assert!(app.agents_settings.loading);
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
fn project_changed_preserves_global_config_readiness() {
    let mut app = test_app();
    app.agents_settings.save_error = Some("stale error".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProjectChanged("/project-a".to_string())),
    );

    assert!(app.agents_settings.project_path.is_none());
    assert!(!app.agents_settings.loading);
    assert!(app.agents_settings.config_ready);
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("stale error"));
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
    main.max_history_messages = 456;
    main.max_iterations = 50_002;
    main.parallel_tools = true;
    main.allow_as_subagent = false;
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
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ForceModelSelectionToggled("main".into(), true)),
    );
    assert!(!app.agents_settings.profile_modal.as_ref().unwrap().force_model_selection);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProfileModelChanged(
            vw_config_types::agent::AgentDefault::Fixed(
                vw_config_types::agent::AgentModelReference {
                    provider: "provider".into(),
                    model: "model".into(),
                },
            ),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ForceModelSelectionToggled("main".into(), true)),
    );
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert!(draft.force_model_selection);
    let patch = build_agents_settings_patch(std::slice::from_ref(draft)).unwrap();
    assert_eq!(patch["agents"]["main"]["force_model_selection"], true);
    assert_eq!(patch["agents"]["main"]["provider"], "provider");
    assert_eq!(patch["agents"]["main"]["model"], "model");
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
    assert!(app.agents_settings.dirty);
    assert!(!app.agents_settings.save_in_flight);
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

    let patch =
        build_agents_settings_patch(&app.agents_settings.entries).expect("patch should serialize");
    assert!(patch["agents"]["main"].get("tool_policy").is_none());
    assert_eq!(patch["agents"]["main"]["skill_policy"], "strict");
    assert_eq!(patch["agents"]["main"]["allowed_tools"], serde_json::json!(["read"]));
    assert_eq!(patch["agents"]["main"]["allowed_skills"], serde_json::json!(["skill-a"]));

    let reloaded_config: vw_config_types::agent::DelegateAgentConfig =
        serde_json::from_value(patch["agents"]["main"].clone())
            .expect("main agent patch should deserialize");
    let reloaded_main =
        crate::app::state::DelegateAgentSettingsEntry::from_config("main", Some(reloaded_config));
    assert_eq!(reloaded_main.skill_policy, DelegateCapabilityPolicy::Strict);
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
            reasoning_options: None,
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
    assert!(summary[0].connected);
    assert!(!summary[1].connected, "配置存在但没有密钥，不应显示为已连接");
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
            reasoning_options: None,
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

    assert_eq!(app.agents_settings.available_tools, vec!["bash".to_string(), "read".to_string()]);
    assert!(!app.agents_settings.available_tools.iter().any(|tool| tool.starts_with("mcp__")));
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

    assert_eq!(
        app.agents_settings.registered_tools.as_deref().expect("tool catalog"),
        ["read".to_string()]
    );
    assert_eq!(entry(&app, "main").allowed_tools, ["read"]);
    assert!(!app.agents_settings.dirty);
    assert!(!app.agents_settings.save_in_flight);
}

#[test]
fn config_loaded_after_tool_catalog_removes_stale_selections_without_creating_user_edits() {
    let mut app = test_app();
    app.agents_settings.config_ready = false;
    app.agents_settings.available_tools = vec!["read".to_string()];
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
    assert!(!app.agents_settings.dirty);
    assert!(!app.agents_settings.save_in_flight);
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
fn agent_config_saves_only_on_click_and_retains_edits_during_save() {
    let mut app = test_app();
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "first".into())),
    );
    assert!(app.agents_settings.dirty);
    assert!(!app.agents_settings.save_in_flight);

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.agents_settings.save_in_flight);
    assert!(!app.agents_settings.dirty);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "second".into())),
    );
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.agents_settings.dirty, "duplicate save must not consume later edits");
    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(!app.agents_settings.save_in_flight);
    assert!(app.agents_settings.dirty);
    assert_eq!(entry(&app, "main").model, "second");

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    let _ = handle_agents_config_saved(&mut app, Ok(()));
    assert!(!app.agents_settings.save_in_flight);
    assert!(!app.agents_settings.dirty);
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
    assert!(app.agents_settings.dirty);
    assert!(!app.agents_settings.save_in_flight);
    assert!(app.agents_settings.save_error.as_deref().unwrap().contains("offline"));
    assert_eq!(entry(&app, "main").model, "saved-later");
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
    assert!(app.agents_settings.save_in_flight);
    assert!(app.agents_settings.save_error.is_none());
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
    assert!(app.agents_settings.dirty);
    assert!(app.agents_settings.project_path.is_none());
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::DiscardChanges));
    assert!(!app.agents_settings.dirty);
    assert!(!app.agents_settings.config_ready);
    assert!(app.agents_settings.loading);
}

#[test]
fn add_agent_defaults_to_all_loaded_tools() {
    let mut app = test_app();
    app.agents_settings.available_tools = vec!["bash".to_string(), "read".to_string()];
    app.agents_settings.new_agent_key_input = "worker_2".to_string();

    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));

    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().allowed_tools,
        vec!["bash", "read"]
    );
}

#[test]
fn allowed_skill_messages_filter_by_scope_and_toggle_membership() {
    let mut app = test_app();
    app.agents_settings.skill_catalog = vec![
        skill_item("project-a", "workspace", true),
        skill_item("project-disabled", "workspace", false),
        skill_item("global-a", "global", true),
        skill_item("bundled-a", "bundled", true),
    ];
    entry_mut(&mut app, "main").defaults.skills =
        vw_config_types::agent::AgentDefault::Fixed(vec!["global-a".into()]);

    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsSelectAll(
            "main".to_string(),
            SkillsDirectoryScope::Project,
        )),
    );
    assert_eq!(
        entry(&app, "main").defaults.skills,
        vw_config_types::agent::AgentDefault::Fixed(vec!["global-a".into(), "project-a".into()])
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
        vw_config_types::agent::AgentDefault::Fixed(vec!["bundled-a".into()])
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
        vw_config_types::agent::AgentDefault::Fixed(vec!["project-a".into()])
    );
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

#[test]
fn successful_catalog_refresh_preserves_unsaved_config_error() {
    let mut app = test_app();
    app.agents_settings.dirty = true;
    app.agents_settings.save_error = Some("save failed".into());
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::Loaded(Ok((vec![], vec![], vec![])))),
    );
    assert_eq!(app.agents_settings.save_error.as_deref(), Some("save failed"));
    assert!(app.agents_settings.dirty);
}

#[test]
fn saved_config_stays_clean_after_discard_refresh_in_either_response_order() {
    for project_path in [None, Some("/project-a".to_string())] {
        for catalog_first in [false, true] {
            let mut app = test_app();
            app.project_path = project_path.clone();
            app.agents_settings.project_path = project_path.clone();
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::ModelChanged(
                    "main".into(),
                    "saved-model".into(),
                )),
            );
            let patch = build_agents_settings_patch(&app.agents_settings.entries).unwrap();
            let mut saved_config = vw_config_types::config::Config::default();
            for (key, value) in patch["agents"].as_object().unwrap() {
                if !value.is_null() {
                    saved_config
                        .agents
                        .insert(key.clone(), serde_json::from_value(value.clone()).unwrap());
                }
            }
            let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
            let _ = handle_agents_config_saved(&mut app, Ok(()));
            assert!(!app.agents_settings.dirty);

            for _ in 0..2 {
                let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::DiscardChanges));
                let catalog = AgentsMessage::Loaded(Ok((vec![], vec![], vec!["read".into()])));
                let config = AgentsMessage::ConfigLoaded {
                    project_path: None,
                    result: Ok(Box::new(saved_config.clone())),
                };
                let responses = if catalog_first { [catalog, config] } else { [config, catalog] };
                for response in responses {
                    let _ = update(&mut app, SettingsMessage::Agents(response));
                    assert!(!app.agents_settings.dirty);
                    assert!(!app.agents_settings.save_in_flight);
                }
                assert!(app.agents_settings.config_ready);
                assert_eq!(entry(&app, "main").model, "saved-model");
                assert!(entry(&app, "main").allowed_tools.iter().all(|tool| tool == "read"));
            }
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "draft".into())),
            );
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::Loaded(Ok((
                    vec![],
                    vec![],
                    vec!["read".into()],
                )))),
            );
            assert!(app.agents_settings.dirty, "catalog refresh must preserve actual edits");
        }
    }
}

#[test]
fn agent_unified_selection_catalog_launch_selects_profile_runtime() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.catalog_busy = true;
    app.agents_settings.entries.push(crate::app::state::DelegateAgentSettingsEntry::from_config(
        "acp-codex",
        Some(vw_config_types::agent::AgentDefinitionConfig {
            runtime: vw_config_types::agent::AgentRuntime::Acp { backend: "codex".into() },
            ..Default::default()
        }),
    ));
    let info = serde_json::from_value(serde_json::json!({
        "id":"catalog-launch-session", "slug":"fixture", "projectID":"fixture-project", "directory":"/tmp/unified-agent-fixture", "title":"New session", "version":"1", "time":{"created":1,"updated":1}
    })).unwrap();
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::CatalogSessionCreated {
            key: "acp-codex".into(),
            result: Ok(info),
        }),
    );
    assert_eq!(app.active_session_id.as_deref(), Some("catalog-launch-session"));
    assert_eq!(app.current_session_runtime().agent.as_deref(), Some("acp-codex"));
    assert_eq!(app.current_session_runtime().acp_agent.as_deref(), Some("codex"));
}

#[test]
fn agent_config_cleanup_skill_directory_preserves_global_draft() {
    let mut app = test_app();
    app.project_path = Some("/workspace".into());
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProfileNameChanged("保留草稿".into())),
    );
    for path in ["/directory-a", "/directory-b"] {
        let _ =
            update(&mut app, SettingsMessage::Agents(AgentsMessage::ProjectSelected(path.into())));
        assert_eq!(app.project_path.as_deref(), Some("/workspace"));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, "保留草稿");
        assert!(app.agents_settings.config_ready);
    }
}

static AGENT_CONFIG_CLEANUP_HTTP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct AgentConfigCleanupGateway {
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    requests: Arc<std::sync::Mutex<Vec<(String, String, Value)>>>,
    server: tokio::task::JoinHandle<()>,
}

impl AgentConfigCleanupGateway {
    async fn start() -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let directory = tempfile::tempdir().unwrap();
        let mut config = vw_config_types::ui::GatewayClientSystemSettingsConfig::default();
        let mut server = config.active_server().clone();
        server.host = "127.0.0.1".into();
        server.port = port;
        config.set_servers(vec![server.clone()], server.id);
        std::fs::write(
            directory.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let previous = std::env::var_os("VIBEWINDOW_CONFIG_DIR");
        unsafe {
            std::env::set_var("VIBEWINDOW_CONFIG_DIR", directory.path());
        }
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let (header_end, length) = loop {
                        let mut buffer = [0; 4096];
                        let count = stream.read(&mut buffer).await.unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header = String::from_utf8_lossy(&bytes[..index]);
                            let length = header
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap_or(0);
                            break (index + 4, length);
                        }
                    };
                    while bytes.len() < header_end + length {
                        let mut buffer = [0; 4096];
                        let count = stream.read(&mut buffer).await.unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    let header = String::from_utf8_lossy(&bytes[..header_end]);
                    let mut parts = header.split_whitespace();
                    let method = parts.next().unwrap().to_string();
                    let path = parts.next().unwrap().to_string();
                    let body = if length == 0 {
                        Value::Null
                    } else {
                        serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap()
                    };
                    recorded.lock().unwrap().push((method.clone(), path.clone(), body));
                    if path.contains("directory-a") {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    let body = if path.starts_with("/v1/skills") {
                        let id = if path.contains("directory-a") { "skill-a" } else { "skill-b" };
                        serde_json::json!([{"id":id,"title":"中文技能", "description":"目录候选", "kind":"personal", "resource_count":0,"installed":true,"enabled":true,"source":"workspace","source_path":null}])
                    } else if path == "/v1/tools" {
                        serde_json::json!({"items": [
                            {"id":"file_read", "identity":{"namespace":null,"name":"file_read"}, "source":"builtin", "exposure":"direct", "display_name":"Read", "description":"Read a file", "input_schema":{"type":"object"}}
                        ]})
                    } else if path.contains("/copy") {
                        {
                            let mut value = serde_json::to_value(
                                vw_config_types::agent::DelegateAgentConfig::default(),
                            )
                            .unwrap();
                            value["key"] = "copy-agent".into();
                            value["label"] = "复制智能体".into();
                            value
                        }
                    } else if path.starts_with("/v1/agent") {
                        serde_json::json!([])
                    } else {
                        serde_json::json!({})
                    };
                    let body = body.to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    stream.write_all(response.as_bytes()).await.unwrap();
                });
            }
        });
        Self { _directory: directory, previous, requests, server }
    }
}

impl Drop for AgentConfigCleanupGateway {
    fn drop(&mut self) {
        self.server.abort();
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var("VIBEWINDOW_CONFIG_DIR", previous);
            } else {
                std::env::remove_var("VIBEWINDOW_CONFIG_DIR");
            }
        }
    }
}

async fn agent_config_cleanup_messages(
    task: iced::Task<crate::app::Message>,
) -> Vec<crate::app::Message> {
    use iced::futures::StreamExt;
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return vec![];
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    vec![]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_global_mutations_do_not_use_skill_directory() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.project_path = Some("/workspace".into());
    app.agents_settings.skill_directory = Some("/directory-b".into());
    let mut custom = DelegateAgentSettingsEntry::from_config("custom-agent", None);
    custom.label = "智能体".into();
    app.agents_settings.entries.push(custom);
    for message in [
        AgentsMessage::CatalogToggle("custom-agent".into(), false),
        AgentsMessage::CatalogCopy("main".into()),
    ] {
        let task = update(&mut app, SettingsMessage::Agents(message));
        for result in agent_config_cleanup_messages(task).await {
            if let crate::app::Message::Settings(message) = result {
                let _ = update(&mut app, message);
            }
        }
        assert!(!app.agents_settings.catalog_busy);
    }
    assert!(app.agents_settings.entries.iter().any(|entry| entry.key == "copy-agent"));
    for key in ["new-agent", "custom-agent", "copy-agent"] {
        if key == "new-agent" {
            app.agents_settings.new_agent_key_input = key.into();
            let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));
        } else {
            let _ =
                update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen(key.into())));
        }
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::ProfileNameChanged("保存草稿".into())),
        );
        for message in [AgentsMessage::ProfileSave, AgentsMessage::ProfileDelete] {
            let task = update(&mut app, SettingsMessage::Agents(message));
            assert!(app.agents_settings.profile_saving);
            for result in agent_config_cleanup_messages(task).await {
                if let crate::app::Message::Settings(message) = result {
                    let _ = update(&mut app, message);
                }
            }
            // Reopen the saved draft to exercise deletion through the same transaction.
            if app.agents_settings.profile_modal.is_none() {
                app.agents_settings.profile_modal =
                    app.agents_settings.entries.iter().find(|e| e.key == key).cloned();
            }
        }
    }
    let recorded = fixture.requests.lock().unwrap();
    let requests = recorded
        .iter()
        .filter(|(method, _, _)| method == "PATCH" || method == "POST")
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 8, "create/edit/delete/toggle/copy must all reach isolated HTTP");
    assert!(
        requests
            .iter()
            .all(|(_, path, _)| !path.contains("directory") && !path.contains("project_path"))
    );
    assert_eq!(
        requests
            .iter()
            .filter(|(method, path, _)| method == "PATCH" && path == "/v1/global/config")
            .count(),
        7
    );
    assert!(
        requests.iter().any(|(method, path, _)| method == "POST" && path == "/v1/agent/main/copy")
    );
    assert!(requests.iter().any(|(_, _, body)| body["agents"]["new-agent"].is_object()));
    assert!(
        requests
            .iter()
            .any(|(_, _, body)| body["agents"].get("custom-agent").is_some_and(Value::is_null))
    );
    assert_eq!(app.project_path.as_deref(), Some("/workspace"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_stale_skill_catalog_is_ignored() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.skills_settings.catalog = vec![skill_item("outer-library", "global", true)];
    app.skills_settings.loading = true;
    let task_a = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillDirectorySelected(Some("/directory-a".into()))),
    );
    let task_b = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillDirectorySelected(Some("/directory-b".into()))),
    );
    let (a, b) =
        tokio::join!(agent_config_cleanup_messages(task_a), agent_config_cleanup_messages(task_b));
    for result in b.into_iter().chain(a) {
        if let crate::app::Message::Settings(message) = result {
            let _ = update(&mut app, message);
        }
    }
    assert_eq!(app.agents_settings.skill_catalog[0].id, "skill-b");
    assert!(!app.agents_settings.skill_catalog_loading);
    assert_eq!(app.skills_settings.catalog[0].id, "outer-library");
    assert!(app.skills_settings.loading);
    assert_eq!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, path, _)| path.starts_with("/v1/skills"))
            .count(),
        2
    );
    // Returning to the same directory still rejects a response from an earlier generation.
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::SkillCatalogRefresh));
    let generation = app.agents_settings.skill_catalog_generation;
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillCatalogLoaded {
            directory: Some("/directory-b".into()),
            generation: generation - 1,
            result: Err("旧请求错误".into()),
        }),
    );
    assert!(app.agents_settings.skill_catalog_error.is_none());
    assert!(app.agents_settings.skill_catalog_loading);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_profile_http_roundtrips_multiline_and_runtime() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.agents_settings.entries.push(DelegateAgentSettingsEntry::from_config("textarea", None));
    for message in [
        AgentsMessage::ProfileOpen("textarea".into()),
        AgentsMessage::ProfileNameChanged("多行智能体".into()),
        AgentsMessage::TemperatureChanged("textarea".into(), 1.2),
        AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::SelectAll),
        AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Paste(Arc::new("中文描述\n第二行\n".into())),
        )),
    ] {
        let _ = update(&mut app, SettingsMessage::Agents(message));
    }
    let task = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileSave));
    assert!(app.agents_settings.profile_saving);
    for result in agent_config_cleanup_messages(task).await {
        if let crate::app::Message::Settings(message) = result {
            let _ = update(&mut app, message);
        }
    }
    assert!(app.agents_settings.profile_modal.is_none());
    let mut payload = {
        let requests = fixture.requests.lock().unwrap();
        let (_, path, body) = requests.iter().find(|(method, _, _)| method == "PATCH").unwrap();
        assert_eq!(path, "/v1/global/config");
        body["agents"]["textarea"].clone()
    };
    assert!(payload.get("description_editor").is_none());
    // The HTTP fixture captures a Merge Patch: null deletes the ACP backend field.
    assert!(payload["runtime"]["backend"].is_null());
    payload["runtime"].as_object_mut().unwrap().remove("backend");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    std::fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
    let config = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let restored = DelegateAgentSettingsEntry::from_config("textarea", Some(config));
    assert_eq!(restored.label, "多行智能体");
    assert_eq!(restored.description, "中文描述\n第二行\n");
    assert_eq!(restored.description_editor.text(), restored.description);
    assert_eq!(restored.temperature, 1.2);
}

#[test]
fn agent_config_cleanup_skill_selection_writes_only_one_default() {
    use vw_config_types::agent::AgentDefault;
    let mut app = test_app();
    app.agents_settings.skill_catalog = vec![skill_item("allowed", "global", true)];
    let agent = entry_mut(&mut app, "main");
    agent.allowed_skills = vec!["allowed".into()];
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled(
            "main".into(),
            "allowed".into(),
            true,
        )),
    );
    let agent = entry(&app, "main");
    assert_eq!(agent.allowed_skills, vec!["allowed"]);
    assert_eq!(agent.defaults.skills, AgentDefault::Fixed(vec!["allowed".into()]));
    let patch = crate::app::components::system_settings_agents::defaults::patch(&agent.defaults);
    assert_eq!(patch["skills"]["value"], serde_json::json!(["allowed"]));
}

#[test]
fn agent_config_cleanup_skill_selection_preserves_inherit_and_rejects_unavailable() {
    use vw_config_types::agent::AgentDefault;
    let mut app = test_app();
    app.agents_settings.skill_catalog = vec![
        skill_item("allowed", "global", true),
        skill_item("outside", "global", true),
        skill_item("disabled", "global", false),
    ];
    let agent = entry_mut(&mut app, "main");
    agent.allowed_skills = vec!["allowed".into()];
    agent.skill_policy = DelegateCapabilityPolicy::Strict;
    agent.defaults.skills = AgentDefault::Inherit;
    let toggle = |id: &str| {
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled("main".into(), id.into(), true))
    };
    let _ = update(&mut app, toggle("allowed"));
    assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Inherit);
    entry_mut(&mut app, "main").defaults.skills = AgentDefault::Fixed(vec![]);
    for id in ["outside", "disabled", "unknown"] {
        let _ = update(&mut app, toggle(id));
        assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Fixed(vec![]));
        assert!(app.active_toast.is_some());
    }
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsSelectAll(
            "main".into(),
            SkillsDirectoryScope::All,
        )),
    );
    assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Fixed(vec!["allowed".into()]));
    assert_eq!(entry(&app, "main").allowed_skills, vec!["allowed"]);
}

#[test]
fn session_entry_catalog_launch_preserves_profile_runtime() {
    use vw_config_types::agent::{AgentDefinitionConfig, AgentRuntime};
    for runtime in [AgentRuntime::Native, AgentRuntime::Acp { backend: "codex".into() }] {
        let mut app = test_app();
        app.show_settings = true;
        app.show_system_settings = true;
        app.screen = crate::app::Screen::Usage;
        app.active_session_id = Some("catalog-old-session".into());
        app.current_session_runtime_mut().agent = Some("old-profile".into());
        let old_id = app.active_session_id.clone();
        let old_runtime = app.current_session_runtime().clone();
        app.agents_settings.entries.push(
            crate::app::state::DelegateAgentSettingsEntry::from_config(
                "fixture-profile",
                Some(AgentDefinitionConfig { runtime: runtime.clone(), ..Default::default() }),
            ),
        );
        app.project_path = None;
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::CatalogLaunch("fixture-profile".into())),
        );
        assert!(!app.agents_settings.catalog_busy);
        assert!(app.active_toast.as_ref().unwrap().message.contains("请先打开项目"));
        app.agents_settings.catalog_busy = true;
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::CatalogSessionCreated {
                key: "fixture-profile".into(),
                result: Err("fixture failure".into()),
            }),
        );
        assert!(app.active_toast.as_ref().unwrap().message.contains("fixture failure"));
        assert!(app.show_settings);
        assert!(app.show_system_settings);
        assert_eq!(app.screen, crate::app::Screen::Usage);
        assert_eq!(app.active_session_id, old_id);
        assert_eq!(app.current_session_runtime().agent, old_runtime.agent);
        app.agents_settings.catalog_busy = true;
        let info = serde_json::from_value(serde_json::json!({"id":"catalog-fixture-new", "slug":"fixture", "projectID":"fixture-project", "directory":"/tmp/catalog-fixture", "title":"New session", "version":"1", "time":{"created":1,"updated":1}})).unwrap();
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::CatalogSessionCreated {
                key: "fixture-profile".into(),
                result: Ok(info),
            }),
        );
        assert_eq!(app.active_session_id.as_deref(), Some("catalog-fixture-new"));
        assert_eq!(app.current_session_runtime().agent.as_deref(), Some("fixture-profile"));
        assert_eq!(
            app.current_session_runtime().acp_agent.as_deref(),
            match runtime {
                AgentRuntime::Native => None,
                AgentRuntime::Acp { .. } => Some("codex"),
            }
        );
        assert!(!app.show_settings);
        assert!(!app.show_system_settings);
        assert_eq!(app.screen, crate::app::Screen::Project);
        assert_eq!(
            app.session_runtime_states.get("catalog-old-session").unwrap().agent,
            old_runtime.agent
        );
        assert!(app.input_text.is_empty());
        assert!(!app.agents_settings.catalog_busy);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_tool_catalog_async_survives_provider_failure() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.agents_settings.available_tools.clear();
    let messages = agent_config_cleanup_messages(super::refresh_task()).await;
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Settings(SettingsMessage::Agents(AgentsMessage::Loaded(Err(_))))
    )));
    for message in messages {
        if let crate::app::Message::Settings(message) = message {
            let _ = update(&mut app, message);
        }
    }
    assert_eq!(app.agents_settings.available_tools, vec!["file_read"]);
    assert!(fixture.requests.lock().unwrap().iter().any(|(_, path, _)| path == "/v1/tools"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_tool_catalog_async_retries_on_profile_open() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let _fixture = AgentConfigCleanupGateway::start().await;
    app.agents_settings.available_tools.clear();
    let task = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    for message in agent_config_cleanup_messages(task).await {
        if let crate::app::Message::Settings(message) = message {
            let _ = update(&mut app, message);
        }
    }
    assert!(app.agents_settings.profile_modal.is_some());
    assert_eq!(app.agents_settings.available_tools, vec!["file_read"]);
}
