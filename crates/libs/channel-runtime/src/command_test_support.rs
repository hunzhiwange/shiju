//! Shared command fixtures for domain tests and real host boundary tests.
// 本文件覆盖运行时命令处理器的命令分派、工具调用与渠道回复。

pub use channel_types::{Channel, SendMessage, self as traits};
pub use channel_support::runtime_state::ChannelRuntimeContext;
pub use crate::runtime_command::command::{ChannelRuntimeCommand, parse_runtime_command};

pub use agent_approval::ApprovalManager;
pub use agent_config::runtime_schema::{
    AutonomyConfig, NonCliNaturalLanguageApprovalMode, ReliabilityConfig,
};
pub use agent_memory::NoneMemory;
pub use agent_observability::NoopObserver;
pub use model_types::{ChatMessage, Provider};
pub use agent_providers::ProviderRuntimeOptions;
pub use tool_types::{Tool, ToolResult};
pub use serde_json::Value;
pub use std::collections::HashMap;
pub use std::path::{Path, PathBuf};
pub use std::sync::{Arc, Mutex};

pub struct StaticProvider;
pub struct CountingProvider(pub Arc<std::sync::atomic::AtomicUsize>);
#[async_trait::async_trait]
impl Provider for CountingProvider {
    async fn chat_with_system(
        &self,
        _system: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok("unexpected model call".into())
    }
}

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

pub struct NamedTool(&'static str);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "unit test tool"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

#[allow(clippy::new_without_default)]
pub struct RecordingChannel {
    pub sent: tokio::sync::Mutex<Vec<SendMessage>>,
    fail_send: bool,
}

impl RecordingChannel {
    pub fn new() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: false }
    }

    pub fn failing() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: true }
    }

    pub async fn sent_contents(&self) -> Vec<String> {
        self.sent.lock().await.iter().map(|message| message.content.clone()).collect()
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

// 工具夹具固定可用集合，验证命令不会越权访问其它工具。
pub fn boxed_tools(names: &[&'static str]) -> Arc<Vec<Box<dyn Tool>>> {
    Arc::new(names.iter().map(|name| Box::new(NamedTool(*name)) as Box<dyn Tool>).collect())
}

pub fn autonomy_config(
    auto_approve: &[&str],
    approvers: &[&str],
    mode: NonCliNaturalLanguageApprovalMode,
) -> AutonomyConfig {
    let mut config = AutonomyConfig::default();
    config.auto_approve = auto_approve.iter().map(|tool| (*tool).to_string()).collect();
    config.always_ask = Vec::new();
    config.non_cli_approval_approvers =
        approvers.iter().map(|approver| (*approver).to_string()).collect();
    config.non_cli_natural_language_approval_mode = mode;
    config
}

pub fn context(
    workspace_dir: &Path,
    tool_names: &[&'static str],
    autonomy: AutonomyConfig,
) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: boxed_tools(tool_names),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 2,
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
        multimodal: agent_config::runtime_schema::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: agent_config::runtime_schema::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&autonomy)),
    }
}

pub fn message(content: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-1".to_string(),
        sender: "sender".to_string(),
        reply_target: "chat-1".to_string(),
        content: content.to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}

pub fn message_in_channel(channel_name: &str, content: &str) -> traits::ChannelMessage {
    let mut msg = message(content);
    msg.channel = channel_name.to_string();
    msg
}


#[derive(Default)]
pub struct CatalogChannel(pub tokio::sync::Mutex<Vec<SendMessage>>);
#[async_trait::async_trait]
impl Channel for CatalogChannel {
    fn name(&self) -> &str {
        "lark"
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.0.lock().await.push(message.clone());
        Ok(())
    }
    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
