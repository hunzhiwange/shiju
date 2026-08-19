use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::hooks::{HookHandler, HookResult};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
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

struct CancelHook;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl HookHandler for CancelHook {
    fn name(&self) -> &str {
        "cancel"
    }

    async fn on_message_received(
        &self,
        _message: traits::ChannelMessage,
    ) -> HookResult<traits::ChannelMessage> {
        HookResult::Cancel("blocked".to_string())
    }
}

struct RecordingChannel {
    supports_drafts: bool,
    sent: tokio::sync::Mutex<Vec<String>>,
    draft_placeholders: tokio::sync::Mutex<Vec<String>>,
    draft_updates: tokio::sync::Mutex<Vec<String>>,
    reactions: tokio::sync::Mutex<Vec<String>>,
    typing_started: AtomicUsize,
    typing_stopped: AtomicUsize,
}

impl RecordingChannel {
    fn new(supports_drafts: bool) -> Self {
        Self {
            supports_drafts,
            sent: tokio::sync::Mutex::new(Vec::new()),
            draft_placeholders: tokio::sync::Mutex::new(Vec::new()),
            draft_updates: tokio::sync::Mutex::new(Vec::new()),
            reactions: tokio::sync::Mutex::new(Vec::new()),
            typing_started: AtomicUsize::new(0),
            typing_stopped: AtomicUsize::new(0),
        }
    }

    async fn sent_contents(&self) -> Vec<String> {
        self.sent.lock().await.clone()
    }

    async fn draft_placeholders(&self) -> Vec<String> {
        self.draft_placeholders.lock().await.clone()
    }

    async fn draft_updates(&self) -> Vec<String> {
        self.draft_updates.lock().await.clone()
    }

    async fn reactions(&self) -> Vec<String> {
        self.reactions.lock().await.clone()
    }

    fn typing_stopped(&self) -> usize {
        self.typing_stopped.load(Ordering::SeqCst)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sent.lock().await.push(message.content.clone());
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn supports_draft_updates(&self) -> bool {
        self.supports_drafts
    }

    async fn send_draft(&self, message: &SendMessage) -> anyhow::Result<Option<String>> {
        self.draft_placeholders.lock().await.push(message.content.clone());
        Ok(Some("draft-1".to_string()))
    }

    async fn update_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        text: &str,
    ) -> anyhow::Result<Option<String>> {
        self.draft_updates.lock().await.push(text.to_string());
        Ok(None)
    }

