// 本文件覆盖运行时命令处理器的命令分派、工具调用与渠道回复。

use super::super::{Channel, ChannelRuntimeContext, SendMessage, traits};
use super::handle_runtime_command_if_needed;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{
    AutonomyConfig, NonCliNaturalLanguageApprovalMode, ReliabilityConfig,
};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use model_types::{ChatMessage, Provider};
use crate::app::agent::providers::ProviderRuntimeOptions;
use crate::app::agent::tools::{Tool, ToolResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct StaticProvider;
struct CountingProvider(Arc<std::sync::atomic::AtomicUsize>);
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

struct NamedTool(&'static str);

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

struct RecordingChannel {
    sent: tokio::sync::Mutex<Vec<SendMessage>>,
    fail_send: bool,
}

impl RecordingChannel {
    fn new() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: false }
    }

    async fn sent_contents(&self) -> Vec<String> {
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
fn boxed_tools(names: &[&'static str]) -> Arc<Vec<Box<dyn Tool>>> {
    Arc::new(names.iter().map(|name| Box::new(NamedTool(*name)) as Box<dyn Tool>).collect())
}

fn autonomy_config(
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

fn context(
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
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&autonomy)),
    }
}

fn message(content: &str) -> traits::ChannelMessage {
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

fn message_in_channel(channel_name: &str, content: &str) -> traits::ChannelMessage {
    let mut msg = message(content);
    msg.channel = channel_name.to_string();
    msg
}

fn create_pending(ctx: &ChannelRuntimeContext, tool_name: &str) -> String {
    ctx.approval_manager
        .create_non_cli_pending_request(
            tool_name,
            "sender",
            "telegram",
            "chat-1",
            Some("reason".to_string()),
            Value::Null,
            None,
            None,
        )
        .request_id
}

async fn dispatch(ctx: &ChannelRuntimeContext, channel: &Arc<dyn Channel>, content: &str) -> bool {
    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    handle_runtime_command_if_needed(ctx, &message(content), Some(channel)).await
}

#[tokio::test(flavor = "multi_thread")]
async fn slash_command_dispatch_covers_session_model_and_approval_variants() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    for content in [
        "/models",
        "/models definitely-missing-provider",
        "/model",
        "/model `gpt-next`",
        "/new",
        "/task",
        "/approve-all-once",
        "/approve-request shell",
    ] {
        assert!(dispatch(&ctx, &channel_dyn, content).await, "{content}");
    }

    let allow_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-allow {allow_id}")).await);
    let confirm_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-confirm {confirm_id}")).await);
    let deny_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-deny {deny_id}")).await);

    for content in ["/approve-pending", "/approve shell", "/unapprove shell", "/approvals"] {
        assert!(dispatch(&ctx, &channel_dyn, content).await, "{content}");
    }

    let sent = channel.sent_contents().await;
    assert!(sent.iter().any(|text| text.contains("无可用模型目录")));
    assert!(sent.iter().any(|text| text.contains("Unknown provider")));
    assert!(sent.iter().any(|text| text.contains("Current model")));
    assert!(sent.iter().any(|text| text.contains("模型未在当前 provider 目录中声明")));
    assert!(sent.iter().any(|text| text.contains("会话已重置")));
    assert!(sent.iter().any(|text| text.contains("我进入了任务模式")));
    assert!(sent.iter().any(|text| text.contains("One-time all-tools approval request created.")));
    assert!(sent.iter().any(|text| text.contains("Approval request created.")));
    assert!(sent.iter().any(|text| text.contains("Approved pending request")));
    assert!(sent.iter().any(|text| text.contains("Approved supervised execution for `shell`")));
    assert!(sent.iter().any(|text| text.contains("Denied pending approval request")));
    assert!(sent.iter().any(|text| text.contains("Pending approval requests")));
    assert!(sent.iter().any(|text| text.contains("Runtime session grant removed for `shell`")));
    assert!(sent.iter().any(|text| text.contains("Supervised non-CLI tool approvals:")));
}

#[tokio::test]
async fn channel_command_help_pipeline_routes_named_instance_without_project_or_model() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    ctx.provider = Arc::new(CountingProvider(calls.clone()));
    let selected = Arc::new(RecordingChannel::new());
    let other = Arc::new(RecordingChannel::new());
    ctx.channels_by_name = Arc::new(HashMap::from([
        (super::super::channel_instance_key("lark", "work"), selected.clone() as Arc<dyn Channel>),
        (super::super::channel_instance_key("lark", "default"), other.clone() as Arc<dyn Channel>),
    ]));
    let mut msg = message_in_channel("lark", "/help");
    msg.instance_id = "work".into();
    // Help remains available when a configured project binding is invalid.
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("lark:work".into(), "missing-project".into());
    super::super::process_channel_message(
        Arc::new(ctx),
        msg,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let sent = selected.sent_contents().await.join("\n");
    assert!(sent.contains("全部已接通命令"));
    assert!(sent.contains("/approve-confirm <request-id>"));
    assert!(other.sent_contents().await.is_empty());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
