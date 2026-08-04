use super::core::Agent;
use crate::app::agent::agent::memory_loader::MemoryLoader;
use crate::app::agent::config::AgentConfig;
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry, NoneMemory};
use crate::app::agent::observability::{NoopObserver, Observer};
use crate::app::agent::providers::traits::ProviderCapabilities;
use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::tools::{Tool, ToolResult};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default)]
struct ProviderState {
    responses: Vec<Result<ChatResponse, String>>,
    seen_models: Vec<String>,
    seen_messages: Vec<Vec<ChatMessage>>,
}

#[derive(Clone, Default)]
struct ProviderHandle {
    state: Arc<Mutex<ProviderState>>,
}

impl ProviderHandle {
    fn with_responses(responses: Vec<Result<ChatResponse, String>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ProviderState {
                responses,
                seen_models: Vec::new(),
                seen_messages: Vec::new(),
            })),
        }
    }

    fn provider(&self) -> Box<dyn Provider> {
        Box::new(RecordingProvider { state: self.state.clone(), native_tools: false })
    }

    fn native_provider(&self) -> Box<dyn Provider> {
        Box::new(RecordingProvider { state: self.state.clone(), native_tools: true })
    }

    fn seen_models(&self) -> Vec<String> {
        self.state.lock().seen_models.clone()
    }

    fn seen_messages(&self) -> Vec<Vec<ChatMessage>> {
        self.state.lock().seen_messages.clone()
    }
}

struct RecordingProvider {
    state: Arc<Mutex<ProviderState>>,
    native_tools: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for RecordingProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_calling: self.native_tools, vision: false }
    }

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
        model: &str,
        _temperature: f64,
    ) -> Result<ChatResponse> {
        let mut state = self.state.lock();
        state.seen_models.push(model.to_string());
        state.seen_messages.push(request.messages.to_vec());

        if state.responses.is_empty() {
            return Ok(text_response("done"));
        }

        match state.responses.remove(0) {
            Ok(response) => Ok(response),
            Err(message) => Err(anyhow!(message)),
        }
    }
}

#[derive(Default)]
struct RecordingMemory {
    stores: Mutex<Vec<(String, String, MemoryCategory, Option<String>)>>,
}

impl RecordingMemory {
    fn stored_entries(&self) -> Vec<(String, String, MemoryCategory, Option<String>)> {
        self.stores.lock().clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
    ) -> Result<()> {
        self.stores.lock().push((
            key.to_string(),
            content.to_string(),
            category,
            session_id.map(ToString::to_string),
        ));
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.stores.lock().len())
    }

    async fn health_check(&self) -> bool {
        true
    }
}

struct StaticMemoryLoader {
    result: Result<String, String>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl MemoryLoader for StaticMemoryLoader {
    async fn load_context(&self, _memory: &dyn Memory, _user_message: &str) -> Result<String> {
        self.result.clone().map_err(|message| anyhow!(message))
    }
}

struct EchoTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "echo"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
        Ok(ToolResult { success: true, output: "tool-out".to_string(), error: None })
    }
}

fn text_response(text: &str) -> ChatResponse {
    ChatResponse {
        text: Some(text.to_string()),
        tool_calls: Vec::new(),
        usage: None,
        reasoning_content: None,
    }
}

fn tool_response() -> ChatResponse {
    ChatResponse {
        text: Some(String::new()),
        tool_calls: vec![ToolCall {
            id: "tc1".to_string(),
            name: "echo".to_string(),
            arguments: "{}".to_string(),
        }],
        usage: None,
        reasoning_content: None,
    }
}

fn build_agent(
    provider: Box<dyn Provider>,
    memory: Arc<dyn Memory>,
    memory_loader: Box<dyn MemoryLoader>,
    config: AgentConfig,
) -> Agent {
    let observer: Arc<dyn Observer> = Arc::new(NoopObserver);

    Agent::builder()
        .provider(provider)
        .tools(vec![Box::new(EchoTool)])
        .memory(memory)
        .observer(observer)
        .memory_loader(memory_loader)
        .config(config)
        .model_name("base-model".to_string())
        .workspace_dir(PathBuf::from("/tmp"))
        .build()
        .expect("agent should build with required dependencies")
}

