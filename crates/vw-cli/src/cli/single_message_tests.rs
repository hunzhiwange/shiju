//! CLI 单消息执行链路的集成式单元测试。
//! 可控的 Memory、Provider 和 Observer 替身用于校验记忆召回、上下文拼接与自动保存阈值。
//! 记忆故障应允许对话继续，而 Provider 故障必须上抛且不记录回合完成。

use super::setup::CliSetup;
use super::single_message::run_single_message;
use crate::app::agent::agent::loop_::core::AUTOSAVE_MIN_MESSAGE_CHARS;
use crate::app::agent::config::Config;
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry};
use crate::app::agent::observability::traits::ObserverMetric;
use crate::app::agent::observability::{Observer, ObserverEvent};
use crate::app::agent::providers::Provider;
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoreCall {
    key: String,
    content: String,
    category: MemoryCategory,
    session_id: Option<String>,
}

#[derive(Default)]
struct RecordingMemory {
    entries: Vec<MemoryEntry>,
    fail_store: bool,
    fail_recall: bool,
    stores: Mutex<Vec<StoreCall>>,
    recalls: Mutex<Vec<(String, usize, Option<String>)>>,
}

impl RecordingMemory {
    fn with_entries(entries: Vec<MemoryEntry>) -> Self {
        Self { entries, ..Self::default() }
    }

    fn failing_store_and_recall() -> Self {
        Self { fail_store: true, fail_recall: true, ..Self::default() }
    }

    fn stores(&self) -> Vec<StoreCall> {
        self.stores.lock().expect("store calls lock should be available").clone()
    }