    async fn add_reaction(
        &self,
        _channel_id: &str,
        _message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        self.reactions.lock().await.push(emoji.to_string());
        Ok(())
    }

    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.typing_started.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.typing_stopped.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn context(
    channels_by_name: HashMap<String, Arc<dyn Channel>>,
    hooks: Option<Arc<crate::app::agent::hooks::HookRunner>>,
) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        channels_by_name: Arc::new(channels_by_name),
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
        hooks,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn message(content: &str, channel: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        id: "msg".to_string(),
        sender: "sender".to_string(),
        reply_target: "reply".to_string(),
        content: content.to_string(),
        channel: channel.to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}

#[tokio::test]
async fn process_channel_message_returns_immediately_when_cancelled() {
    let ctx = Arc::new(context(HashMap::new(), None));
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    process_channel_message(Arc::clone(&ctx), message("hello", "test"), cancellation).await;

    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[tokio::test]
async fn process_channel_message_stops_when_hook_cancels_message() {
    let mut runner = crate::app::agent::hooks::HookRunner::new();
    runner.register(Box::new(CancelHook));
    let ctx = Arc::new(context(HashMap::new(), Some(Arc::new(runner))));

    process_channel_message(Arc::clone(&ctx), message("hello", "test"), CancellationToken::new())
        .await;

    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[tokio::test]
async fn process_channel_message_consumes_runtime_command_before_model_execution() {
    let channel = Arc::new(RecordingChannel::new(false));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(
        HashMap::from([("telegram".to_string(), Arc::clone(&channel_dyn))]),
        None,
    ));
    let msg = message("/model gpt-4.1", "telegram");

    process_channel_message(Arc::clone(&ctx), msg.clone(), CancellationToken::new()).await;

    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
    assert_eq!(get_route_selection(ctx.as_ref(), &conversation_history_key(&msg)).model, "gpt-4.1");
    assert_eq!(channel.sent_contents().await.len(), 1);
}

#[tokio::test]
async fn process_channel_message_notifies_provider_initialization_failure() {
    let channel = Arc::new(RecordingChannel::new(false));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(
        HashMap::from([("telegram".to_string(), Arc::clone(&channel_dyn))]),
        None,
    ));
    let msg = message("hello", "telegram");
    ctx.route_overrides.lock().unwrap().insert(
        conversation_history_key(&msg),
        ChannelRouteSelection {
            provider: "missing-provider".to_string(),
            model: "model".to_string(),
            task_mode_enabled: false,
        },
    );

    process_channel_message(Arc::clone(&ctx), msg, CancellationToken::new()).await;

    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Failed to initialize provider `missing-provider`"));
}

#[test]
fn excluded_tools_snapshot_for_channel_skips_cli_and_clones_runtime_list() {
    let ctx = context(HashMap::new(), None);
    ctx.non_cli_excluded_tools.lock().unwrap().extend(["shell".to_string(), "grep".to_string()]);

    assert!(excluded_tools_snapshot_for_channel(&ctx, "cli").is_empty());
    assert_eq!(
        excluded_tools_snapshot_for_channel(&ctx, "telegram"),
        vec!["shell".to_string(), "grep".to_string()]
    );
}

#[tokio::test]
async fn prepare_streaming_ui_state_skips_setup_when_streaming_is_disabled() {
    let msg = message("hello", "telegram");
    let state = prepare_streaming_ui_state(None, &msg, false, false).await;

    assert!(state.delta_tx.is_none());
    assert!(state.draft_message_id.is_none());
    assert!(state.draft_updater.is_none());
}

#[tokio::test]
async fn prepare_streaming_ui_state_without_target_keeps_delta_channel_only() {
    let msg = message("hello", "telegram");
    let state = prepare_streaming_ui_state(None, &msg, false, true).await;

    assert!(state.delta_tx.is_some());
    assert!(state.draft_message_id.is_none());
    assert!(state.draft_updater.is_none());

    maybe_add_processing_reaction(None, &msg).await;
}

#[tokio::test]
async fn prepare_streaming_ui_state_tracks_visible_draft_updates_only() {
    let channel = Arc::new(RecordingChannel::new(true));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let msg = message("hello", "telegram");
    let state = prepare_streaming_ui_state(Some(&channel_dyn), &msg, false, true).await;
    let delta_tx = state.delta_tx.expect("streaming should allocate a delta channel");

    delta_tx
        .send(format!("{}hidden", crate::app::agent::agent::loop_::DRAFT_PROGRESS_SENTINEL))
        .await
        .unwrap();
    delta_tx.send("abc".to_string()).await.unwrap();
    delta_tx.send(crate::app::agent::agent::loop_::DRAFT_CLEAR_SENTINEL.to_string()).await.unwrap();
    delta_tx.send("xy".to_string()).await.unwrap();
    drop(delta_tx);

    assert_eq!(state.draft_message_id.as_deref(), Some("draft-1"));
    finish_feedback_tasks(state.draft_updater, None, None).await;

    assert_eq!(channel.draft_placeholders().await, vec!["...".to_string()]);
    assert_eq!(channel.draft_updates().await, vec!["abc".to_string(), "xy".to_string()]);
}

#[tokio::test]
async fn maybe_add_processing_reaction_records_processing_emoji() {
    let channel = Arc::new(RecordingChannel::new(false));
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    maybe_add_processing_reaction(Some(&channel_dyn), &message("hello", "telegram")).await;

    assert_eq!(channel.reactions().await, vec!["\u{1F440}".to_string()]);
}

#[tokio::test]
async fn finish_feedback_tasks_cancels_typing_task() {
    let channel = Arc::new(RecordingChannel::new(false));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (typing_cancellation, typing_task) =
        start_typing_feedback(Some(&channel_dyn), "reply".to_string());

    finish_feedback_tasks(None, typing_cancellation, typing_task).await;

    assert_eq!(channel.typing_stopped(), 1);
}
