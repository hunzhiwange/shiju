//! 测试代理运行入口的消息编排、错误传播与可观测事件记录。

use super::core::Agent;
use super::run::{run, run_with_agent_factory};
use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::{Observer, ObserverEvent};
use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::any::Any;
use std::sync::{Arc, Mutex};

struct StaticProvider;

struct FailingProvider;

#[derive(Clone)]
struct MessageRecordingProvider {
    messages: Arc<Mutex<Vec<Vec<ChatMessage>>>>,
}

impl MessageRecordingProvider {
    fn new() -> Self {
        Self { messages: Arc::new(Mutex::new(Vec::new())) }
    }

    fn messages(&self) -> Vec<Vec<ChatMessage>> {
        self.messages.lock().unwrap().clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok("unused".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some("agent response".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for FailingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok("unused".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        Err(anyhow!("provider failed"))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for MessageRecordingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> Result<String> {
        Ok("unused".to_string())
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        self.messages.lock().unwrap().push(request.messages.to_vec());
        Ok(ChatResponse {
            text: Some("agent response".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct RecordingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl RecordingObserver {
    fn new() -> Self {
        Self { events: Mutex::new(Vec::new()) }
    }

    fn events(&self) -> Vec<ObserverEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().unwrap().push(event.clone());
    }

    fn record_metric(&self, _metric: &crate::app::agent::observability::traits::ObserverMetric) {}

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn agent_for_config(config: &Config, observer: Arc<RecordingObserver>) -> Result<Agent> {
    agent_for_config_with_provider(config, observer, Box::new(StaticProvider))
}

fn agent_for_config_with_provider(
    config: &Config,
    observer: Arc<RecordingObserver>,
    provider: Box<dyn Provider>,
) -> Result<Agent> {
    Agent::builder()
        .provider(provider)
        .tools(Vec::new())
        .memory(Arc::new(NoneMemory::new()))
        .observer(observer)
        .model_name(
            config
                .default_model
                .clone()
                .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string()),
        )
        .temperature(config.default_temperature)
        .workspace_dir(std::env::temp_dir())
        .build()
}

fn has_agent_start(
    events: &[ObserverEvent],
    expected_provider: &str,
    expected_model: &str,
) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            ObserverEvent::AgentStart { provider, model }
                if provider == expected_provider && model == expected_model
        )
    })
}

fn has_agent_end(events: &[ObserverEvent], expected_provider: &str, expected_model: &str) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            ObserverEvent::AgentEnd { provider, model, tokens_used: None, cost_usd: None, .. }
                if provider == expected_provider && model == expected_model
        )
    })
}

#[tokio::test]
async fn run_rejects_missing_message_before_starting_agent() {
    let error = run(Config::default(), None, None, None, 0.7).await.unwrap_err();

    assert!(error.to_string().contains("provide a message"));
}

#[tokio::test]
async fn run_propagates_default_agent_factory_error_for_unknown_provider() {
    let mut config = Config::default();
    config.memory.backend = "none".to_string();
    config.default_provider = Some("task-0034-unknown-provider".to_string());
    config.workspace_dir =
        std::env::temp_dir().join(format!("vibe-window-task-0034-{}", std::process::id()));

    let error = run(config, Some("hello".to_string()), None, None, 0.7)
        .await
        .expect_err("unknown provider should fail through the public run entrypoint");

    assert!(error.to_string().contains("Unknown provider: task-0034-unknown-provider"));
}

#[tokio::test]
async fn run_applies_overrides_and_returns_agent_response() {
    let observer = Arc::new(RecordingObserver::new());
    let observer_for_agent = Arc::clone(&observer);
    let captured_config = Arc::new(Mutex::new(None::<Config>));
    let captured_config_for_factory = Arc::clone(&captured_config);

    let response = run_with_agent_factory(
        Config::default(),
        Some("hello".to_string()),
        Some("test-provider".to_string()),
        Some("test-model".to_string()),
        0.25,
        move |config| {
            *captured_config_for_factory.lock().unwrap() = Some(config.clone());
            agent_for_config(config, observer_for_agent)
        },
    )
    .await
    .unwrap();

    assert_eq!(response, "agent response");

    let effective_config = captured_config.lock().unwrap().clone().unwrap();
    assert_eq!(effective_config.default_provider.as_deref(), Some("test-provider"));
    assert_eq!(effective_config.default_model.as_deref(), Some("test-model"));
    assert_eq!(effective_config.default_temperature, 0.25);

    let events = observer.events();
    assert!(has_agent_start(&events, "test-provider", "test-model"));
    assert!(has_agent_end(&events, "test-provider", "test-model"));
}

#[tokio::test]
async fn run_uses_documented_provider_and_model_fallbacks() {
    let observer = Arc::new(RecordingObserver::new());
    let observer_for_agent = Arc::clone(&observer);
    let config = Config { default_provider: None, default_model: None, ..Config::default() };

    let response =
        run_with_agent_factory(config, Some("hello".to_string()), None, None, 0.7, move |config| {
            agent_for_config(config, observer_for_agent)
        })
        .await
        .unwrap();

    assert_eq!(response, "agent response");

    let events = observer.events();
    assert!(has_agent_start(&events, "openrouter", "anthropic/claude-sonnet-4-20250514"));
    assert!(has_agent_end(&events, "openrouter", "anthropic/claude-sonnet-4-20250514"));
}

#[tokio::test]
async fn run_propagates_agent_factory_error() {
    let config = Config { default_provider: None, default_model: None, ..Config::default() };

    let error = run_with_agent_factory(config, Some("hello".to_string()), None, None, 0.7, |_| {
        Err(anyhow!("factory failed"))
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("factory failed"));
}

#[tokio::test]
async fn run_propagates_provider_error_without_recording_end_event() {
    let observer = Arc::new(RecordingObserver::new());
    let observer_for_agent = Arc::clone(&observer);
    let config = Config { default_provider: None, default_model: None, ..Config::default() };

    let error =
        run_with_agent_factory(config, Some("hello".to_string()), None, None, 0.7, move |config| {
            agent_for_config_with_provider(config, observer_for_agent, Box::new(FailingProvider))
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains("provider failed"));

    let events = observer.events();
    assert!(events.iter().any(|event| matches!(event, ObserverEvent::AgentStart { .. })));
    assert!(!events.iter().any(|event| matches!(event, ObserverEvent::AgentEnd { .. })));
}

#[tokio::test]
async fn run_accepts_empty_message_string() {
    let observer = Arc::new(RecordingObserver::new());
    let observer_for_agent = Arc::clone(&observer);
    let provider = MessageRecordingProvider::new();
    let provider_for_agent = provider.clone();

    let response = run_with_agent_factory(
        Config::default(),
        Some(String::new()),
        None,
        None,
        0.0,
        move |config| {
            agent_for_config_with_provider(config, observer_for_agent, Box::new(provider_for_agent))
        },
    )
    .await
    .unwrap();

    assert_eq!(response, "agent response");

    let messages = provider.messages();
    let final_user_message = messages
        .first()
        .and_then(|request_messages| {
            request_messages.iter().rfind(|message| message.role == "user")
        })
        .expect("provider should receive a user message");
    assert!(final_user_message.content.ends_with("] "));
}
