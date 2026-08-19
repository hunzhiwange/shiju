use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{
    AutonomyConfig, ClassificationRule, ModelRouteConfig, QueryClassificationConfig,
    ReliabilityConfig,
};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::tools::Tool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn route(provider: &str, model: &str, task_mode_enabled: bool) -> ChannelRouteSelection {
    ChannelRouteSelection {
        provider: provider.to_string(),
        model: model.to_string(),
        task_mode_enabled,
    }
}

fn context(workspace_dir: &Path) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new("default-provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::<Box<dyn Tool>>::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("default-model".to_string()),
        temperature: 0.2,
        auto_save_memory: false,
        max_iterations: 4,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(ReliabilityConfig::default()),
        provider_runtime_options: ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(workspace_dir)),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

#[test]
fn resolve_provider_alias_normalizes_known_provider_id_case() {
    let ids = available_provider_ids();
    if let Some(first) = ids.first() {
        assert_eq!(resolve_provider_alias(&first.to_ascii_uppercase()), Some(first.clone()));
    }
    assert_eq!(resolve_provider_alias("unknown-provider"), None);
}

#[test]
fn resolve_provider_alias_rejects_blank_input() {
    assert_eq!(resolve_provider_alias("   "), None);
}

#[test]
fn available_provider_ids_is_sorted() {
    let ids = available_provider_ids();
    let mut sorted = ids.clone();
    sorted.sort();

    assert_eq!(ids, sorted);
}

#[test]
fn route_selection_defaults_to_runtime_defaults_and_removes_default_override() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path());

    assert_eq!(default_route_selection(&ctx), route("default-provider", "default-model", false));
    assert_eq!(
        get_route_selection(&ctx, "sender"),
        route("default-provider", "default-model", false)
    );

    set_route_selection(&ctx, "sender", route("other-provider", "other-model", true));
    assert_eq!(get_route_selection(&ctx, "sender"), route("other-provider", "other-model", true));

    set_route_selection(&ctx, "sender", route("default-provider", "default-model", false));
    assert_eq!(
        get_route_selection(&ctx, "sender"),
        route("default-provider", "default-model", false)
    );
    assert!(!ctx.route_overrides.lock().unwrap().contains_key("sender"));
}

#[test]
fn route_selection_recovers_from_poisoned_override_lock() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path());
    let overrides = Arc::clone(&ctx.route_overrides);
    let default_route = route("default-provider", "default-model", false);
    let custom_route = route("other-provider", "other-model", true);

    let _ = std::thread::spawn(move || {
        let _guard = overrides.lock().unwrap();
        panic!("poison route overrides");
    })
    .join();

    assert!(ctx.route_overrides.is_poisoned());
    assert_eq!(get_route_selection(&ctx, "sender"), default_route);

    set_route_selection(&ctx, "sender", custom_route.clone());
    assert_eq!(get_route_selection(&ctx, "sender"), custom_route);

    set_route_selection(&ctx, "sender", default_route.clone());
    assert_eq!(get_route_selection(&ctx, "sender"), default_route);
    assert!(!ctx.route_overrides.lock().unwrap_or_else(|e| e.into_inner()).contains_key("sender"));
}

#[test]
fn classify_message_route_returns_matching_model_route() {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = context(temp.path());
    ctx.query_classification = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "code".to_string(),
            keywords: vec!["rust".to_string()],
            priority: 5,
            ..Default::default()
        }],
    };
    ctx.model_routes = vec![ModelRouteConfig {
        hint: "code".to_string(),
        provider: "code-provider".to_string(),
        model: "code-model".to_string(),
        max_tokens: None,
        api_key: None,
    }];

    assert_eq!(
        classify_message_route(&ctx, "Please write Rust code"),
        Some(route("code-provider", "code-model", false))
    );
    assert_eq!(classify_message_route(&ctx, "hello"), None);
}

#[test]
fn classify_message_route_returns_none_when_hint_has_no_model_route() {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = context(temp.path());
    ctx.query_classification = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "missing".to_string(),
            keywords: vec!["route-me".to_string()],
            ..Default::default()
        }],
    };

    assert_eq!(classify_message_route(&ctx, "route-me"), None);
}

#[test]
fn classify_message_route_returns_none_when_classification_is_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = context(temp.path());
    ctx.model_routes = vec![ModelRouteConfig {
        hint: "code".to_string(),
        provider: "code-provider".to_string(),
        model: "code-model".to_string(),
        max_tokens: None,
        api_key: None,
    }];

    assert!(!ctx.query_classification.enabled);
    assert_eq!(classify_message_route(&ctx, "Please write Rust code"), None);
}

#[test]
fn load_cached_model_preview_handles_missing_invalid_and_limited_cache() {
    let temp = tempfile::tempdir().unwrap();

    assert!(load_cached_model_preview(temp.path(), "openai").is_empty());

    let state_dir = temp.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join(MODEL_CACHE_FILE), "not json").unwrap();
    assert!(load_cached_model_preview(temp.path(), "openai").is_empty());

    let models = (0..12).map(|idx| format!("model-{idx}")).collect::<Vec<_>>();
    std::fs::write(
        state_dir.join(MODEL_CACHE_FILE),
        serde_json::json!({
            "entries": [
                {"provider": "other", "models": ["ignored"]},
                {"provider": "openai", "models": models}
            ]
        })
        .to_string(),
    )
    .unwrap();

    let preview = load_cached_model_preview(temp.path(), "openai");
    assert_eq!(preview.len(), MODEL_CACHE_PREVIEW_LIMIT);
    assert_eq!(preview.first().map(String::as_str), Some("model-0"));
    assert_eq!(preview.last().map(String::as_str), Some("model-9"));
    assert!(load_cached_model_preview(temp.path(), "missing").is_empty());
}

#[test]
fn build_models_help_response_includes_cached_models_or_refresh_hint() {
    let temp = tempfile::tempdir().unwrap();
    let current = route("openai", "gpt-5", false);

    let no_cache = build_models_help_response(&current, temp.path());
    assert!(no_cache.contains("Current provider: `openai`"));
    assert!(no_cache.contains("No cached model list found"));

    let state_dir = temp.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        state_dir.join(MODEL_CACHE_FILE),
        serde_json::json!({
            "entries": [{"provider": "openai", "models": ["gpt-5", "gpt-5-mini"]}]
        })
        .to_string(),
    )
    .unwrap();

    let cached = build_models_help_response(&current, temp.path());
    assert!(cached.contains("Cached model IDs (top 2):"));
    assert!(cached.contains("- `gpt-5`"));
    assert!(cached.contains("- `gpt-5-mini`"));
}

#[test]
fn build_providers_help_response_lists_current_route_and_commands() {
    let response = build_providers_help_response(&route("openai", "gpt-5", false));

    assert!(response.contains("Current provider: `openai`"));
    assert!(response.contains("Switch provider with `/models <provider>`"));
    assert!(response.contains("Request supervised tool approval"));
    assert!(response.contains("Available providers:"));
    for provider_id in available_provider_ids() {
        assert!(response.contains(&format!("- {provider_id}")));
    }
}
