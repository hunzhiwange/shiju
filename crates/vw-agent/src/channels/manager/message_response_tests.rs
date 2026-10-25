// 本文件验证消息响应的草稿、反应、钩子与渠道发送协作。

use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::hooks::{HookHandler, HookResult, HookRunner};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use crate::app::agent::tools::{Tool, ToolResult};

#[derive(Default)]
struct RecordingChannel {
    sent: tokio::sync::Mutex<Vec<String>>,
    finalized: tokio::sync::Mutex<Vec<String>>,
    cancelled: tokio::sync::Mutex<Vec<String>>,
    reactions: tokio::sync::Mutex<Vec<String>>,
    fail_finalize: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "test"
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

    // 假渠道分别记录草稿完成与取消，使响应状态转换可独立断言。
    async fn finalize_draft(
        &self,
        _recipient: &str,
        message_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        self.finalized.lock().await.push(format!("{message_id}:{text}"));
        if self.fail_finalize {
            anyhow::bail!("finalize failed");
        }
        Ok(())
    }

    async fn cancel_draft(&self, _recipient: &str, message_id: &str) -> anyhow::Result<()> {
        self.cancelled.lock().await.push(message_id.to_string());
        Ok(())
    }

    async fn add_reaction(
        &self,
        _recipient: &str,
        _message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        self.reactions.lock().await.push(format!("add:{emoji}"));
        Ok(())
    }

    async fn remove_reaction(
        &self,
        _recipient: &str,
        _message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        self.reactions.lock().await.push(format!("remove:{emoji}"));
        Ok(())
    }
}

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

enum SendingHookMode {
    Cancel,
    Modify(String),
}

struct SendingHook {
    mode: SendingHookMode,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl HookHandler for SendingHook {
    fn name(&self) -> &str {
        "sending-hook"
    }

    async fn on_message_sending(
        &self,
        _channel: String,
        _recipient: String,
        content: String,
    ) -> HookResult<(String, String, String)> {
        match &self.mode {
            SendingHookMode::Cancel => HookResult::Cancel("blocked".to_string()),
            SendingHookMode::Modify(modified) => HookResult::Continue((
                "rewritten-channel".to_string(),
                "rewritten-recipient".to_string(),
                modified.replace("{content}", &content),
            )),
        }
    }
}

struct NamedTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::default())
    }
}

fn message(channel: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg".to_string(),
        sender: "sender".to_string(),
        reply_target: "recipient".to_string(),
        content: "user turn".to_string(),
        channel: channel.to_string(),
        timestamp: 1,
        thread_ts: Some("thread".to_string()),
    }
}

fn route() -> ChannelRouteSelection {
    ChannelRouteSelection {
        provider: "provider".to_string(),
        model: "model".to_string(),
        reasoning_effort: None,
        task_mode_enabled: false,
    }
}

fn context(max_iterations: usize) -> ChannelRuntimeContext {
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
        max_iterations,
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
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn context_with_sending_hook(mode: SendingHookMode) -> ChannelRuntimeContext {
    let mut runner = HookRunner::new();
    runner.register(Box::new(SendingHook { mode }));

    let mut ctx = context(3);
    ctx.hooks = Some(Arc::new(runner));
    ctx
}

fn context_with_tool(tool_name: &'static str) -> ChannelRuntimeContext {
    let mut ctx = context(3);
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(NamedTool { name: tool_name })];
    ctx.tools_registry = Arc::new(tools);
    ctx
}

async fn elapsed_timeout() -> tokio::time::error::Elapsed {
    tokio::time::timeout(std::time::Duration::from_millis(0), std::future::pending::<()>())
        .await
        .expect_err("pending future should time out immediately")
}

#[tokio::test]
async fn completed_success_sends_response_and_appends_history() {
    let ctx = context(3);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("assistant reply".to_string()))),
        "history",
        &[ChatMessage::system("system")],
        1,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(channel.sent.lock().await.as_slice(), ["assistant reply"]);
    let histories = ctx.conversation_histories.lock().unwrap();
    assert_eq!(histories["history"].last().unwrap().content, "assistant reply");
}

