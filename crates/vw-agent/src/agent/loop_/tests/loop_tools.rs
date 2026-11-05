//! # 工具调用循环测试模块
//!
//! 本模块保留 `run_tool_call_loop` 相关测试辅助结构，并按职责将具体测试
//! 拆分到多个独立子文件，降低单文件复杂度。

use super::*;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::ChatResponse;
use crate::app::agent::providers::traits::ProviderCapabilities;

mod approvals;
mod multimodal;
mod native_mode;
mod parallel_execution;
mod recovery;

struct NonVisionProvider {
    calls: Arc<AtomicUsize>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for NonVisionProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("ok".to_string())
    }
}

struct VisionProvider {
    calls: Arc<AtomicUsize>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for VisionProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities { native_tool_search: false, native_tool_calling: false, vision: true }
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("ok".to_string())
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);

        let marker_count = crate::app::agent::multimodal::count_image_markers(request.messages);
        if marker_count == 0 {
            anyhow::bail!("expected image markers in request messages");
        }
        if request.tools.is_some() {
            anyhow::bail!("no tools should be attached for this test");
        }

        Ok(ChatResponse {
            text: Some("vision-ok".to_string()),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }
}

struct ScriptedProvider {
    responses: Arc<Mutex<VecDeque<ChatResponse>>>,
    capabilities: ProviderCapabilities,
}

impl ScriptedProvider {
    fn from_text_responses(responses: Vec<&str>) -> Self {
        let scripted = responses
            .into_iter()
            .map(|text| ChatResponse {
                text: Some(text.to_string()),
                tool_calls: Vec::new(),
                usage: None,
                reasoning_content: None,
            })
            .collect();

        Self {
            responses: Arc::new(Mutex::new(scripted)),
            capabilities: ProviderCapabilities::default(),
        }
    }

    fn with_native_tool_support(mut self) -> Self {
        self.capabilities.native_tool_calling = true;
        self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for ScriptedProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities.clone()
    }

    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("chat_with_system should not be used in scripted provider tests");
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let mut responses = self.responses.lock().expect("responses lock should be valid");
        responses
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("scripted provider exhausted responses"))
    }
}

struct CountingTool {
    name: String,
    invocations: Arc<AtomicUsize>,
}

impl CountingTool {
    fn new(name: &str, invocations: Arc<AtomicUsize>) -> Self {
        Self { name: name.to_string(), invocations }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CountingTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Counts executions for loop-stability tests"
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
    ) -> anyhow::Result<crate::app::agent::tools::ToolResult> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        let value = args.get("value").and_then(serde_json::Value::as_str).unwrap_or_default();

        Ok(crate::app::agent::tools::ToolResult {
            success: true,
            output: format!("counted:{value}"),
            error: None,
        })
    }
}

struct DelayTool {
    name: String,
    delay_ms: u64,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
}

impl DelayTool {
    fn new(
        name: &str,
        delay_ms: u64,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    ) -> Self {
        Self { name: name.to_string(), delay_ms, active, max_active }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for DelayTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Delay tool for testing parallel tool execution"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> anyhow::Result<crate::app::agent::tools::ToolResult> {
        let now_active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(now_active, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        self.active.fetch_sub(1, Ordering::SeqCst);

        let value =
            args.get("value").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();

        Ok(crate::app::agent::tools::ToolResult {
            success: true,
            output: format!("ok:{value}"),
            error: None,
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        self.name.starts_with("delay_")
    }

    fn is_read_only(&self) -> bool {
        self.name.starts_with("delay_")
    }
}
