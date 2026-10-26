//! 测试查询引擎的意图路由、上下文构建与执行结果整合。
//! 脚本化供应商、可控记忆和记录观察器共同验证失败回滚及用量饱和计算。

use super::*;

use crate::app::agent::config::Config;
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry};
use crate::app::agent::observability::{NoopObserver, Observer, ObserverEvent};
use crate::app::agent::providers::traits::TokenUsage;
use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider};
use crate::app::agent::tools::ToolResult;
use crate::observability::traits::ObserverMetric;
use crate::providers::traits::ProviderCapabilities;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeMemory {
    entries: Vec<MemoryEntry>,
}

impl FakeMemory {
    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self { entries }
    }
}

fn memory_entry(key: &str, content: &str, score: Option<f64>) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "now".to_string(),
        session_id: None,
        score,
    }
}

#[async_trait::async_trait]
impl Memory for FakeMemory {
    fn name(&self) -> &str {
        "fake"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(self.entries.clone())
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

enum ProviderStep {
    Response(ChatResponse),
    Error(&'static str),
}

#[derive(Default)]
struct ProviderState {
    steps: Mutex<VecDeque<ProviderStep>>,
    system_error: Option<&'static str>,
    seen_messages: Mutex<Vec<Vec<ChatMessage>>>,
    seen_models: Mutex<Vec<String>>,
    seen_temperatures: Mutex<Vec<f64>>,
}

#[derive(Clone, Default)]
struct ScriptedProvider {
    state: Arc<ProviderState>,
}

impl ScriptedProvider {
    fn with_steps(steps: Vec<ProviderStep>) -> Self {
        Self {
            state: Arc::new(ProviderState {
                steps: Mutex::new(steps.into_iter().collect()),
                ..ProviderState::default()
            }),
        }
    }

    fn with_system_error(error: &'static str) -> Self {
        Self {
            state: Arc::new(ProviderState {
                system_error: Some(error),
                ..ProviderState::default()
            }),
        }
    }

    fn seen_messages(&self) -> Vec<Vec<ChatMessage>> {
        self.state.seen_messages.lock().unwrap().clone()
    }

    fn seen_models(&self) -> Vec<String> {
        self.state.seen_models.lock().unwrap().clone()
    }

    fn seen_temperatures(&self) -> Vec<f64> {
        self.state.seen_temperatures.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Provider for ScriptedProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: true, vision: false }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        if let Some(error) = self.state.system_error {
            anyhow::bail!("{error}");
        }
        Ok("summary".to_string())
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.state.seen_messages.lock().unwrap().push(request.messages.to_vec());
        self.state.seen_models.lock().unwrap().push(model.to_string());
        self.state.seen_temperatures.lock().unwrap().push(temperature);

        match self.state.steps.lock().unwrap().pop_front() {
            Some(ProviderStep::Response(response)) => Ok(response),
            Some(ProviderStep::Error(message)) => anyhow::bail!("{message}"),
            None => anyhow::bail!("scripted provider exhausted"),
        }
    }
}

#[derive(Default)]
struct CapturingObserver {
    events: Mutex<Vec<String>>,
    metrics: AtomicUsize,
    flushes: AtomicUsize,
}

impl Observer for CapturingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().unwrap().push(format!("{event:?}"));
    }

    fn record_metric(&self, _metric: &ObserverMetric) {
        self.metrics.fetch_add(1, Ordering::SeqCst);
    }

    fn flush(&self) {
        self.flushes.fetch_add(1, Ordering::SeqCst);
    }