#[tokio::test]
async fn completed_success_blocks_canary_leak_before_sending() {
    let ctx = context(3);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("leaked TOKEN-123".to_string()))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        Some("TOKEN-123"),
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(true),
    )
    .await;

    assert_eq!(
        channel.sent.lock().await.as_slice(),
        ["I blocked that response because it attempted to reveal protected internal context."]
    );
    {
        let histories = ctx.conversation_histories.lock().unwrap();
        assert_eq!(
            histories["history"].last().unwrap().content,
            "I blocked that response because it attempted to reveal protected internal context."
        );
    }
    assert_eq!(channel.reactions.lock().await.as_slice(), ["remove:\u{1F440}", "add:\u{2705}"]);
}

#[tokio::test]
async fn completed_success_hook_cancel_suppresses_send_and_history() {
    let ctx = context_with_sending_hook(SendingHookMode::Cancel);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("assistant reply".to_string()))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(channel.sent.lock().await.is_empty());
    assert!(channel.reactions.lock().await.is_empty());
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("history"));
}

#[tokio::test]
async fn completed_success_hook_mutation_is_truncated_before_delivery() {
    let oversized = "x".repeat(CHANNEL_HOOK_MAX_OUTBOUND_CHARS + 32);
    let ctx = context_with_sending_hook(SendingHookMode::Modify(oversized));
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("original".to_string()))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    {
        let sent = channel.sent.lock().await;
        assert_eq!(sent.len(), 1);
        assert!(sent[0].chars().count() <= CHANNEL_HOOK_MAX_OUTBOUND_CHARS);
        assert_ne!(sent[0], "original");
    }
}

#[tokio::test]
async fn completed_success_returns_safe_fallback_for_tool_only_output() {
    let ctx = context_with_tool("shell");
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok(
            r#"{"name":"shell","arguments":{"cmd":"echo hi"}}"#.to_string()
        ))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(
        channel.sent.lock().await.as_slice(),
        [
            "I encountered malformed tool-call output and could not produce a safe reply. Please try again."
        ]
    );
}

#[tokio::test]
async fn completed_success_records_tool_summary_except_for_telegram() {
    let history = vec![ChatMessage::assistant(
        r#"{"tool_calls":[{"function":{"name":"shell","arguments":"{}"}}]}"#,
    )];

    let discord_ctx = context(3);
    handle_llm_result(
        &discord_ctx,
        message("discord"),
        None,
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("done".to_string()))),
        "discord-history",
        &history,
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    let telegram_ctx = context(3);
    handle_llm_result(
        &telegram_ctx,
        message("telegram"),
        None,
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("done".to_string()))),
        "telegram-history",
        &history,
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    let discord_histories = discord_ctx.conversation_histories.lock().unwrap();
    assert_eq!(
        discord_histories["discord-history"].last().unwrap().content,
        "[Used tools: shell]\ndone"
    );
    let telegram_histories = telegram_ctx.conversation_histories.lock().unwrap();
    assert_eq!(telegram_histories["telegram-history"].last().unwrap().content, "done");
}

#[tokio::test]
async fn completed_success_finalizes_draft_and_falls_back_to_send_on_failure() {
    let ctx = context(1);
    let channel = Arc::new(RecordingChannel { fail_finalize: true, ..RecordingChannel::default() });
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Ok("final text".to_string()))),
        "history",
        &[ChatMessage::system("system")],
        1,
        true,
        Instant::now(),
        Some("draft-1"),
        "user turn",
        None,
        "\u{2705}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(channel.finalized.lock().await.as_slice(), ["draft-1:final text"]);
    assert_eq!(channel.sent.lock().await.as_slice(), ["final text"]);
}