    fn recalls(&self) -> Vec<(String, usize, Option<String>)> {
        self.recalls.lock().expect("recall calls lock should be available").clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Memory for RecordingMemory {
    fn name(&self) -> &str {
        "single-message-test"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.stores.lock().expect("store calls lock should be available").push(StoreCall {
            key: key.to_string(),
            content: content.to_string(),
            category,
            session_id: session_id.map(str::to_string),
        });

        if self.fail_store {
            anyhow::bail!("memory store failed");
        }

        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.recalls.lock().expect("recall calls lock should be available").push((
            query.to_string(),
            limit,
            session_id.map(str::to_string),
        ));

        if self.fail_recall {
            anyhow::bail!("memory recall failed");
        }

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

#[derive(Clone, Debug, PartialEq)]
struct ProviderCall {
    system_prompt: Option<String>,
    message: String,
    model: String,
    temperature: f64,
}

enum ProviderOutcome {
    Reply(String),
    Error(String),
}

struct RecordingProvider {
    outcome: ProviderOutcome,
    calls: Arc<Mutex<Vec<ProviderCall>>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for RecordingProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.lock().expect("provider calls lock should be available").push(ProviderCall {
            system_prompt: system_prompt.map(str::to_string),
            message: message.to_string(),
            model: model.to_string(),
            temperature,
        });

        match &self.outcome {
            ProviderOutcome::Reply(response) => Ok(response.clone()),
            ProviderOutcome::Error(message) => Err(anyhow::anyhow!(message.clone())),
        }
    }
}

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl RecordingObserver {
    fn turn_complete_count(&self) -> usize {
        self.events
            .lock()
            .expect("observer events lock should be available")
            .iter()
            .filter(|event| matches!(event, ObserverEvent::TurnComplete))
            .count()
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().expect("observer events lock should be available").push(event.clone());
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "single-message-test"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn provider(outcome: ProviderOutcome) -> (Box<dyn Provider>, Arc<Mutex<Vec<ProviderCall>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let provider = RecordingProvider { outcome, calls: Arc::clone(&calls) };
    (Box::new(provider), calls)
}

fn setup(
    provider: Box<dyn Provider>,
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
) -> CliSetup {
    CliSetup {
        observer,
        mem: memory,
        provider,
        tools_registry: Vec::new(),
        provider_name: "test-provider".to_string(),
        model_name: "test-model".to_string(),
        system_prompt: "test-system-prompt".to_string(),
        approval_manager: None,
        channel_name: "cli",
    }
}

fn memory_entry(key: &str, content: &str) -> MemoryEntry {
    MemoryEntry {
        id: key.to_string(),
        key: key.to_string(),
        content: content.to_string(),
        category: MemoryCategory::Core,
        timestamp: "test-time".to_string(),
        session_id: None,
        score: Some(1.0),
    }
}

#[tokio::test]
async fn run_single_message_autosaves_at_character_boundary_and_returns_provider_response() {
    let config = Config::default();
    let message = "界".repeat(AUTOSAVE_MIN_MESSAGE_CHARS);
    let memory = Arc::new(RecordingMemory::default());
    let observer = Arc::new(RecordingObserver::default());
    let (provider, provider_calls) =
        provider(ProviderOutcome::Reply("provider response".to_string()));
    let setup = setup(provider, memory.clone(), observer.clone());

    let response = run_single_message(&config, &setup, message.clone(), 0.25)
        .await
        .expect("provider response should be returned");

    assert_eq!(response, "provider response");
    let stores = memory.stores();
    assert_eq!(stores.len(), 1);
    assert!(stores[0].key.starts_with("user_msg_"));
    assert_eq!(stores[0].content, message);
    assert_eq!(stores[0].category, MemoryCategory::Conversation);
    assert_eq!(stores[0].session_id, None);
    assert_eq!(memory.recalls(), vec![(message.clone(), 5, None)]);

    let calls = provider_calls.lock().expect("provider calls lock should be available");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].system_prompt.as_deref(), Some("test-system-prompt"));
    assert_eq!(calls[0].model, "test-model");
    assert_eq!(calls[0].temperature, 0.25);
    assert!(calls[0].message.starts_with('['));
    assert!(calls[0].message.ends_with(&format!("] {message}")));
    assert!(!calls[0].message.contains("[Memory context]"));
    assert_eq!(observer.turn_complete_count(), 1);
}

#[tokio::test]
async fn run_single_message_prepends_memory_context_when_autosave_is_disabled() {
    let mut config = Config::default();
    config.memory.auto_save = false;
    let message = "use the remembered preference".to_string();
    let memory = Arc::new(RecordingMemory::with_entries(vec![memory_entry(
        "preference",
        "keep answers concise",
    )]));
    let observer = Arc::new(RecordingObserver::default());
    let (provider, provider_calls) = provider(ProviderOutcome::Reply("concise reply".to_string()));
    let setup = setup(provider, memory.clone(), observer.clone());

    let response = run_single_message(&config, &setup, message.clone(), 0.5)
        .await
        .expect("provider response should be returned");

    assert_eq!(response, "concise reply");
    assert!(memory.stores().is_empty());
    assert_eq!(memory.recalls(), vec![(message.clone(), 5, None)]);
    let calls = provider_calls.lock().expect("provider calls lock should be available");
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].message.starts_with("[Memory context]\n- preference: keep answers concise\n\n[")
    );
    assert!(calls[0].message.ends_with(&format!("] {message}")));
    assert_eq!(observer.turn_complete_count(), 1);
}

#[tokio::test]
async fn run_single_message_skips_autosave_below_character_threshold() {
    let config = Config::default();
    let message = "x".repeat(AUTOSAVE_MIN_MESSAGE_CHARS - 1);
    let memory = Arc::new(RecordingMemory::default());
    let observer = Arc::new(RecordingObserver::default());
    let (provider, _provider_calls) = provider(ProviderOutcome::Reply("response".to_string()));
    let setup = setup(provider, memory.clone(), observer);

    let response = run_single_message(&config, &setup, message.clone(), 0.0)
        .await
        .expect("provider response should be returned");

    assert_eq!(response, "response");
    assert!(memory.stores().is_empty());
    assert_eq!(memory.recalls(), vec![(message, 5, None)]);
}

#[tokio::test]
async fn run_single_message_continues_when_memory_store_and_recall_fail() {
    let config = Config::default();
    let message = "m".repeat(AUTOSAVE_MIN_MESSAGE_CHARS);
    let memory = Arc::new(RecordingMemory::failing_store_and_recall());
    let observer = Arc::new(RecordingObserver::default());
    let (provider, provider_calls) =
        provider(ProviderOutcome::Reply("fallback response".to_string()));
    let setup = setup(provider, memory.clone(), observer.clone());

    let response = run_single_message(&config, &setup, message.clone(), 0.75)
        .await
        .expect("memory failures should not stop the provider call");

    assert_eq!(response, "fallback response");
    assert_eq!(memory.stores().len(), 1);
    assert_eq!(memory.recalls(), vec![(message.clone(), 5, None)]);
    let calls = provider_calls.lock().expect("provider calls lock should be available");
    assert_eq!(calls.len(), 1);
    assert!(!calls[0].message.contains("[Memory context]"));
    assert!(calls[0].message.ends_with(&format!("] {message}")));
    assert_eq!(observer.turn_complete_count(), 1);
}

#[tokio::test]
async fn run_single_message_propagates_provider_error_without_recording_turn_complete() {
    let mut config = Config::default();
    config.memory.auto_save = false;
    let memory = Arc::new(RecordingMemory::default());
    let observer = Arc::new(RecordingObserver::default());
    let (provider, provider_calls) =
        provider(ProviderOutcome::Error("provider unavailable".to_string()));
    let setup = setup(provider, memory, observer.clone());

    let error = run_single_message(&config, &setup, "hello".to_string(), 1.0)
        .await
        .expect_err("provider error should be propagated");

    assert_eq!(error.to_string(), "provider unavailable");
    assert_eq!(provider_calls.lock().expect("provider calls lock should be available").len(), 1);
    assert_eq!(observer.turn_complete_count(), 0);
}
