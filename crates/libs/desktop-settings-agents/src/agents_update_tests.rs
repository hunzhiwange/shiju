use super::*;
use std::collections::HashMap;
use model_types::{ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost, ModelCostCache, ModelLimit, ProviderSource};
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
            protocols: None,
            id: "gpt-4.1".to_string(),
            url: "https://api.example.com/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: "GPT-4.1".to_string(),
        family: None,
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
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
            protocols: None,
            id: "vision-model".to_string(),
            url: "https://api.example.com/v1".to_string(),
            adapter: "openai-compatible".to_string(),
        },
        name: "Vision Model".to_string(),
        family: None,
        capabilities: Capabilities {
            native_tool_search: false,
            catalog_execution: None,
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
