use super::core::Agent;
use crate::app::agent::config::{AgentConfig, ClassificationRule, QueryClassificationConfig};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::tools::{Tool, ToolResult};
use crate::providers::traits::ProviderCapabilities;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

struct CoreTestProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for CoreTestProvider {
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

struct NativeCoreTestProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for NativeCoreTestProvider {
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
        Ok("ok".to_string())
    }
}

struct CoreTestTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CoreTestTool {
    fn name(&self) -> &str {
        "core_test_tool"
    }

    fn description(&self) -> &str {
        "core test tool"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

fn build_agent() -> Agent {
    build_agent_with_provider_and_tools(Box::new(CoreTestProvider), Vec::new())
}

fn build_agent_with_provider_and_tools(
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
) -> Agent {
    Agent::builder()
        .tools(tools)
        .provider(provider)
        .memory(Arc::new(NoneMemory::new()))
        .observer(Arc::new(NoopObserver))
        .build()
        .expect("core test agent should build")
}

#[test]
fn builder_returns_empty_builder_that_fails_without_required_parts() {
    let error = match Agent::builder().build() {
        Ok(_) => panic!("builder should reject missing required dependencies"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("tools are required"));
}

#[test]
fn history_returns_empty_slice_for_new_agent() {
    let agent = build_agent();

    assert!(agent.history().is_empty());
}

#[test]
fn history_returns_messages_in_insertion_order() {
    let mut agent = build_agent();

    agent.history.push(ChatMessage::system("system prompt"));
    agent.history.push(ChatMessage::user("hello"));
    agent.history.push(ChatMessage::assistant("hi"));

    let history = agent.history();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[0].content, "system prompt");
    assert_eq!(history[1].role, "user");
    assert_eq!(history[1].content, "hello");
    assert_eq!(history[2].role, "assistant");
    assert_eq!(history[2].content, "hi");
}

#[test]
fn clear_history_removes_all_messages() {
    let mut agent = build_agent();
    agent.history.push(ChatMessage::user("first"));
    agent.history.push(ChatMessage::assistant("second"));

    agent.clear_history();

    assert!(agent.history().is_empty());
}

#[test]
fn clear_history_is_idempotent_for_empty_history() {
    let mut agent = build_agent();

    agent.clear_history();
    agent.clear_history();

    assert!(agent.history().is_empty());
}

#[test]
fn trim_history_keeps_system_messages_and_recent_non_system_messages() {
    let mut agent = build_agent();
    agent.config = AgentConfig { max_history_messages: 2, ..AgentConfig::default() };
    agent.history.push(ChatMessage::system("system one"));
    agent.history.push(ChatMessage::user("first"));
    agent.history.push(ChatMessage::assistant("second"));
    agent.history.push(ChatMessage::user("third"));

    agent.trim_history();

    assert_eq!(agent.history().len(), 3);
    assert_eq!(agent.history()[0].content, "system one");
    assert_eq!(agent.history()[1].content, "second");
    assert_eq!(agent.history()[2].content, "third");
}

#[test]
fn trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit() {
    let mut agent = build_agent();
    agent.config = AgentConfig { max_history_messages: 2, ..AgentConfig::default() };
    agent.history.push(ChatMessage::system("system one"));
    agent.history.push(ChatMessage::system("system two"));
    agent.history.push(ChatMessage::system("system three"));
    agent.history.push(ChatMessage::user("user"));

    agent.trim_history();

    assert_eq!(agent.history().len(), 4);
    assert_eq!(agent.history()[0].content, "system one");
    assert_eq!(agent.history()[1].content, "system two");
    assert_eq!(agent.history()[2].content, "system three");
    assert_eq!(agent.history()[3].content, "user");
}

#[test]
fn build_system_prompt_includes_tool_protocol_for_prompt_guided_provider() {
    let agent = build_agent_with_provider_and_tools(
        Box::new(CoreTestProvider),
        vec![Box::new(CoreTestTool)],
    );

    let prompt = agent.build_system_prompt().expect("system prompt should build");

    assert!(prompt.contains("Tool Use Protocol"));
    assert!(prompt.contains("<tool>"));
}

#[test]
fn build_system_prompt_omits_tool_protocol_for_native_provider() {
    let agent = build_agent_with_provider_and_tools(
        Box::new(NativeCoreTestProvider),
        vec![Box::new(CoreTestTool)],
    );

    let prompt = agent.build_system_prompt().expect("system prompt should build");

    assert!(!prompt.contains("Tool Use Protocol"));
    assert!(!prompt.contains("<tool>"));
}

#[test]
fn classify_model_returns_hint_for_available_hint_without_route_mapping() {
    let mut agent = build_agent();
    agent.classification_config = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "fast".to_string(),
            keywords: vec!["quick".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 10,
        }],
    };
    agent.available_hints = vec!["fast".to_string()];
    agent.route_model_by_hint = HashMap::new();

    assert_eq!(agent.classify_model("quick summary"), "hint:fast");
}

#[test]
fn classify_model_returns_base_model_when_matched_hint_is_unavailable() {
    let mut agent = build_agent();
    agent.model_name = "base-model".to_string();
    agent.classification_config = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "fast".to_string(),
            keywords: vec!["quick".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 10,
        }],
    };

    assert_eq!(agent.classify_model("quick summary"), "base-model");
}

#[test]
fn classify_model_returns_base_model_when_no_rule_matches() {
    let mut agent = build_agent();
    agent.model_name = "base-model".to_string();
    agent.classification_config = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "fast".to_string(),
            keywords: vec!["quick".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 10,
        }],
    };
    agent.available_hints = vec!["fast".to_string()];

    assert_eq!(agent.classify_model("slow summary"), "base-model");
}

#[test]
fn from_config_builds_agent_with_none_memory_and_supported_provider() {
    let mut config = crate::app::agent::config::Config::default();
    config.memory.backend = "none".to_string();
    config.default_provider = Some("openai".to_string());
    config.default_model = Some("testcov-0076-model".to_string());
    config.default_temperature = 0.25;
    config.workspace_dir =
        std::env::temp_dir().join(format!("vibe-window-testcov-0076-{}", std::process::id()));

    let agent = Agent::from_config(&config).expect("supported provider should build");

    assert!(agent.history().is_empty());
    assert_eq!(agent.model_name, "testcov-0076-model");
    assert_eq!(agent.temperature, 0.25);
    assert_eq!(agent.workspace_dir, config.workspace_dir);
    assert_eq!(agent.memory.name(), "none");
}

#[test]
fn from_config_rejects_unknown_provider() {
    let mut config = crate::app::agent::config::Config::default();
    config.memory.backend = "none".to_string();
    config.default_provider = Some("testcov-0076-unknown-provider".to_string());
    config.workspace_dir =
        std::env::temp_dir().join(format!("vibe-window-testcov-0076-error-{}", std::process::id()));

    let error = match Agent::from_config(&config) {
        Ok(_) => panic!("unknown provider should be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("Unknown provider: testcov-0076-unknown-provider"));
}

#[tokio::test]
async fn run_single_delegates_message_to_turn_and_returns_response() {
    let mut agent = build_agent();

    let response = agent.run_single("hello").await.expect("run_single should succeed");

    assert_eq!(response, "ok");
    assert!(agent.history().iter().any(|message| message.role == "user"));
    assert_eq!(agent.history().last().map(|message| message.content.as_str()), Some("ok"));
}
