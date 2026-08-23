// 本文件验证消息处理辅助流程中的记忆召回、工具存储与发送记录。

use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::hooks::{HookHandler, HookResult};
use crate::app::agent::memory::{MemoryCategory, MemoryEntry};
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug)]
struct StoredMemoryRecord {
    key: String,
    content: String,
    category: MemoryCategory,
    session_id: Option<String>,
}

#[derive(Default)]
struct RecordingMemory {
    stores: AtomicUsize,
    stored_records: Mutex<Vec<StoredMemoryRecord>>,
    recall_queries: Mutex<Vec<String>>,
    recall_entries: Vec<MemoryEntry>,
}

impl RecordingMemory {
// 召回夹具显式注入排序结果，隔离验证提示上下文拼装。
    fn with_recall_entries(recall_entries: Vec<MemoryEntry>) -> Self {
        Self { recall_entries, ..Self::default() }
    }

    fn stored_records(&self) -> Vec<StoredMemoryRecord> {
        self.stored_records.lock().unwrap().clone()
    }

    fn recall_queries(&self) -> Vec<String> {
        self.recall_queries.lock().unwrap().clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Memory for RecordingMemory {
    fn name(&self) -> &str {
        "recording"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.stores.fetch_add(1, Ordering::SeqCst);
        self.stored_records.lock().unwrap().push(StoredMemoryRecord {
            key: key.to_string(),
            content: content.to_string(),
            category,
            session_id: session_id.map(str::to_string),
        });
        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.recall_queries.lock().unwrap().push(query.to_string());
        Ok(self.recall_entries.clone())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct RecordingChannel {
    sent: tokio::sync::Mutex<Vec<SendMessage>>,
}

impl RecordingChannel {
    async fn sent_messages(&self) -> Vec<SendMessage> {
        self.sent.lock().await.clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
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

struct MutatingHook;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl HookHandler for MutatingHook {
    fn name(&self) -> &str {
        "mutating"
    }

    async fn on_message_received(
        &self,
        mut message: traits::ChannelMessage,
    ) -> HookResult<traits::ChannelMessage> {
        message.content.push_str(" patched");
        HookResult::Continue(message)
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
        HookResult::Cancel("drop".to_string())
    }
}

fn channel_message(content: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        id: "msg-1".to_string(),
        sender: "sender".to_string(),
        reply_target: "recipient".to_string(),
        content: content.to_string(),
        channel: "test".to_string(),
        timestamp: 1,
        thread_ts: Some("thread".to_string()),
    }
}

fn memory_entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        session_id: None,
        score,
    }
}

fn test_context(
    memory: Arc<dyn Memory>,
    hooks: Option<Arc<crate::app::agent::hooks::HookRunner>>,
) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("test-provider".to_string()),
        memory,
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: true,
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

#[test]
fn log_inbound_message_accepts_long_unicode_content() {
    let msg = channel_message(&"你好 ".repeat(80));

    log_inbound_message(&msg);
}

#[tokio::test]
async fn apply_inbound_hooks_returns_original_message_without_hooks() {
    let ctx = test_context(Arc::new(RecordingMemory::default()), None);
    let msg = channel_message("hello");

    let processed = apply_inbound_hooks(&ctx, msg.clone()).await.expect("message should continue");

    assert_eq!(processed.content, msg.content);
}

#[tokio::test]
async fn apply_inbound_hooks_applies_modified_message() {
    let mut runner = crate::app::agent::hooks::HookRunner::new();
    runner.register(Box::new(MutatingHook));
    let ctx = test_context(Arc::new(RecordingMemory::default()), Some(Arc::new(runner)));

    let processed = apply_inbound_hooks(&ctx, channel_message("hello")).await.unwrap();

    assert_eq!(processed.content, "hello patched");
}

#[tokio::test]
async fn apply_inbound_hooks_returns_none_when_hook_cancels() {
    let mut runner = crate::app::agent::hooks::HookRunner::new();
    runner.register(Box::new(CancelHook));
    let ctx = test_context(Arc::new(RecordingMemory::default()), Some(Arc::new(runner)));

    assert!(apply_inbound_hooks(&ctx, channel_message("hello")).await.is_none());
}

#[tokio::test]
async fn apply_semantic_guard_skips_commands_and_missing_runtime_config() {
    let ctx = test_context(Arc::new(RecordingMemory::default()), None);

    assert_eq!(apply_semantic_guard(&ctx, &channel_message(" /help"), None).await, Some(false));
    assert_eq!(
        apply_semantic_guard(&ctx, &channel_message("normal message"), None).await,
        Some(false)
    );
}

#[tokio::test]
async fn apply_semantic_guard_skips_when_runtime_config_file_is_missing() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut ctx = test_context(Arc::new(RecordingMemory::default()), None);
    ctx.provider_runtime_options.vibewindow_dir = Some(temp.path().to_path_buf());

    let result = apply_semantic_guard(&ctx, &channel_message("normal message"), None).await;

    assert_eq!(result, Some(false));
}

#[tokio::test]
async fn apply_semantic_guard_skips_when_runtime_config_is_invalid() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut ctx = test_context(Arc::new(RecordingMemory::default()), None);
    ctx.provider_runtime_options.vibewindow_dir = Some(temp.path().to_path_buf());
    tokio::fs::write(temp.path().join(".vibewindow.json"), "security = [")
        .await
        .expect("write invalid runtime config");

    let result = apply_semantic_guard(&ctx, &channel_message("normal message"), None).await;

    assert_eq!(result, Some(false));
}

#[tokio::test]
async fn apply_semantic_guard_returns_canary_flag_from_valid_disabled_config() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut ctx = test_context(Arc::new(RecordingMemory::default()), None);
    ctx.provider_runtime_options.vibewindow_dir = Some(temp.path().to_path_buf());
    let mut config = Config::default();
    config.security.canary_tokens = true;
    config.security.semantic_guard = false;
    let contents = toml::to_string(&config).expect("config should serialize");
    tokio::fs::write(temp.path().join(".vibewindow.json"), contents)
        .await
        .expect("write runtime config");

