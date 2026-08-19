use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

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

struct RecordingChannel {
    name: &'static str,
    sent: tokio::sync::Mutex<Vec<SendMessage>>,
    block_sends: bool,
    release_sends: AtomicBool,
    release_notify: Notify,
    active_sends: AtomicUsize,
    max_active_sends: AtomicUsize,
}

impl RecordingChannel {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            sent: tokio::sync::Mutex::new(Vec::new()),
            block_sends: false,
            release_sends: AtomicBool::new(true),
            release_notify: Notify::new(),
            active_sends: AtomicUsize::new(0),
            max_active_sends: AtomicUsize::new(0),
        }
    }

    fn blocking(name: &'static str) -> Self {
        Self {
            name,
            sent: tokio::sync::Mutex::new(Vec::new()),
            block_sends: true,
            release_sends: AtomicBool::new(false),
            release_notify: Notify::new(),
            active_sends: AtomicUsize::new(0),
            max_active_sends: AtomicUsize::new(0),
        }
    }

    async fn sent_contents(&self) -> Vec<String> {
        self.sent.lock().await.iter().map(|message| message.content.clone()).collect()
    }

    fn release(&self) {
        self.release_sends.store(true, AtomicOrdering::SeqCst);
        self.release_notify.notify_waiters();
    }

    async fn wait_for_active_sends(&self, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while self.active_sends.load(AtomicOrdering::SeqCst) < expected {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("active send count should reach expected value");
    }

    fn max_active_sends(&self) -> usize {
        self.max_active_sends.load(AtomicOrdering::SeqCst)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        self.name
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let active = self.active_sends.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        self.max_active_sends.fetch_max(active, AtomicOrdering::SeqCst);
        while self.block_sends && !self.release_sends.load(AtomicOrdering::SeqCst) {
            self.release_notify.notified().await;
        }
        self.sent.lock().await.push(message.clone());
        self.active_sends.fetch_sub(1, AtomicOrdering::SeqCst);
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

fn context(
    workspace_dir: &Path,
    channel_name: &str,
    channel: Option<Arc<dyn Channel>>,
    interrupt_on_new_message: bool,
) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    let mut channels_by_name = HashMap::new();
    if let Some(channel) = channel {
        channels_by_name.insert(channel_name.to_string(), channel);
    }

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
        workspace_dir: Arc::new(PathBuf::from(workspace_dir)),
        message_timeout_secs: 30,
        interrupt_on_new_message,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn message(id: &str, channel: &str, sender: &str, reply_target: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        id: id.to_string(),
        sender: sender.to_string(),
        reply_target: reply_target.to_string(),
        content: "/model".to_string(),
        channel: channel.to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}

#[test]
fn compute_max_in_flight_messages_clamps_to_configured_bounds() {
    assert_eq!(compute_max_in_flight_messages(0), CHANNEL_MIN_IN_FLIGHT_MESSAGES);
    assert_eq!(compute_max_in_flight_messages(1), CHANNEL_MIN_IN_FLIGHT_MESSAGES);
    assert_eq!(compute_max_in_flight_messages(4), 4 * CHANNEL_PARALLELISM_PER_CHANNEL);
    assert_eq!(compute_max_in_flight_messages(usize::MAX), CHANNEL_MAX_IN_FLIGHT_MESSAGES);
}

#[tokio::test]
async fn log_worker_join_result_ignores_success_and_records_join_errors() {
    log_worker_join_result(Ok(()));

    let panicked = tokio::spawn(async {
        panic!("worker failed");
    })
    .await;
    log_worker_join_result(panicked.map(|_| ()));

    let handle = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    });
    handle.abort();
    log_worker_join_result(handle.await.map(|_| ()));
}

#[tokio::test]
async fn run_message_dispatch_loop_processes_runtime_commands_until_input_closes() {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "telegram", Some(channel_dyn), false));
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(rx, ctx, 4).await;

    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 2);
    assert!(sent.iter().all(|content| content.contains("Current model")));
}

#[tokio::test]
async fn run_message_dispatch_loop_limits_in_flight_workers() {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::blocking("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "telegram", Some(channel_dyn), false));
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let dispatch = tokio::spawn(run_message_dispatch_loop(rx, ctx, 1));

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    channel.wait_for_active_sends(1).await;
    tokio::time::sleep(Duration::from_millis(25)).await;

    assert_eq!(channel.max_active_sends(), 1);

    channel.release();
    drop(tx);
    tokio::time::timeout(Duration::from_secs(1), dispatch).await.unwrap().unwrap();

    assert_eq!(channel.sent_contents().await.len(), 2);
    assert_eq!(channel.max_active_sends(), 1);
}

#[tokio::test]
async fn run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel() {
    // interrupt_on_new_message 为 true，但通道不是 telegram，
    // 因此 interrupt_enabled 为 false，两条同发送者消息都应被独立处理。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("slack"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "slack", Some(channel_dyn), true));
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "slack", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "slack", "sender-1", "chat-1")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(rx, ctx, 4).await;

    // 未触发中断，两条消息都得到回复。
    assert_eq!(channel.sent_contents().await.len(), 2);
}

#[tokio::test]
async fn run_message_dispatch_loop_keeps_distinct_telegram_senders_independent() {
    // 启用中断但两条消息来自不同发送者作用域，互不取消，应都完成。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "telegram", Some(channel_dyn), true));
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(rx, ctx, 4).await;

    assert_eq!(channel.sent_contents().await.len(), 2);
}

#[tokio::test]
async fn run_message_dispatch_loop_stops_when_semaphore_closed_before_receive() {
    // max_in_flight_messages 为 0 时信号量无许可，acquire 会等待；
    // 这里验证关闭接收端后循环优雅退出且不发送任何消息。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "telegram", Some(channel_dyn), false));
    let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(1);
    drop(tx);

    tokio::time::timeout(Duration::from_secs(1), run_message_dispatch_loop(rx, ctx, 1))
        .await
        .expect("loop should exit promptly when receiver is closed");

    assert!(channel.sent_contents().await.is_empty());
}

#[tokio::test]
async fn run_message_dispatch_loop_interrupts_previous_telegram_sender_request() {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::blocking("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(context(tmp.path(), "telegram", Some(channel_dyn), true));
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let dispatch = tokio::spawn(run_message_dispatch_loop(rx, ctx, 2));

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    channel.wait_for_active_sends(1).await;
    tx.send(message("msg-2", "telegram", "sender-1", "chat-1")).await.unwrap();
    tokio::time::sleep(Duration::from_millis(25)).await;

    assert_eq!(channel.max_active_sends(), 1);
    assert!(channel.sent_contents().await.is_empty());

    channel.release();
    drop(tx);
    tokio::time::timeout(Duration::from_secs(1), dispatch).await.unwrap().unwrap();

    assert_eq!(channel.sent_contents().await.len(), 2);
    assert_eq!(channel.max_active_sends(), 1);
}
