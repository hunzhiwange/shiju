use super::*;
use crate::app::agent::config::DelegateAgentConfig;
use crate::app::agent::providers::{ChatRequest, ChatResponse, Provider, ProviderRuntimeOptions};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use anyhow::anyhow;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

fn invalid_provider_config() -> DelegateAgentConfig {
    delegate_config("invalid-provider-for-forwarding-test")
}

fn delegate_config(provider: &str) -> DelegateAgentConfig {
    DelegateAgentConfig {
        label: None,
        description: None,
        builtin: false,
        mode: "all".to_string(),
        enabled: true,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        provider: provider.to_string(),
        model: "test-model".to_string(),
        system_prompt: None,
        api_key: None,
        temperature: None,
        top_p: None,
        identity_format: None,
        hidden: false,
        max_depth: 2,
        agentic: false,
        allowed_tools: Vec::new(),
        allowed_skills: Vec::new(),
        enabled_identity_files: Vec::new(),
        options: HashMap::new(),
        permission: Value::Null,
        max_iterations: 2,
        steps: None,
    }
}

struct EmptyResponseProvider;
struct SlowProvider;
struct ErrorResponseProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for EmptyResponseProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        assert_eq!(system_prompt, Some("system rules"));
        assert!(message.contains("[Context]\ncontext details\n\n[Task]\ninspect"));
        assert_eq!(model, "test-model");
        assert_eq!(temperature, 0.7);
        Ok("  \n".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Err(anyhow!("chat should not be used by non-agentic delegate execution"))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for SlowProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        tokio::time::sleep(std::time::Duration::from_secs(DELEGATE_TIMEOUT_SECS + 1)).await;
        Ok("late response".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Err(anyhow!("chat should not be used by non-agentic delegate execution"))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for ErrorResponseProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Err(anyhow!("provider exploded"))
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Err(anyhow!("chat should not be used by non-agentic delegate execution"))
    }
}

#[tokio::test]
async fn execute_skips_duplicate_security_check_for_agent_tool_forwarding() {
    let mut agents = HashMap::new();
    agents.insert("worker".to_string(), invalid_provider_config());
    let readonly =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = DelegateTool::new(agents, None, readonly);

    let result = execute(
        &tool,
        json!({
            "agent": "worker",
            "prompt": "inspect the failure",
            "_via_agent_tool": true
        }),
    )
    .await
    .expect("forwarded execution should return a tool result");

    assert!(!result.success);
    assert!(
        result.error.as_deref().unwrap_or_default().contains("Failed to create provider"),
        "forwarded calls should reach provider creation instead of read-only rejection"
    );
}

#[tokio::test]
async fn execute_with_provider_factory_formats_empty_non_agentic_response() {
    let mut config = delegate_config("mock-provider");
    config.api_key = Some("agent-secret".to_string());
    config.system_prompt = Some("system rules".to_string());

    let mut agents = HashMap::new();
    agents.insert("worker".to_string(), config);
    let tool = DelegateTool::new(
        agents,
        Some("fallback-secret".to_string()),
        Arc::new(SecurityPolicy::default()),
    );

    let result = execute_with_provider_factory(
        &tool,
        json!({
            "agent": "worker",
            "prompt": "inspect",
            "context": "context details"
        }),
        &|provider, credential, _options: &ProviderRuntimeOptions| {
            assert_eq!(provider, "mock-provider");
            assert_eq!(credential, Some("agent-secret"));
            Ok(Box::new(EmptyResponseProvider))
        },
    )
    .await
    .expect("injected provider should execute");

    assert!(result.success);
    assert_eq!(result.output, "[Agent 'worker' (mock-provider/test-model)]\n[Empty response]");
    assert!(result.error.is_none());
}

#[tokio::test(start_paused = true)]
async fn execute_with_provider_factory_reports_non_agentic_timeout() {
    let mut agents = HashMap::new();
    agents.insert("worker".to_string(), delegate_config("mock-provider"));
    let tool = DelegateTool::new(agents, None, Arc::new(SecurityPolicy::default()));

    let result = execute_with_provider_factory(
        &tool,
        json!({
            "agent": "worker",
            "prompt": "inspect"
        }),
        &|_provider, _credential, _options: &ProviderRuntimeOptions| Ok(Box::new(SlowProvider)),
    )
    .await
    .expect("timeout should return a tool result");

    assert!(!result.success);
    assert_eq!(result.output, "");
    assert_eq!(result.error.as_deref(), Some("Agent 'worker' timed out after 120s"));
}

#[tokio::test]
async fn execute_with_provider_factory_reports_non_agentic_provider_error() {
    let mut agents = HashMap::new();
    agents.insert("worker".to_string(), delegate_config("mock-provider"));
    let tool = DelegateTool::new(agents, None, Arc::new(SecurityPolicy::default()));

    let result = execute_with_provider_factory(
        &tool,
        json!({
            "agent": "worker",
            "prompt": "inspect"
        }),
        &|_provider, _credential, _options: &ProviderRuntimeOptions| {
            Ok(Box::new(ErrorResponseProvider))
        },
    )
    .await
    .expect("provider error should return a tool result");

    assert!(!result.success);
    assert_eq!(result.output, "");
    assert_eq!(result.error.as_deref(), Some("Agent 'worker' failed: provider exploded"));
}
