use super::super::*;
use super::task_mode::{handle_task_mode_message_if_needed, set_sender_task_mode};
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

struct RecordingChannel {
    sent: tokio::sync::Mutex<Vec<SendMessage>>,
    fail_send: bool,
}

impl RecordingChannel {
    fn failing() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: true }
    }
}

impl Default for RecordingChannel {
    fn default() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: false }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        if self.fail_send {
            anyhow::bail!("send failed");
        }
        self.sent.lock().await.push(message.clone());
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
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
        max_tool_iterations: 4,
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
fn set_sender_task_mode_toggles_only_task_mode_flag() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    set_route_selection(&ctx, "sender", route("custom-provider", "custom-model", false));

    set_sender_task_mode(&ctx, "sender", true);
    assert_eq!(get_route_selection(&ctx, "sender"), route("custom-provider", "custom-model", true));

    set_sender_task_mode(&ctx, "sender", false);
    assert_eq!(
        get_route_selection(&ctx, "sender"),
        route("custom-provider", "custom-model", false)
    );
}

#[tokio::test]
async fn task_mode_message_returns_false_when_mode_is_disabled() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    let handled = handle_task_mode_message_if_needed(
        &ctx,
        &message("create this"),
        "sender",
        Some(&channel_dyn),
    )
    .await;

    assert!(!handled);
    assert!(channel.sent.lock().await.is_empty());
}

#[tokio::test]
async fn task_mode_consumes_message_without_channel_or_content() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    set_sender_task_mode(&ctx, "sender", true);

    assert!(
        handle_task_mode_message_if_needed(&ctx, &message("create this"), "sender", None).await
    );

    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    assert!(
        handle_task_mode_message_if_needed(&ctx, &message("   "), "sender", Some(&channel_dyn))
            .await
    );
    assert!(channel.sent.lock().await.is_empty());
}

#[tokio::test]
async fn task_mode_creates_task_and_sends_threaded_response() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    set_route_selection(&ctx, "sender", route("default-provider", "manual-model", true));
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    let handled = handle_task_mode_message_if_needed(
        &ctx,
        &message("  ship the feature  "),
        "sender",
        Some(&channel_dyn),
    )
    .await;

    assert!(handled);
    let sent = channel.sent.lock().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].content.starts_with("任务已创建，任务编号：T"));
    assert_eq!(sent[0].recipient, "room-1");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn task_mode_keeps_auto_model_when_route_uses_auto() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    set_route_selection(&ctx, "sender", route("default-provider", "auto", true));
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    let handled = handle_task_mode_message_if_needed(
        &ctx,
        &message("  keep auto model  "),
        "sender",
        Some(&channel_dyn),
    )
    .await;

    assert!(handled);
    let sent = channel.sent.lock().await;
    assert_eq!(sent.len(), 1);
    let task_id = sent[0]
        .content
        .strip_prefix("任务已创建，任务编号：")
        .expect("success response should include task id")
        .to_string();
    drop(sent);

    let conn = rusqlite::Connection::open(
        temp.path()
            .join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
            .join("tasks")
            .join("_index.sqlite3"),
    )
    .expect("task db should open");
    let (model, prompt): (String, String) = conn
        .query_row("SELECT model, prompt FROM tasks WHERE id = ?1", [&task_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("created task should be persisted");

    assert_eq!(model, "auto");
    assert_eq!(prompt, "keep auto model");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn task_mode_reports_task_creation_error() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_file = temp.path().join("project-file");
    std::fs::write(&project_file, "").expect("project file");
    let ctx = context(&project_file);
    set_sender_task_mode(&ctx, "sender", true);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    let handled = handle_task_mode_message_if_needed(
        &ctx,
        &message("make a task"),
        "sender",
        Some(&channel_dyn),
    )
    .await;

    assert!(handled);
    let sent = channel.sent.lock().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].content.starts_with("任务创建失败："));
}

#[tokio::test]
async fn task_mode_still_consumes_message_when_reply_send_fails() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path());
    set_sender_task_mode(&ctx, "sender", true);
    let channel = Arc::new(RecordingChannel::failing());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    let handled = handle_task_mode_message_if_needed(
        &ctx,
        &message("create even if reply fails"),
        "sender",
        Some(&channel_dyn),
    )
    .await;

    assert!(handled);
    assert!(channel.sent.lock().await.is_empty());
}
