// 本文件验证渠道消息执行的会话权限、可靠性快照、取消与超时结果。

use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

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

fn context(workspace_dir: PathBuf) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
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
        workspace_dir: Arc::new(workspace_dir),
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

fn message(sender: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        id: "msg".to_string(),
        sender: sender.to_string(),
        reply_target: "reply".to_string(),
        content: "hello".to_string(),
        channel: "test".to_string(),
        timestamp: 1,
        thread_ts: None,
    }
}

fn setup() -> (TempDir, ChannelRuntimeContext, traits::ChannelMessage) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let sender = format!("sender-{}", std::process::id());
    let ctx = context(workspace.path().to_path_buf());
    let msg = message(&sender);
    (workspace, ctx, msg)
}

#[test]
fn channel_session_options_enable_full_access() {
    let options = channel_session_options();

    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
}

#[test]
fn channel_request_uses_runtime_reliability_snapshot() {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let mut ctx = context(workspace.path().to_path_buf());
    ctx.reliability = Arc::new(crate::app::agent::config::ReliabilityConfig {
        provider_retries: 8,
        provider_backoff_ms: 1_750,
        ..Default::default()
    });

    let reliability = channel_request_reliability(&ctx);
    assert_eq!(reliability.provider_retries, 8);
    assert_eq!(reliability.provider_backoff_ms, 1_750);
}

async fn cleanup_sender_session(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) {
    let scope = resolve_channel_project_scope_id(ctx).await;
    let key = format!("{scope}::{}", sender_session_key(msg));
    sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
}

#[tokio::test]
// 预先取消必须在启动提供者前短路，避免产生副作用。
async fn run_message_execution_returns_cancelled_when_token_is_already_cancelled() {
    let (_workspace, ctx, msg) = setup();
    let route = ChannelRouteSelection {
        provider: "provider".to_string(),
        model: "model".to_string(),
        task_mode_enabled: false,
    };
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let result =
        run_message_execution(&ctx, &msg, &route, Vec::new(), None, 30, &cancellation).await;

    assert!(matches!(result, LlmExecutionResult::Cancelled));
    cleanup_sender_session(&ctx, &msg).await;
}

#[tokio::test]
async fn run_message_execution_wraps_timeout_as_completed_result() {
    let (_workspace, ctx, msg) = setup();
    let route = ChannelRouteSelection {
        provider: "provider".to_string(),
        model: "model".to_string(),
        task_mode_enabled: false,
    };
    let cancellation = CancellationToken::new();

    let result =
        run_message_execution(&ctx, &msg, &route, Vec::new(), None, 0, &cancellation).await;

    assert!(matches!(result, LlmExecutionResult::Completed(Err(_))));
    cleanup_sender_session(&ctx, &msg).await;
}
