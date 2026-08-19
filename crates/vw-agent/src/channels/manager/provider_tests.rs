use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::tools::{ToolResult, ToolSpec};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct TestTool {
    id: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for TestTool {
    fn name(&self) -> &str {
        self.id
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.id, self.description(), self.parameters_schema())
    }
}

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

fn context(default_provider: &str) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new(default_provider.to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: Some("https://ctx.example.test".to_string()),
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

#[test]
fn filtered_tool_specs_for_runtime_returns_all_specs_when_no_exclusions() {
    let tools: Vec<Box<dyn Tool>> =
        vec![Box::new(TestTool { id: "shell" }), Box::new(TestTool { id: "file_read" })];

    let specs = filtered_tool_specs_for_runtime(&tools, &[]);

    assert_eq!(
        specs.iter().map(|spec| spec.id.as_str()).collect::<Vec<_>>(),
        ["shell", "file_read"]
    );
}

#[test]
fn filtered_tool_specs_for_runtime_excludes_exact_id_matches_only() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(TestTool { id: "shell" }),
        Box::new(TestTool { id: "shell_extra" }),
        Box::new(TestTool { id: "memory_recall" }),
    ];
    let excluded = vec!["shell".to_string(), "missing".to_string()];

    let specs = filtered_tool_specs_for_runtime(&tools, &excluded);

    assert_eq!(
        specs.iter().map(|spec| spec.id.as_str()).collect::<Vec<_>>(),
        ["shell_extra", "memory_recall"]
    );
}

#[tokio::test]
async fn get_or_create_provider_returns_cached_provider_before_default_lookup() {
    let ctx = context("openai");
    let cached: Arc<dyn Provider> = Arc::new(StaticProvider);
    ctx.provider_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert("openai".to_string(), Arc::clone(&cached));

    let provider = get_or_create_provider(&ctx, "openai").await.unwrap();

    assert!(Arc::ptr_eq(&provider, &cached));
}

#[tokio::test]
async fn get_or_create_provider_returns_default_provider_without_caching_it() {
    let ctx = context("openai");

    let provider = get_or_create_provider(&ctx, "openai").await.unwrap();

    assert!(Arc::ptr_eq(&provider, &ctx.provider));
    assert!(ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
}

#[tokio::test]
async fn get_or_create_provider_recovers_from_poisoned_cache_for_default_provider() {
    let ctx = context("openai");
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner());
        panic!("poison provider cache");
    }));

    let provider = get_or_create_provider(&ctx, "openai").await.unwrap();

    assert!(Arc::ptr_eq(&provider, &ctx.provider));
}

#[tokio::test]
async fn get_or_create_provider_creates_and_caches_supported_non_default_provider() {
    let ctx = context("ctx-provider");

    let provider = get_or_create_provider(&ctx, "openai").await.unwrap();
    let cached = ctx
        .provider_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get("openai")
        .cloned()
        .expect("created provider should be cached");

    assert!(Arc::ptr_eq(&provider, &cached));
}

#[tokio::test]
async fn get_or_create_provider_concurrent_initialization_reuses_cached_instance() {
    let ctx = context("ctx-provider");

    let (first, second) = tokio::join!(
        get_or_create_provider(&ctx, "openai"),
        get_or_create_provider(&ctx, "openai")
    );
    let first = first.unwrap();
    let second = second.unwrap();

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner()).len(), 1);
}

#[tokio::test]
async fn create_resilient_provider_nonblocking_returns_provider_for_supported_provider() {
    let provider = create_resilient_provider_nonblocking(
        "openai",
        Some("key".to_string()),
        Some("https://example.test".to_string()),
        crate::app::agent::config::ReliabilityConfig::default(),
        crate::app::agent::providers::ProviderRuntimeOptions::default(),
    )
    .await
    .unwrap();

    assert!(!provider.supports_native_tools());
}

#[tokio::test]
async fn create_resilient_provider_nonblocking_returns_error_for_unknown_provider() {
    let err = match create_resilient_provider_nonblocking(
        "__definitely_missing_provider__",
        None,
        None,
        crate::app::agent::config::ReliabilityConfig::default(),
        crate::app::agent::providers::ProviderRuntimeOptions::default(),
    )
    .await
    {
        Ok(_) => panic!("unknown provider should fail"),
        Err(error) => error,
    };

    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn create_routed_provider_nonblocking_returns_provider_for_supported_provider() {
    let provider = create_routed_provider_nonblocking(
        "openai",
        None,
        None,
        crate::app::agent::config::ReliabilityConfig::default(),
        Vec::new(),
        "gpt".to_string(),
        crate::app::agent::providers::ProviderRuntimeOptions::default(),
    )
    .await
    .unwrap();

    assert!(!provider.supports_native_tools());
}

#[tokio::test]
async fn create_routed_provider_nonblocking_returns_error_for_unknown_provider() {
    let err = match create_routed_provider_nonblocking(
        "__definitely_missing_provider__",
        None,
        None,
        crate::app::agent::config::ReliabilityConfig::default(),
        Vec::new(),
        "model".to_string(),
        crate::app::agent::providers::ProviderRuntimeOptions::default(),
    )
    .await
    {
        Ok(_) => panic!("unknown routed provider should fail"),
        Err(error) => error,
    };

    assert!(!err.to_string().is_empty());
}
