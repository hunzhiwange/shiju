// 本文件覆盖运行时命令处理器的命令分派、工具调用与渠道回复。

use super::super::{Channel, ChannelRuntimeContext, SendMessage, traits};
use super::command::{ChannelRuntimeCommand, parse_runtime_command};
use super::handle_runtime_command_if_needed;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{
    AutonomyConfig, NonCliNaturalLanguageApprovalMode, ReliabilityConfig,
};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::tools::{Tool, ToolResult};
use serde_json::Value;
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

    fn failing() -> Self {
        Self { sent: tokio::sync::Mutex::new(Vec::new()), fail_send: true }
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
    handle_runtime_command_if_needed(ctx, &message(content), Some(channel)).await
}

async fn dispatch_message(
    ctx: &ChannelRuntimeContext,
    channel: &Arc<dyn Channel>,
    msg: &traits::ChannelMessage,
) -> bool {
    handle_runtime_command_if_needed(ctx, msg, Some(channel)).await
}

#[test]
fn runtime_command_parser_keeps_model_switch_payload() {
    assert_eq!(
        parse_runtime_command("telegram", "/model gpt-4.1"),
        Some(ChannelRuntimeCommand::SetModel("gpt-4.1".to_string()))
    );
}

#[tokio::test]
async fn non_command_returns_false_when_task_mode_is_off() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(
        !handle_runtime_command_if_needed(&ctx, &message("ordinary chat"), Some(&channel_dyn))
            .await
    );
    assert!(channel.sent_contents().await.is_empty());
}

#[tokio::test]
async fn parsed_command_is_consumed_without_target_channel() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );

    assert!(handle_runtime_command_if_needed(&ctx, &message("/model"), None).await);
}

#[tokio::test]
async fn approval_management_command_denies_sender_not_in_approvers() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &["telegram:admin"], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "/approve shell").await);
    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Approval-management command denied"));
    assert!(sent[0].contains("telegram:admin"));
}

#[tokio::test]
async fn natural_language_approval_denies_unauthorized_sender_before_direct_promotion() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(
            &[],
            &["telegram:zeta", "telegram:admin"],
            NonCliNaturalLanguageApprovalMode::Direct,
        ),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "approve tool shell").await);

    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Approval-management command denied"));
    assert!(sent[0].contains("Allowed approvers: telegram:admin, telegram:zeta"));
    assert!(!ctx.approval_manager.is_non_cli_session_granted("shell"));
    assert!(
        ctx.approval_manager
            .list_non_cli_pending_requests(Some("sender"), Some("telegram"), Some("chat-1"))
            .is_empty()
    );
}

#[tokio::test]
async fn natural_language_approval_commands_follow_runtime_policy_modes() {
    let tmp = tempfile::tempdir().unwrap();

    let disabled = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Disabled),
    );
    let disabled_channel = Arc::new(RecordingChannel::new());
    let disabled_dyn: Arc<dyn Channel> = disabled_channel.clone();
    assert!(dispatch(&disabled, &disabled_dyn, "approve tool shell").await);
    assert!(
        disabled_channel.sent_contents().await[0]
            .contains("Natural-language approval commands are disabled")
    );

    let request_confirm = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::RequestConfirm),
    );
    let request_channel = Arc::new(RecordingChannel::new());
    let request_dyn: Arc<dyn Channel> = request_channel.clone();
    assert!(dispatch(&request_confirm, &request_dyn, "approve tool shell").await);
    assert!(request_channel.sent_contents().await[0].contains("Approval request created."));
    assert_eq!(
        request_confirm
            .approval_manager
            .list_non_cli_pending_requests(Some("sender"), Some("telegram"), Some("chat-1"))
            .len(),
        1
    );

    let direct = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let direct_channel = Arc::new(RecordingChannel::new());
    let direct_dyn: Arc<dyn Channel> = direct_channel.clone();
    assert!(dispatch(&direct, &direct_dyn, "approve tool shell").await);
    assert!(
        direct_channel.sent_contents().await[0]
            .contains("Approved supervised execution for `shell`")
    );
    assert!(direct.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn channel_specific_natural_language_policy_override_is_used() {
    let tmp = tempfile::tempdir().unwrap();
    let mut autonomy = autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Disabled);
    autonomy
        .non_cli_natural_language_approval_mode_by_channel
        .insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Direct);
    let ctx = context(tmp.path(), &["shell"], autonomy);
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let msg = message_in_channel("telegram", "approve tool shell");

    assert!(dispatch_message(&ctx, &channel_dyn, &msg).await);
    assert!(channel.sent_contents().await[0].contains("Approved supervised execution for `shell`"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn direct_mode_keeps_natural_language_unapprove_as_revoke() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    ctx.approval_manager.grant_non_cli_session("shell");
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "unapprove shell").await);
    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Runtime session grant removed for `shell`: yes."));
    assert!(!ctx.approval_manager.is_non_cli_session_granted("shell"));
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
    assert!(sent.iter().any(|text| text.contains("Available providers:")));
    assert!(sent.iter().any(|text| text.contains("Unknown provider")));
    assert!(sent.iter().any(|text| text.contains("Current model")));
    assert!(sent.iter().any(|text| text.contains("Model switched to `gpt-next`")));
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
async fn task_mode_plain_message_is_consumed_without_reply_channel() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "/task").await);
    assert!(handle_runtime_command_if_needed(&ctx, &message("create a task"), None).await);
}

#[tokio::test]
async fn send_failure_still_consumes_runtime_command() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::failing());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "/approvals").await);
}