fn default_agent(provider: Box<dyn Provider>, memory_loader: Box<dyn MemoryLoader>) -> Agent {
    build_agent(provider, Arc::new(NoneMemory::new()), memory_loader, AgentConfig::default())
}

fn final_user_message(messages: &[ChatMessage]) -> &ChatMessage {
    messages
        .iter()
        .rfind(|message| message.role == "user")
        .expect("request should contain user message")
}

async fn collect_chunks(mut rx: tokio::sync::mpsc::Receiver<String>) -> Vec<String> {
    let mut chunks = Vec::new();
    while let Some(chunk) = rx.recv().await {
        chunks.push(chunk);
    }
    chunks
}

#[path = "turn_research_tests.rs"]
mod turn_research_tests;

#[test]
fn run_single_method_is_available_on_agent_type() {
    let method = Agent::run_single;

    let _ = method;
}

#[tokio::test]
async fn turn_initializes_system_message_and_enriches_user_message() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("hello"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader {
            result: Ok("[Memory context]\n- tone: concise\n\n".to_string()),
        }),
    );

    let response = agent.turn("hi").await.expect("turn should succeed");

    assert_eq!(response, "hello");
    assert_eq!(agent.history()[0].role, "system");
    assert_eq!(agent.history()[1].role, "user");
    assert_eq!(agent.history()[2].content, "hello");

    let requests = provider.seen_messages();
    let user = final_user_message(&requests[0]);
    assert!(user.content.contains("[Memory context]\n- tone: concise"));
    assert!(user.content.contains("] hi"));
}

#[tokio::test]
async fn turn_timestamp_uses_configured_timezone() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("hello"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.timezone = chrono_tz::UTC;

    agent.turn("hi").await.expect("turn should succeed");

    let requests = provider.seen_messages();
    let user = final_user_message(&requests[0]);
    assert!(user.content.contains("+00:00 (UTC)"));
    assert!(user.content.ends_with("] hi"));
}

#[tokio::test]
async fn turn_skips_memory_context_when_loader_fails() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("plain"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Err("recall failed".to_string()) }),
    );

    let response = agent.turn("hi").await.expect("turn should keep going without memory");

    assert_eq!(response, "plain");
    let requests = provider.seen_messages();
    let user = final_user_message(&requests[0]);
    assert!(!user.content.contains("[Memory context]"));
    assert!(user.content.contains("] hi"));
}

#[tokio::test]
async fn turn_auto_save_records_user_message_without_blocking_response() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("saved"))]);
    let memory = Arc::new(RecordingMemory::default());
    let mut agent = build_agent(
        provider.provider(),
        memory.clone(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
        AgentConfig::default(),
    );
    agent.auto_save = true;

    let response = agent.turn("remember me").await.expect("turn should succeed");

    assert_eq!(response, "saved");
    assert_eq!(
        memory.stored_entries(),
        vec![(
            "user_msg".to_string(),
            "remember me".to_string(),
            MemoryCategory::Conversation,
            None
        )]
    );
}

#[tokio::test]
async fn turn_executes_tool_calls_and_returns_final_response() {
    let provider =
        ProviderHandle::with_responses(vec![Ok(tool_response()), Ok(text_response("done"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );

    let response = agent.turn("use a tool").await.expect("tool loop should finish");

    assert_eq!(response, "done");
    assert!(agent.history().iter().any(|message| message.role == "tool"));
    assert_eq!(provider.seen_messages().len(), 2);
}

#[tokio::test]
async fn turn_routes_classified_message_to_hint_model() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("classified"))]);
    let mut route_model_by_hint = HashMap::new();
    route_model_by_hint.insert("fast".to_string(), "fast-model".to_string());
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.classification_config = crate::app::agent::config::QueryClassificationConfig {
        enabled: true,
        rules: vec![crate::app::agent::config::ClassificationRule {
            hint: "fast".to_string(),
            keywords: vec!["quick".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 10,
        }],
    };
    agent.available_hints = vec!["fast".to_string()];
    agent.route_model_by_hint = route_model_by_hint;

    let response = agent.turn("quick summary").await.expect("turn should succeed");

    assert_eq!(response, "classified");
    assert_eq!(provider.seen_models(), vec!["hint:fast".to_string()]);
}