    let result = apply_semantic_guard(&ctx, &channel_message("normal message"), None).await;

    assert_eq!(result, Some(true));
}

#[tokio::test]
async fn apply_semantic_guard_enabled_without_available_backend_allows_message() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut ctx = test_context(Arc::new(RecordingMemory::default()), None);
    ctx.provider_runtime_options.vibewindow_dir = Some(temp.path().to_path_buf());
    let mut config = Config::default();
    config.security.canary_tokens = true;
    config.security.semantic_guard = true;
    config.security.semantic_guard_collection = "attack-patterns".to_string();
    config.security.semantic_guard_threshold = 0.8;
    let contents = toml::to_string(&config).expect("config should serialize");
    tokio::fs::write(temp.path().join(".vibewindow.json"), contents)
        .await
        .expect("write runtime config");

    let result = apply_semantic_guard(&ctx, &channel_message("normal message"), None).await;

    assert_eq!(result, Some(true));
}

#[tokio::test]
async fn block_semantic_guard_message_records_warning_in_thread() {
    let channel = Arc::new(RecordingChannel::default());
    let target_channel: Arc<dyn Channel> = channel.clone();
    let msg = channel_message("dangerous prompt");
    let detection = crate::security::semantic_guard::SemanticMatch {
        score: 0.91,
        key: "attack-1".to_string(),
        category: "prompt-leak".to_string(),
    };

    block_semantic_guard_message(&msg, Some(&target_channel), &detection, 0.8, "attack-patterns")
        .await;

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "recipient");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread"));
    assert!(sent[0].content.contains("security.semantic_guard"));
    assert!(sent[0].content.contains("semantic_match=0.91"));
    assert!(sent[0].content.contains("category=prompt-leak"));
}

#[tokio::test]
async fn notify_provider_init_failure_sends_sanitized_threaded_notice() {
    let channel = Arc::new(RecordingChannel::default());
    let target_channel: Arc<dyn Channel> = channel.clone();
    let msg = channel_message("hello");
    let error = anyhow::anyhow!("request failed with token sk-{}", "a".repeat(48));

    notify_provider_init_failure(&msg, Some(&target_channel), "openai", error).await;

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "recipient");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread"));
    assert!(sent[0].content.contains("Failed to initialize provider `openai`"));
    assert!(sent[0].content.contains("Please run `/models`"));
    assert!(sent[0].content.contains("[REDACTED]"));
    assert!(!sent[0].content.contains("sk-"));
}

#[tokio::test]
async fn notify_provider_init_failure_without_channel_is_noop() {
    let msg = channel_message("hello");

    notify_provider_init_failure(&msg, None, "openai", anyhow::anyhow!("boom")).await;
}

