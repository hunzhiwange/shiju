//! 测试工具调用循环的终止、重试、迭代耗尽与工具过滤策略。
//! 队列式供应商精确控制每轮响应，验证重试次数和迭代耗尽的停止位置。

use super::services::run_tool_call_loop;
use crate::core::constants::EMPTY_FINAL_RESPONSE_RETRY_PROMPT;
use crate::core::errors::ToolLoopCancelled;
use model_types::{ChatMessage, Provider};
use tool_types::Tool;
use model_types::ProviderCapabilities;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_util::sync::CancellationToken;

struct StaticProvider {
    responses: Mutex<VecDeque<model_types::ChatResponse>>,
    calls: Arc<AtomicUsize>,
    requested_tools: Arc<Mutex<Vec<Vec<String>>>>,
}

impl StaticProvider {
    fn from_text(response: &'static str, calls: Arc<AtomicUsize>) -> Self {
        Self::from_responses(
            vec![model_types::ChatResponse {
                text: Some(response.to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            }],
            calls,
        )
    }

    fn from_responses(
        responses: Vec<model_types::ChatResponse>,
        calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            calls,
            requested_tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requested_tools(&self) -> Vec<Vec<String>> {
        self.requested_tools.lock().expect("requested tools mutex poisoned").clone()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_search: false, native_tool_calling: true, vision: false }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("tool loop tests should use structured chat")
    }

    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tool_ids =
            request.tools.unwrap_or_default().iter().map(|spec| spec.id.clone()).collect();
        self.requested_tools.lock().expect("requested tools mutex poisoned").push(tool_ids);
        self.responses
            .lock()
            .expect("responses mutex poisoned")
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("static provider response script exhausted"))
    }
}

struct EchoTool {
    name: &'static str,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "Echoes the provided value"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> anyhow::Result<tool_types::ToolResult> {
        Ok(tool_types::ToolResult {
            success: true,
            output: args
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            error: None,
        })
    }
}

mod tests;