    fn name(&self) -> &str {
        "capturing"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn chat_text(text: &str, usage: Option<TokenUsage>) -> ChatResponse {
    ChatResponse {
        text: Some(text.to_string()),
        tool_calls: Vec::new(),
        usage,
        reasoning_content: None,
    }
}

fn chat_tool_call(id: &str) -> ChatResponse {
    ChatResponse {
        text: None,
        tool_calls: vec![crate::app::agent::providers::ToolCall {
            id: id.to_string(),
            name: "hook_probe".to_string(),
            arguments: "{}".to_string(),
        }],
        usage: None,
        reasoning_content: None,
    }
}

struct HookProbeTool;

#[async_trait::async_trait]
impl Tool for HookProbeTool {
    fn name(&self) -> &str {
        "hook_probe"
    }

    fn description(&self) -> &str {
        "Records one deterministic tool completion for hook tests"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

fn command_logger_runtime(enabled: bool) -> crate::app::agent::hooks::HookRuntime {
    let mut config = Config::default();
    config.hooks.enabled = enabled;
    config.hooks.builtin.command_logger = true;
    config.hooks.plugins.enabled = false;
    crate::app::agent::hooks::build_hook_runtime(&config).expect("hook runtime")
}

// 查询引擎夹具关闭无关外部能力，只保留可记录供应商与可控记忆。
fn test_engine(provider: ScriptedProvider, memory: Arc<dyn Memory>) -> QueryEngine {
    QueryEngine {
        session_id: format!("query-engine-{}", uuid::Uuid::new_v4()),
        reliability: crate::config::ReliabilityConfig { provider_retries: 0, ..Default::default() },
        history: Vec::new(),
        request_system_prompt: "system prompt".to_string(),
        tools_registry: Vec::new(),
        observer: Arc::new(RecordingObserver::new(Arc::new(NoopObserver))),
        provider: Arc::new(provider),
        memory,
        provider_name: "scripted".to_string(),
        model_name: "model-a".to_string(),
        default_temperature: 0.25,
        multimodal_config: crate::app::agent::config::MultimodalConfig::default(),
        compact_context: false,
        model_limit: crate::app::agent::provider::provider::ModelLimit {
            context: 0,
            input: None,
            output: 0,
        },
        max_iterations: 2,
        max_history_messages: 10,
        min_memory_relevance_score: 0.5,
        turn_count: 0,
        timezone: chrono_tz::Asia::Shanghai,
        hook_runtime: None,
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the shared scripted model boundary.
async fn query_engine_uses_hook_runtime_for_primary_and_retry_turns() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("summary".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let runtime = command_logger_runtime(true);
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_tool_call("primary-tool")),
        ProviderStep::Error("prompt too long"),
        ProviderStep::Response(chat_tool_call("retry-tool")),
        ProviderStep::Response(chat_text("recovered", None)),
    ]);
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.tools_registry = vec![Box::new(HookProbeTool)];
    engine.compact_context = true;
    engine.hook_runtime = Some(runtime.clone());
    engine.history.push(ChatMessage::user("prior task"));

    let response = engine.submit_message("exercise primary and retry").await.unwrap();

    assert_eq!(response, "recovered");
    assert_eq!(runtime.audit_store().entries().len(), 2);

    let disabled = command_logger_runtime(false);
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_tool_call("disabled-tool")),
        ProviderStep::Response(chat_text("disabled", None)),
    ]);
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.tools_registry = vec![Box::new(HookProbeTool)];
    engine.hook_runtime = Some(disabled.clone());

    assert_eq!(engine.submit_message("hooks disabled").await.unwrap(), "disabled");
    assert!(disabled.audit_store().entries().is_empty());
}

#[tokio::test]
async fn agent_and_query_engine_use_ephemeral_query_engine_context_each_turn() {
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_text("first", None)),
        ProviderStep::Response(chat_text("second", None)),
    ]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));

    engine.submit_message("one").await.expect("first turn should succeed");
    engine.submit_message("two").await.expect("second turn should succeed");

    let requests = provider_handle.seen_messages();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request[0].role, "user");
        assert!(request[0].content.starts_with("<system-reminder>"));
    }
    assert!(
        engine
            .history_snapshot()
            .iter()
            .all(|message| !message.content.contains("<system-reminder>"))
    );
    assert!(engine.history_snapshot().iter().all(|message| message.role != "system"));
}

#[tokio::test]
async fn agent_and_query_engine_keep_context_ephemeral_without_trimming_history() {
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Response(chat_text("first", None)),
        ProviderStep::Response(chat_text("second", None)),
    ]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.max_history_messages = 1;

    engine.submit_message("one").await.expect("first turn should succeed");
    engine.submit_message("two").await.expect("second turn should succeed");

    let requests = provider_handle.seen_messages();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request[0].content.starts_with("<system-reminder>")));
    let history = engine.history_snapshot();
    assert_eq!(history.len(), 4);
    assert!(history[0].content.contains("one"));
    assert_eq!(history[3].content, "second");
    assert!(history.iter().all(|message| !message.content.contains("<system-reminder>")));
}

