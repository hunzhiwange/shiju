use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn context_with_exclusions(excluded: Vec<String>) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(excluded)),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

#[test]
fn is_non_cli_tool_excluded_matches_exact_tool_names() {
    let ctx = context_with_exclusions(vec!["shell".to_string(), "memory_recall".to_string()]);

    assert!(is_non_cli_tool_excluded(&ctx, "shell"));
    assert!(!is_non_cli_tool_excluded(&ctx, "shell_extra"));
    assert!(!is_non_cli_tool_excluded(&ctx, ""));
}

#[test]
fn is_non_cli_tool_excluded_returns_false_for_empty_exclusion_list() {
    let ctx = context_with_exclusions(Vec::new());

    assert!(!is_non_cli_tool_excluded(&ctx, "shell"));
    assert!(!is_non_cli_tool_excluded(&ctx, ""));
}

#[test]
fn is_non_cli_tool_excluded_recovers_from_poisoned_lock() {
    let ctx = context_with_exclusions(vec!["shell".to_string()]);

    // 故意污染互斥锁：在持锁线程内 panic，使锁进入 poisoned 状态。
    let tools = Arc::clone(&ctx.non_cli_excluded_tools);
    let _ = std::thread::spawn(move || {
        let _guard = tools.lock().unwrap();
        panic!("poison the lock");
    })
    .join();

    assert!(ctx.non_cli_excluded_tools.is_poisoned());
    // 即使锁被污染，函数仍应恢复内部值并正确判断。
    assert!(is_non_cli_tool_excluded(&ctx, "shell"));
    assert!(!is_non_cli_tool_excluded(&ctx, "other"));
}

#[test]
fn is_context_window_overflow_error_matches_known_provider_phrases() {
    assert!(is_context_window_overflow_error(&anyhow::anyhow!("maximum context length reached")));
    assert!(is_context_window_overflow_error(&anyhow::anyhow!("too many tokens")));
    assert!(!is_context_window_overflow_error(&anyhow::anyhow!("ordinary failure")));
}

#[test]
fn is_context_window_overflow_error_matches_every_known_hint_case_insensitively() {
    // 覆盖全部已知提示，并验证不区分大小写匹配（输入使用大写）。
    for phrase in [
        "this prompt EXCEEDS THE CONTEXT WINDOW",
        "CONTEXT WINDOW OF THIS MODEL is 8k",
        "MAXIMUM CONTEXT LENGTH is 4096",
        "CONTEXT LENGTH EXCEEDED by request",
        "TOO MANY TOKENS supplied",
        "TOKEN LIMIT EXCEEDED for model",
        "PROMPT IS TOO LONG to process",
        "the INPUT IS TOO LONG",
    ] {
        assert!(
            is_context_window_overflow_error(&anyhow::anyhow!("{phrase}")),
            "phrase should match: {phrase}"
        );
    }
}

#[test]
fn is_context_window_overflow_error_rejects_empty_and_unrelated_messages() {
    assert!(!is_context_window_overflow_error(&anyhow::anyhow!("")));
    assert!(!is_context_window_overflow_error(&anyhow::anyhow!("network timeout")));
}

#[test]
fn is_tool_iteration_limit_error_delegates_to_agent_loop_detector() {
    assert!(is_tool_iteration_limit_error(&anyhow::anyhow!(
        "Agent exceeded maximum tool iterations (7)"
    )));
    assert!(!is_tool_iteration_limit_error(&anyhow::anyhow!("ordinary failure")));
}
