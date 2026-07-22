use super::*;
use crate::app::agent::config::DelegateAgentConfig;
use crate::app::agent::providers::{ChatRequest, ChatResponse};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;

struct EmptyResponseProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for EmptyResponseProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".to_string())
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            text: Some(" \n\t ".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct PassiveTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for PassiveTool {
    fn name(&self) -> &str {
        "passive"
    }

    fn description(&self) -> &str {
        "available to the delegated agent"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "unused".to_string(), error: None })
    }
}

fn agentic_config() -> DelegateAgentConfig {
    DelegateAgentConfig {
        label: None,
        description: None,
        builtin: false,
        mode: "all".to_string(),
        enabled: true,
        provider: "test-provider".to_string(),
        model: "test-model".to_string(),
        system_prompt: None,
        api_key: None,
        temperature: Some(0.2),
        top_p: None,
        identity_format: None,
        hidden: false,
        max_depth: 2,
        agentic: true,
        allowed_tools: vec!["passive".to_string()],
        allowed_skills: Vec::new(),
        options: HashMap::new(),
        permission: Value::Null,
        max_iterations: 2,
        steps: None,
    }
}

#[tokio::test]
async fn execute_agentic_renders_whitespace_only_response_as_empty() {
    let tool =
        DelegateTool::new(HashMap::new(), None, std::sync::Arc::new(SecurityPolicy::default()))
            .with_parent_tools(std::sync::Arc::new(vec![std::sync::Arc::new(PassiveTool)]));

    let result = execute_agentic(
        &tool,
        "summarizer",
        &agentic_config(),
        &EmptyResponseProvider,
        None,
        "summarize",
        0.2,
    )
    .await
    .expect("agentic execution should return a tool result");

    assert!(result.success);
    assert_eq!(
        result.output,
        "[Agent 'summarizer' (test-provider/test-model, agentic)]\n[Empty response]"
    );
    assert_eq!(result.error, None);
}