#[tokio::test]
async fn maybe_auto_save_message_records_conversation_memory_with_threaded_key() {
    let memory = Arc::new(RecordingMemory::default());
    let ctx = test_context(Arc::clone(&memory) as Arc<dyn Memory>, None);

    maybe_auto_save_message(&ctx, &channel_message("short")).await;
    maybe_auto_save_message(&ctx, &channel_message("this message is long enough")).await;

    assert_eq!(memory.stores.load(Ordering::SeqCst), 1);
    let records = memory.stored_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key, "test_thread_sender_msg-1");
    assert_eq!(records[0].content, "this message is long enough");
    assert_eq!(records[0].category, MemoryCategory::Conversation);
    assert!(records[0].session_id.is_none());
}

#[tokio::test]
async fn maybe_auto_save_message_respects_disabled_flag_and_character_boundary() {
    let memory = Arc::new(RecordingMemory::default());
    let mut ctx = test_context(Arc::clone(&memory) as Arc<dyn Memory>, None);
    let exact_minimum = "好".repeat(AUTOSAVE_MIN_MESSAGE_CHARS);

    maybe_auto_save_message(&ctx, &channel_message(&exact_minimum)).await;
    ctx.auto_save_memory = false;
    maybe_auto_save_message(&ctx, &channel_message("this message is also long enough")).await;

    assert_eq!(memory.stores.load(Ordering::SeqCst), 1);
}

#[test]
fn has_prior_history_reports_only_non_empty_history() {
    let ctx = test_context(Arc::new(RecordingMemory::default()), None);

    assert!(!has_prior_history(&ctx, "sender"));
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("hello")]);

    assert!(has_prior_history(&ctx, "sender"));
}

#[test]
fn has_prior_history_recovers_from_poisoned_lock() {
    let ctx = test_context(Arc::new(RecordingMemory::default()), None);
    let histories = Arc::clone(&ctx.conversation_histories);
    let poison_result = std::panic::catch_unwind(move || {
        let mut guard = histories.lock().unwrap();
        guard.insert("sender".to_string(), vec![ChatMessage::user("hello")]);
        panic!("poison history lock");
    });

    assert!(poison_result.is_err());
    assert!(has_prior_history(&ctx, "sender"));
}

#[tokio::test]
async fn load_prior_turns_normalizes_cached_turns() {
    let ctx = test_context(Arc::new(RecordingMemory::default()), None);
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("one"), ChatMessage::user("two")]);

    let turns = load_prior_turns(&ctx, "sender", "ignored", true).await;

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].content, "one\n\ntwo");
}

#[tokio::test]
async fn load_prior_turns_injects_memory_context_for_first_user_turn() {
    let memory = Arc::new(RecordingMemory::with_recall_entries(vec![memory_entry(
        "prefers_short_answers",
        "Use concise responses.",
        Some(0.8),
    )]));
    let ctx = test_context(Arc::clone(&memory) as Arc<dyn Memory>, None);
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("placeholder")]);

    let turns = load_prior_turns(&ctx, "sender", "What should I do?", false).await;

    assert_eq!(memory.recall_queries(), vec!["What should I do?".to_string()]);
    assert_eq!(turns.len(), 1);
    assert!(turns[0].content.starts_with("[Memory context]\n"));
    assert!(turns[0].content.contains("- prefers_short_answers: Use concise responses.\n"));
    assert!(turns[0].content.ends_with("What should I do?"));
    assert!(!turns[0].content.contains("placeholder"));
}

#[tokio::test]
async fn load_prior_turns_does_not_inject_memory_when_last_turn_is_assistant() {
    let memory = Arc::new(RecordingMemory::with_recall_entries(vec![memory_entry(
        "prefers_short_answers",
        "Use concise responses.",
        None,
    )]));
    let ctx = test_context(Arc::clone(&memory) as Arc<dyn Memory>, None);
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::assistant("already answered")]);

    let turns = load_prior_turns(&ctx, "sender", "What should I do?", false).await;

    assert_eq!(memory.recall_queries(), vec!["What should I do?".to_string()]);
    assert!(turns.is_empty());
}

#[tokio::test]
async fn load_prior_turns_queries_memory_but_keeps_missing_history_empty() {
    let memory = Arc::new(RecordingMemory::with_recall_entries(vec![memory_entry(
        "preference",
        "Remember this.",
        Some(0.9),
    )]));
    let ctx = test_context(Arc::clone(&memory) as Arc<dyn Memory>, None);

    let turns = load_prior_turns(&ctx, "missing", "new question", false).await;

    assert_eq!(memory.recall_queries(), vec!["new question".to_string()]);
    assert!(turns.is_empty());
}