#[tokio::test]
async fn cancelled_result_cancels_existing_draft() {
    let ctx = context(1);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Cancelled,
        "history",
        &[],
        0,
        true,
        Instant::now(),
        Some("draft-1"),
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(channel.cancelled.lock().await.as_slice(), ["draft-1"]);
}

#[tokio::test]
async fn cancelled_result_without_channel_leaves_history_unchanged() {
    let ctx = context(1);

    handle_llm_result(
        &ctx,
        message("discord"),
        None,
        &route(),
        LlmExecutionResult::Cancelled,
        "history",
        &[],
        0,
        true,
        Instant::now(),
        Some("draft-1"),
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[tokio::test]
async fn tool_loop_cancelled_error_cancels_existing_draft() {
    let ctx = context(1);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Err(anyhow::Error::new(
            crate::app::agent::agent::loop_::ToolLoopCancelled,
        )))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        Some("draft-1"),
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(channel.cancelled.lock().await.as_slice(), ["draft-1"]);
}

#[tokio::test]
async fn context_window_error_sends_retry_guidance() {
    let ctx = context(1);
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("history".to_string(), vec![ChatMessage::user("old")]);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!("maximum context length reached")))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(channel.sent.lock().await[0].contains("Context window exceeded"));
}

#[tokio::test]
async fn tool_iteration_limit_preserves_history_with_pause_marker() {
    let ctx = context(7);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!(
            "Agent exceeded maximum tool iterations (7)"
        )))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(channel.sent.lock().await[0].contains("tool-iteration limit (7)"));
    let histories = ctx.conversation_histories.lock().unwrap();
    assert!(histories["history"].last().unwrap().content.contains("Task paused"));
}

#[tokio::test]
async fn generic_error_records_failure_marker_and_notifies_channel() {
    let ctx = context(1);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!("provider exploded")))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(channel.sent.lock().await[0].contains("provider exploded"));
    let histories = ctx.conversation_histories.lock().unwrap();
    assert!(histories["history"].last().unwrap().content.contains("Task failed"));
}

#[tokio::test]
async fn vision_capability_error_rolls_back_orphan_user_turn_and_finalizes_draft() {
    let ctx = context(1);
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("history".to_string(), vec![ChatMessage::user("image prompt")]);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Ok(Err(anyhow::Error::new(
            crate::app::agent::providers::ProviderCapabilityError {
                provider: "provider".to_string(),
                capability: "vision".to_string(),
                message: "disabled".to_string(),
            },
        )))),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        Some("draft-1"),
        "image prompt",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("history"));
    let finalized = channel.finalized.lock().await;
    assert_eq!(finalized.len(), 1);
    assert!(finalized[0].contains("draft-1:\u{26A0}\u{FE0F} Error:"));
    assert!(finalized[0].contains("capability=vision"));
}

#[tokio::test]
async fn timeout_records_failure_marker_and_sends_guidance() {
    let ctx = context(2);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Err(elapsed_timeout().await)),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        None,
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(
        channel.sent.lock().await.as_slice(),
        ["\u{26A0}\u{FE0F} Request timed out while waiting for the model. Please try again."]
    );
    let histories = ctx.conversation_histories.lock().unwrap();
    assert!(histories["history"].last().unwrap().content.contains("Task timed out"));
}

#[tokio::test]
async fn timeout_finalizes_existing_draft() {
    let ctx = context(2);
    let channel = Arc::new(RecordingChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    handle_llm_result(
        &ctx,
        message("discord"),
        Some(&channel_dyn),
        &route(),
        LlmExecutionResult::Completed(Err(elapsed_timeout().await)),
        "history",
        &[],
        0,
        true,
        Instant::now(),
        Some("draft-1"),
        "user turn",
        None,
        "\u{26A0}\u{FE0F}",
        &crate::app::agent::security::CanaryGuard::new(false),
    )
    .await;

    assert_eq!(
        channel.finalized.lock().await.as_slice(),
        [
            "draft-1:\u{26A0}\u{FE0F} Request timed out while waiting for the model. Please try again."
        ]
    );
}