#[test]
fn query_engine_usage_totals_billable_tokens() {
    let usage = QueryEngineUsage {
        input_tokens: 10,
        output_tokens: 7,
        cached_tokens: 3,
        reasoning_tokens: 99,
        llm_calls: 2,
    };

    assert_eq!(usage.total_tokens(), 20);
    assert_eq!(usage.as_ui_token_usage().input_tokens, 10);
    assert_eq!(usage.as_ui_token_usage().reasoning_tokens, 99);
}

#[test]
fn query_engine_usage_total_saturates_on_overflow() {
    let usage = QueryEngineUsage {
        input_tokens: i64::MAX,
        output_tokens: 1,
        cached_tokens: 1,
        reasoning_tokens: 0,
        llm_calls: 0,
    };

    assert_eq!(usage.total_tokens(), i64::MAX);
}

#[test]
fn recording_observer_accumulates_successful_llm_usage_and_forwards_calls() {
    let inner = Arc::new(CapturingObserver::default());
    let observer = RecordingObserver::new(inner.clone());

    observer.record_event(&ObserverEvent::LlmResponse {
        provider: "p".to_string(),
        model: "m".to_string(),
        duration: std::time::Duration::from_millis(1),
        success: true,
        error_message: None,
        input_tokens: Some(5),
        output_tokens: Some(7),
        cached_tokens: Some(11),
        reasoning_tokens: Some(13),
    });
    observer.record_event(&ObserverEvent::LlmResponse {
        provider: "p".to_string(),
        model: "m".to_string(),
        duration: std::time::Duration::from_millis(1),
        success: false,
        error_message: Some("failed".to_string()),
        input_tokens: Some(100),
        output_tokens: Some(100),
        cached_tokens: Some(100),
        reasoning_tokens: Some(100),
    });
    observer.record_metric(&ObserverMetric::QueueDepth(3));
    observer.flush();

    let usage = observer.usage_snapshot();
    assert_eq!(usage.llm_calls, 1);
    assert_eq!(usage.input_tokens, 5);
    assert_eq!(usage.output_tokens, 7);
    assert_eq!(usage.cached_tokens, 11);
    assert_eq!(usage.reasoning_tokens, 13);
    assert_eq!(observer.name(), "capturing");
    assert!(observer.as_any().is::<RecordingObserver>());
    assert_eq!(inner.events.lock().unwrap().len(), 2);
    assert_eq!(inner.metrics.load(Ordering::SeqCst), 1);
    assert_eq!(inner.flushes.load(Ordering::SeqCst), 1);
}

#[test]
fn recording_observer_records_success_with_missing_usage_as_zero() {
    let observer = RecordingObserver::new(Arc::new(NoopObserver));

    observer.record_event(&ObserverEvent::LlmResponse {
        provider: "p".to_string(),
        model: "m".to_string(),
        duration: std::time::Duration::from_millis(1),
        success: true,
        error_message: None,
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });

    let usage = observer.usage_snapshot();
    assert_eq!(usage.llm_calls, 1);
    assert_eq!(usage.total_tokens(), 0);
    assert_eq!(usage.reasoning_tokens, 0);
}

#[test]
fn recording_observer_saturates_oversized_token_counts() {
    let observer = RecordingObserver::new(Arc::new(NoopObserver));

    observer.record_event(&ObserverEvent::LlmResponse {
        provider: "p".to_string(),
        model: "m".to_string(),
        duration: std::time::Duration::from_millis(1),
        success: true,
        error_message: None,
        input_tokens: Some(u64::MAX),
        output_tokens: Some(u64::MAX),
        cached_tokens: Some(u64::MAX),
        reasoning_tokens: Some(u64::MAX),
    });

    let usage = observer.usage_snapshot();
    assert_eq!(usage.llm_calls, 1);
    assert_eq!(usage.input_tokens, i64::MAX);
    assert_eq!(usage.output_tokens, i64::MAX);
    assert_eq!(usage.cached_tokens, i64::MAX);
    assert_eq!(usage.reasoning_tokens, i64::MAX);
}