#[tokio::test]
async fn turn_propagates_provider_error() {
    let provider = ProviderHandle::with_responses(vec![Err("provider failed".to_string())]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );

    let error = agent.turn("hi").await.expect_err("provider error should propagate");

    assert!(error.to_string().contains("provider failed"));
}

#[tokio::test]
async fn turn_trims_history_after_success() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("trimmed"))]);
    let mut config = AgentConfig::default();
    config.max_history_messages = 1;
    let mut agent = build_agent(
        provider.provider(),
        Arc::new(NoneMemory::new()),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
        config,
    );

    let response = agent.turn("short history").await.expect("turn should succeed");

    assert_eq!(response, "trimmed");
    assert_eq!(agent.history().iter().filter(|message| message.role == "system").count(), 1);
    assert_eq!(agent.history().iter().filter(|message| message.role != "system").count(), 1);
    assert_eq!(
        agent.history().last().expect("assistant response should remain").content,
        "trimmed"
    );
}

#[tokio::test]
async fn turn_with_stream_sends_progress_and_final_response() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("streamed answer"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    let (tx, rx) = tokio::sync::mpsc::channel(8);

    let response = agent.turn_with_stream("hi", tx).await.expect("streaming turn should succeed");

    let chunks = collect_chunks(rx).await;

    assert_eq!(response, "streamed answer");
    assert!(chunks.iter().any(|chunk| chunk.contains("思考中")));
    assert!(chunks.iter().any(|chunk| chunk == "streamed "));
    assert!(chunks.iter().any(|chunk| chunk == "answer"));
}

#[tokio::test]
async fn turn_with_stream_timestamp_uses_configured_timezone() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("streamed answer"))]);
    let mut agent = default_agent(
        provider.provider(),
        Box::new(StaticMemoryLoader { result: Ok(String::new()) }),
    );
    agent.timezone = chrono_tz::UTC;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);

    agent.turn_with_stream("hi", tx).await.expect("streaming turn should succeed");

    let requests = provider.seen_messages();
    let user = final_user_message(&requests[0]);
    assert!(user.content.contains("+00:00 (UTC)"));
    assert!(user.content.ends_with("] hi"));
}

#[tokio::test]
async fn turn_with_stream_auto_saves_and_uses_memory_context() {
    let provider = ProviderHandle::with_responses(vec![Ok(text_response("streamed"))]);
    let memory = Arc::new(RecordingMemory::default());
    let mut agent = build_agent(
        provider.provider(),
        memory.clone(),
        Box::new(StaticMemoryLoader { result: Ok("[Memory context]\n".to_string()) }),
        AgentConfig::default(),
    );
    agent.auto_save = true;
    let (tx, rx) = tokio::sync::mpsc::channel(8);

    let response =
        agent.turn_with_stream("stream memory", tx).await.expect("streaming turn should succeed");

    assert_eq!(response, "streamed");
    assert_eq!(
        memory.stored_entries(),
        vec![(
            "user_msg".to_string(),
            "stream memory".to_string(),
            MemoryCategory::Conversation,
            None
        )]
    );
    let requests = provider.seen_messages();
    let user = final_user_message(&requests[0]);
    assert!(user.content.contains("[Memory context]"));
    assert!(user.content.contains("] stream memory"));
    assert!(!collect_chunks(rx).await.is_empty());
}
