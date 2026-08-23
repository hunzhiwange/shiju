use super::super::*;
use super::session_commands::{
    handle_new_session, handle_set_model, handle_set_provider, handle_show_model,
    handle_show_providers, handle_task_mode, with_test_provider_init_error,
};
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{AutonomyConfig, ReliabilityConfig};
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

fn context(
    workspace_dir: &Path,
    default_provider: &str,
    default_model: &str,
) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new(default_provider.to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::<Box<dyn Tool>>::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new(default_model.to_string()),
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
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn message(content: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        id: "msg-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "room-1".to_string(),
        content: content.to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}

#[test]
fn handle_show_providers_marks_current_provider() {
    let current = route("openai", "gpt-4.1", false);

    let response = handle_show_providers(&current);

    assert!(response.contains("openai"));
    assert!(response.contains("gpt-4.1"));
}

#[test]
fn handle_show_model_includes_current_model() {
    let current = route("openai", "gpt-4.1", false);

    let response = handle_show_model(&current, Path::new("/tmp"));

    assert!(response.contains("gpt-4.1"));
}

#[test]
fn handle_set_model_rejects_blank_model_without_mutating_route() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let mut current = route("default-provider", "default-model", false);

    let response = handle_set_model(&ctx, "sender", &mut current, "  `  `  ".to_string());

    assert_eq!(response, "Model ID cannot be empty. Use `/model <model-id>`.");
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[test]
fn handle_set_model_trims_backticks_persists_route_and_clears_history() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("default-provider", "default-model", false);

    let response = handle_set_model(&ctx, "sender", &mut current, "  `gpt-5`  ".to_string());

    assert!(response.contains("Model switched to `gpt-5`"));
    assert_eq!(current, route("default-provider", "gpt-5", false));
    assert_eq!(get_route_selection(&ctx, "sender"), current);
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}

#[test]
fn handle_set_model_updates_only_target_sender_and_preserves_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("other-sender".to_string(), vec![ChatMessage::user("other turn")]);
    set_route_selection(&ctx, "other-sender", route("other-provider", "other-model", false));
    let mut current = route("default-provider", "default-model", true);

    let response = handle_set_model(&ctx, "target-sender", &mut current, "next-model".to_string());

    assert!(response.contains("Model switched to `next-model`"));
    assert_eq!(current, route("default-provider", "next-model", true));
    assert_eq!(
        get_route_selection(&ctx, "other-sender"),
        route("other-provider", "other-model", false)
    );
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("other-sender"));
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_reports_unknown_provider_without_mutating_state() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("default-provider", "default-model", false);

    let raw_provider = "missing-provider-for-session-command-test";
    let response =
        handle_set_provider(&ctx, "sender", &mut current, raw_provider.to_string()).await;

    assert_eq!(
        response,
        format!("Unknown provider `{raw_provider}`. Use `/providers` to list valid providers.")
    );
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_uses_default_provider_without_initializing_new_provider() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), &provider_name, "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("old-provider", "custom-model", false);

    let response =
        handle_set_provider(&ctx, "sender", &mut current, provider_name.to_ascii_uppercase()).await;

    assert!(response.contains(&format!("Provider switched to `{provider_name}`")));
    assert_eq!(current, route(&provider_name, "custom-model", false));
    assert_eq!(get_route_selection(&ctx, "sender"), current);
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.provider_cache.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_leaves_history_when_provider_is_already_selected() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), &provider_name, "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route(&provider_name, "default-model", false);

    let response = handle_set_provider(&ctx, "sender", &mut current, provider_name.clone()).await;

    assert!(response.contains(&format!("Provider switched to `{provider_name}`")));
    assert_eq!(current, route(&provider_name, "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_reports_sanitized_init_error_without_mutating_state() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("default-provider", "default-model", false);
    let injected_error = "provider init failed Authorization: Bearer sk-secret-value";

    let response = with_test_provider_init_error(
        injected_error,
        handle_set_provider(&ctx, "sender", &mut current, provider_name.clone()),
    )
    .await;

    assert!(response.contains(&format!("Failed to initialize provider `{provider_name}`")));
    assert!(response.contains("Route unchanged."));
    assert!(response.contains("[REDACTED]"));
    assert!(!response.contains("sk-secret-value"));
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[tokio::test]
async fn handle_new_session_clears_session_history_and_disables_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let msg = message("/new");
    let sender_key = sender_session_key(&msg);
    let scope = resolve_channel_project_scope_id(&ctx).await;
    let session_key = format!("{scope}::{sender_key}");
    sender_session_store().lock().unwrap().insert(session_key.clone(), "session-1".to_string());
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(sender_key.clone(), vec![ChatMessage::user("old turn")]);
    set_route_selection(&ctx, &sender_key, route("default-provider", "default-model", true));

    let response = handle_new_session(&ctx, &msg, &sender_key).await;

    assert_eq!(response, "会话已重置. 开始新会话.");
    assert!(!sender_session_store().lock().unwrap().contains_key(&session_key));
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&sender_key));
    assert!(!get_route_selection(&ctx, &sender_key).task_mode_enabled);
}

#[tokio::test]
async fn handle_task_mode_clears_session_history_and_enables_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let msg = message("/task");
    let sender_key = sender_session_key(&msg);
    let scope = resolve_channel_project_scope_id(&ctx).await;
    let session_key = format!("{scope}::{sender_key}");
    sender_session_store().lock().unwrap().insert(session_key.clone(), "session-1".to_string());
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(sender_key.clone(), vec![ChatMessage::user("old turn")]);

    let response = handle_task_mode(&ctx, &msg, &sender_key).await;

    assert_eq!(response, "我进入了任务模式。");
    assert!(!sender_session_store().lock().unwrap().contains_key(&session_key));
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&sender_key));
    assert!(get_route_selection(&ctx, &sender_key).task_mode_enabled);
}
// 本文件验证会话提供者和模型的展示、切换、持久化与历史清理。