#[test]
fn session_state_counts_non_system_messages_and_remaining_budget() {
    let provider = ScriptedProvider::default();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("one"),
        ChatMessage::assistant("two"),
        ChatMessage::tool("three"),
    ];
    engine.max_history_messages = 2;
    engine.max_iterations = 4;
    engine.turn_count = 9;
    engine.observer.replace_usage(QueryEngineUsage {
        input_tokens: 1,
        output_tokens: 2,
        cached_tokens: 3,
        reasoning_tokens: 4,
        llm_calls: 5,
    });

    let state = engine.session_state();

    assert_eq!(state.turn_count, 9);
    assert_eq!(state.usage.total_tokens(), 6);
    assert_eq!(state.budget.max_iterations, 4);
    assert_eq!(state.budget.max_history_messages, 2);
    assert_eq!(state.budget.non_system_messages, 3);
    assert_eq!(state.budget.remaining_history_messages, 0);
}

#[test]
fn snapshot_replace_and_restore_round_trip_history_turns_and_usage() {
    let provider = ScriptedProvider::default();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.history.push(ChatMessage::user("original"));
    engine.turn_count = 2;
    engine.observer.replace_usage(QueryEngineUsage {
        input_tokens: 3,
        output_tokens: 4,
        cached_tokens: 5,
        reasoning_tokens: 6,
        llm_calls: 7,
    });

    let snapshot = engine.snapshot();
    engine.replace_history(vec![ChatMessage::system("replacement")]);
    engine.turn_count = 0;
    engine.observer.replace_usage(QueryEngineUsage::default());

    engine.restore_snapshot(snapshot);

    assert_eq!(engine.turn_count(), 2);
    assert_eq!(engine.history_snapshot().last().unwrap().content, "original");
    assert_eq!(engine.session_state().usage.llm_calls, 7);
}

#[tokio::test]
async fn submit_message_success_enriches_history_and_records_usage() {
    let usage = TokenUsage {
        input_tokens: Some(5),
        output_tokens: Some(7),
        cached_tokens: Some(2),
        reasoning_tokens: Some(3),
    };
    let provider = ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text(
        "final answer",
        Some(usage),
    ))]);
    let provider_handle = provider.clone();
    let memory = Arc::new(FakeMemory::with_entries(vec![memory_entry(
        "style",
        "prefer concise replies",
        Some(0.9),
    )]));
    let mut engine = test_engine(provider, memory);

    let result = engine.submit_message("hello").await.expect("submit should succeed");

    assert_eq!(result, "final answer");
    assert_eq!(engine.turn_count(), 1);
    let history = engine.history_snapshot();
    assert_eq!(history.len(), 3);
    assert_eq!(history[1].role, "user");
    assert!(history[1].content.contains("[Memory context]"));
    assert!(history[1].content.contains("prefer concise replies"));
    assert!(history[1].content.contains("hello"));
    assert_eq!(history[2].content, "final answer");

    let state = engine.session_state();
    assert_eq!(state.usage.llm_calls, 1);
    assert_eq!(state.usage.total_tokens(), 14);
    assert_eq!(state.usage.reasoning_tokens, 3);
    assert_eq!(provider_handle.seen_models(), vec!["model-a".to_string()]);
    assert_eq!(provider_handle.seen_temperatures(), vec![0.25]);
    assert!(provider_handle.seen_messages()[0][1].content.contains("hello"));
}

#[tokio::test]
async fn submit_message_without_memory_context_uses_timestamp_prefix() {
    let provider =
        ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text("final answer", None))]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));

    let result = engine.submit_message("plain request").await.expect("submit should succeed");

    assert_eq!(result, "final answer");
    let seen = provider_handle.seen_messages();
    let user_message = &seen[0][1].content;
    assert!(!user_message.contains("[Memory context]"));
    assert!(user_message.starts_with('['));
    assert!(user_message.contains("] plain request"));
}

#[tokio::test]
async fn submit_message_timestamp_uses_configured_timezone() {
    let provider =
        ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text("final answer", None))]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.timezone = chrono_tz::Asia::Shanghai;

    engine.submit_message("plain request").await.expect("submit should succeed");

    let seen = provider_handle.seen_messages();
    let user_message = &seen[0][1].content;
    assert!(user_message.contains("+08:00 (Asia/Shanghai)"));
    assert!(user_message.ends_with("] plain request"));
}

