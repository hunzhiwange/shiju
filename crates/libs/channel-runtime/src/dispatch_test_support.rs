//! 真实宿主调度验收共享的通道记录夹具。
use channel_support::runtime_state::ChannelRuntimeContext;
use channel_types::{Channel, SendMessage, self as traits};
use agent_approval::ApprovalManager;
use agent_config::runtime_schema::AutonomyConfig;
use agent_memory::NoneMemory;
use agent_observability::NoopObserver;
use model_types::Provider;
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

pub struct RecordingChannel {
    name: &'static str,
    pub sent: tokio::sync::Mutex<Vec<SendMessage>>,
    block_sends: bool,
    block_feedback: bool,
    release_sends: AtomicBool,
    release_notify: Notify,
    active_sends: AtomicUsize,
    max_active_sends: AtomicUsize,
}

impl RecordingChannel {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            sent: tokio::sync::Mutex::new(Vec::new()),
            block_sends: false,
            block_feedback: false,
            release_sends: AtomicBool::new(true),
            release_notify: Notify::new(),
            active_sends: AtomicUsize::new(0),
            max_active_sends: AtomicUsize::new(0),
        }
    }

    pub fn blocking(name: &'static str) -> Self {
        Self {
            name,
            sent: tokio::sync::Mutex::new(Vec::new()),
            block_sends: true,
            block_feedback: false,
            release_sends: AtomicBool::new(false),
            release_notify: Notify::new(),
            active_sends: AtomicUsize::new(0),
            max_active_sends: AtomicUsize::new(0),
        }
    }

    pub fn feedback_blocking(name: &'static str) -> Self {
        Self { block_feedback: true, ..Self::blocking(name) }
    }

    pub async fn sent_contents(&self) -> Vec<String> {
        self.sent.lock().await.iter().map(|message| message.content.clone()).collect()
    }

    pub fn release(&self) {
        self.release_sends.store(true, AtomicOrdering::SeqCst);
        self.release_notify.notify_waiters();
    }

    pub async fn wait_for_active_sends(&self, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while self.active_sends.load(AtomicOrdering::SeqCst) < expected {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("active send count should reach expected value");
    }

    pub fn max_active_sends(&self) -> usize {
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

    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()> {
        if self.block_feedback {
            self.send(&SendMessage::new("typing feedback stopped", recipient)).await?;
        }
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn context(
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
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
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
        reliability: Arc::new(agent_config::runtime_schema::ReliabilityConfig::default()),
        provider_runtime_options: agent_providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(workspace_dir)),
        message_timeout_secs: 30,
        interrupt_on_new_message,
        multimodal: agent_config::runtime_schema::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: agent_config::runtime_schema::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

pub fn message(
    id: &str,
    channel: &str,
    sender: &str,
    reply_target: &str,
) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: id.to_string(),
        sender: sender.to_string(),
        reply_target: reply_target.to_string(),
        content: "/model".to_string(),
        channel: channel.to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}
