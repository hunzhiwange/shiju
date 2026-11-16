// 覆盖网关 WebSocket 的握手鉴权、消息处理与连接清理。

// 私有事件解析对缺失可选字段使用稳定默认值，升级失败则返回明确错误。
use super::*;

use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use model_types::{ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::{Tool, ToolResult};
use model_types::ProviderCapabilities;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::routing::get;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::Duration;
use tower::util::ServiceExt;
use vw_api_tools::{ToolResultContentDto, ToolResultDto};

struct StaticProvider;

// 队列 provider 驱动一次工具调用再返回文本，验证 WebSocket 复用完整工具循环。
struct ToolCallingProvider {
    responses: Mutex<VecDeque<ChatResponse>>,
}

impl ToolCallingProvider {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self { responses: Mutex::new(responses.into()) }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for ToolCallingProvider {
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
        anyhow::bail!("websocket tool test uses structured chat")
    }

    async fn chat(
        &self,
        _request: ChatRequest<'_>,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        self.responses
            .lock()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("websocket response script exhausted"))
    }
}

// 探针工具只记录钩子是否到达，不执行真实外部动作。
struct HookProbeTool;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for HookProbeTool {
    fn name(&self) -> &str {
        "hook_probe"
    }

    fn description(&self) -> &str {
        "Deterministic WebSocket hook probe"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

fn tool_call_response(id: &str) -> ChatResponse {
    ChatResponse {
        text: None,
        tool_calls: vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: id.to_string(),
            name: "hook_probe".to_string(),
            arguments: "{}".to_string(),
        }],
        usage: None,
        reasoning_content: None,
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

fn authenticated_state() -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(true, &["paired-token".to_string()])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(super::super::GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(super::super::IdempotencyStore::new(
            Duration::from_secs(300),
            1000,
        )),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

#[tokio::test]
async fn gateway_websocket_passes_hook_runtime_to_tool_loop() {
    let mut state = authenticated_state();
    state.provider = Arc::new(ToolCallingProvider::new(vec![
        tool_call_response("enabled-call"),
        text_response("enabled"),
        tool_call_response("disabled-call"),
        text_response("disabled"),
    ]));
    state.tools_registry_exec = Arc::new(vec![Box::new(HookProbeTool)]);
    let mut hook_config = state.config.lock().clone();
    hook_config.hooks.enabled = true;
    hook_config.hooks.builtin.command_logger = true;
    hook_config.hooks.plugins.enabled = false;
    let runtime = crate::app::agent::hooks::build_hook_runtime(&hook_config).unwrap();
    state.hook_runtime = Some(runtime.clone());
    let mut history = vec![ChatMessage::user("run enabled tool")];
    assert_eq!(
        run_websocket_tool_loop(&state, &mut history, "test", None, None, None,).await.unwrap(),
        "enabled"
    );
    assert_eq!(runtime.audit_store().entries().len(), 1);

    hook_config.hooks.enabled = false;
    runtime.replace(runtime.build_candidate(&hook_config).unwrap());
    let mut history = vec![ChatMessage::user("run disabled tool")];
    assert_eq!(
        run_websocket_tool_loop(&state, &mut history, "test", None, None, None).await.unwrap(),
        "disabled"
    );
    assert_eq!(runtime.audit_store().entries().len(), 1);
}

#[test]
fn websocket_host_handler_is_available() {
    let _ = handle_ws_chat;
}

#[tokio::test]
async fn websocket_upgrade_requires_real_hyper_upgrade_extension() {
    let app = Router::new().route("/ws", get(handle_ws_chat)).with_state(authenticated_state());
    let request = Request::get("/ws")
        .header(header::HOST, "localhost")
        .header(header::CONNECTION, "upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_VERSION, "13")
        .header(header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ==")
        .header(header::SEC_WEBSOCKET_PROTOCOL, "vibewindow.v1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
}