#[tokio::test]
async fn new_rejects_invalid_programmatic_timezone() {
    let mut config = Config::default();
    config.app_ui.system_settings.timezone = "Mars/Olympus_Mons".to_string();
    let provider: Arc<dyn Provider> = Arc::new(ScriptedProvider::default());

    let error = match QueryEngine::new_with_provider(config, "session", Some(provider)).await {
        Ok(_) => panic!("invalid timezone should be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("invalid IANA timezone: Mars/Olympus_Mons"));
}

#[tokio::test]
async fn submit_message_success_preserves_raw_history_after_turn() {
    let provider =
        ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text("new answer", None))]);
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.max_history_messages = 1;
    engine.history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("old request"),
        ChatMessage::assistant("old answer"),
    ];

    let result = engine.submit_message("new request").await.expect("submit should succeed");

    assert_eq!(result, "new answer");
    let history = engine.history_snapshot();
    assert_eq!(history.len(), 5);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].content, "old request");
    assert_eq!(history[4].content, "new answer");
    let state = engine.session_state();
    assert_eq!(state.budget.non_system_messages, 4);
    assert_eq!(state.budget.remaining_history_messages, 0);
}

#[tokio::test]
async fn submit_message_error_rolls_back_history_and_turn_count() {
    let provider = ScriptedProvider::with_steps(vec![ProviderStep::Error("boom")]);
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.history.push(ChatMessage::assistant("previous"));
    let before = engine.history_snapshot();

    let result = engine.submit_message("hello").await;

    assert!(result.is_err());
    assert_eq!(engine.turn_count(), 0);
    assert_eq!(engine.history_snapshot().len(), before.len());
    assert_eq!(engine.history_snapshot()[0].content, "previous");
    assert_eq!(engine.session_state().usage.llm_calls, 0);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the shared scripted model boundary.
async fn submit_message_compacts_before_the_primary_request() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("summary".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let provider =
        ScriptedProvider::with_steps(vec![ProviderStep::Response(chat_text("done", None))]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.compact_context = true;
    engine.max_history_messages = 100;
    engine.model_limit = crate::app::agent::provider::provider::ModelLimit {
        context: 8192,
        input: Some(8192),
        output: 0,
    };
    engine.history = vec![ChatMessage::system("system")];
    engine.history.push(ChatMessage::user("message-0 中文😀".repeat(5000)));

    engine.submit_message("latest").await.expect("primary request should succeed");

    let primary_request = provider_handle.seen_messages().pop().expect("primary request");
    assert!(primary_request.iter().any(|message| message.content.contains("[Compaction summary]")));
    assert!(!primary_request.iter().any(|message| message.content.contains("message-0")));
    assert!(engine.history_snapshot().iter().any(|message| message.content.contains("message-0")));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the shared scripted model boundary.
async fn prompt_too_long_compacts_and_retries_once() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("summary".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let provider = ScriptedProvider::with_steps(vec![
        ProviderStep::Error("prompt too long"),
        ProviderStep::Response(chat_text("recovered", None)),
    ]);
    let provider_handle = provider.clone();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.compact_context = true;
    engine.max_history_messages = 21;
    engine.history = vec![ChatMessage::system("system")];
    for index in 0..23 {
        engine.history.push(ChatMessage::user(format!("message-{index}")));
    }

    let response = engine.submit_message("latest").await.expect("one retry should recover");

    assert_eq!(response, "recovered");
    assert_eq!(provider_handle.seen_messages().len(), 2);
    assert!(
        provider_handle.seen_messages()[1]
            .iter()
            .any(|message| message.content.contains("[Compaction summary]"))
    );
    assert!(engine.history_snapshot().iter().any(|message| message.content == "message-0"));
    assert!(
        !engine
            .history_snapshot()
            .iter()
            .any(|message| message.content.contains("[Compaction summary]"))
    );
}

#[tokio::test]
async fn govern_history_does_not_apply_message_count_trimming() {
    let provider = ScriptedProvider::default();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.max_history_messages = 2;
    engine.history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("old"),
        ChatMessage::assistant("middle"),
        ChatMessage::user("new"),
    ];

    engine.govern_history().await;

    let history = engine.history_snapshot();
    assert_eq!(
        history.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(),
        vec!["system", "old", "middle", "new"]
    );
}

#[tokio::test]
async fn govern_history_preserves_history_after_a_response() {
    let provider = ScriptedProvider::default();
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.compact_context = true;
    engine.max_history_messages = 21;
    engine.history = vec![ChatMessage::system("system")];
    for index in 0..23 {
        engine.history.push(ChatMessage::user(format!("message-{index}")));
    }

    engine.govern_history().await;

    let history = engine.history_snapshot();
    assert_eq!(history.len(), 24);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].content, "message-0");
    assert_eq!(history.last().unwrap().content, "message-22");
}

#[tokio::test]
async fn govern_history_never_uses_local_summary_fallback() {
    let provider = ScriptedProvider::with_system_error("summary failed");
    let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
    engine.compact_context = true;
    engine.max_history_messages = 21;
    engine.history = vec![ChatMessage::system("system")];
    for index in 0..23 {
        engine.history.push(ChatMessage::user(format!("message-{index}")));
    }

    engine.govern_history().await;

    let history = engine.history_snapshot();
    assert_eq!(history.len(), 24);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].content, "message-0");
    assert_eq!(history.last().unwrap().content, "message-22");
}

#[test]
fn build_tool_descriptions_tracks_optional_tools() {
    let mut config = Config::default();
    let base = build_tool_descriptions(&config);
    assert!(base.iter().any(|(name, _)| *name == "bash"));
    assert!(!base.iter().any(|(name, _)| *name == "Browser"));
    let agent_tool = base
        .iter()
        .find_map(|(name, description)| (*name == "AgentTool").then_some(*description))
        .expect("default eligible agents should expose AgentTool");
    for expected in [
        "complex",
        "simple",
        "parallel",
        "background",
        "run",
        "status",
        "wait",
        "stop",
        "synthesize",
    ] {
        assert!(agent_tool.contains(expected), "missing '{expected}': {agent_tool}");
    }

    config.browser.enabled = true;
    config.browser.browser_open = "disable".to_string();
    config.web_fetch.enabled = true;
    config.web_search.enabled = true;

    let descriptions = build_tool_descriptions(&config);
    assert!(descriptions.iter().any(|(name, _)| *name == "Browser"));
    assert!(!descriptions.iter().any(|(name, _)| *name == "BrowserOpen"));
    assert!(descriptions.iter().any(|(name, _)| *name == "WebFetch"));
    assert!(descriptions.iter().any(|(name, _)| *name == "WebSearch"));
}

#[test]
fn build_tool_descriptions_includes_browser_open_when_enabled() {
    let mut config = Config::default();
    config.browser.enabled = true;
    config.browser.browser_open = "system".to_string();

    let descriptions = build_tool_descriptions(&config);

    assert!(descriptions.iter().any(|(name, _)| *name == "BrowserOpen"));
    assert!(descriptions.iter().any(|(name, _)| *name == "Browser"));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the scripted model boundary.
async fn repeated_compaction_replays_latest_checkpoint_against_raw_history() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    let mut engine = test_engine(ScriptedProvider::default(), Arc::new(FakeMemory::default()));
    engine.compact_context = true;
    engine.history = vec![ChatMessage::user("original"), ChatMessage::user("next")];
    for summary in ["first checkpoint", "second checkpoint"] {
        crate::session::processor::enqueue_test_llm_stream(vec![
            crate::session::llm::StreamEvent::Delta(summary.into()),
            crate::session::llm::StreamEvent::Done {
                usage: Default::default(),
                finish_reason: Some("stop".into()),
            },
        ]);
        let context = engine.compact_before_primary_request(true).await.unwrap();
        assert!(context[0].content.contains(summary));
        engine.history.push(ChatMessage::user("continue"));
    }
    let requests = crate::session::processor::take_test_llm_messages(&engine.session_id);
    assert_eq!(requests.len(), 2);
    assert!(requests[1].messages[0]["content"].as_str().unwrap().contains("first checkpoint"));
    engine.compact_context = false;
    let restored = engine.compact_before_primary_request(false).await.unwrap();
    assert!(restored[0].content.contains("second checkpoint"));
    assert_eq!(engine.history[0].content, "original");
    assert_eq!(engine.history.len(), 4);
}
