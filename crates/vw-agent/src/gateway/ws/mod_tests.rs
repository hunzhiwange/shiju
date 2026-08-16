use super::*;

use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::{ChatRequest, ChatResponse, Provider, ToolCall};
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::{Tool, ToolResult};
use crate::providers::traits::ProviderCapabilities;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::routing::get;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::Duration;
use tower::util::ServiceExt;
use vw_api_types::tools::{ToolResultContentDto, ToolResultDto};

struct StaticProvider;

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
        ProviderCapabilities { native_tool_calling: true, vision: false }
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
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(true, &["paired-token".to_string()])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(super::super::GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(super::super::IdempotencyStore::new(
            Duration::from_secs(300),
            1000,
        )),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
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
        run_websocket_tool_loop(
            &state,
            &mut history,
            "test",
            None,
            None,
            None,
        )
        .await
        .unwrap(),
        "enabled"
    );
    assert_eq!(runtime.audit_store().entries().len(), 1);

    hook_config.hooks.enabled = false;
    runtime.replace(runtime.build_candidate(&hook_config).unwrap());
    let mut history = vec![ChatMessage::user("run disabled tool")];
    assert_eq!(
        run_websocket_tool_loop(&state, &mut history, "test", None, None, None)
            .await
            .unwrap(),
        "disabled"
    );
    assert_eq!(runtime.audit_store().entries().len(), 1);
}

#[test]
fn websocket_entrypoints_and_constants_are_available() {
    let _ = handle_ws_chat;
    let _ = handle_socket;
    let _ = emit_ws_delta_event;
    assert_eq!(WS_CHAT_SUBPROTOCOL, "vibewindow.v1");
    assert!(EMPTY_WS_RESPONSE_FALLBACK.contains("Tool execution completed"));
}

#[test]
fn extract_ws_bearer_token_falls_back_when_authorization_is_not_bearer() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Basic dXNlcjpwYXNz".parse().unwrap());
    headers.insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        "vibewindow.v1, bearer.protocol-token".parse().unwrap(),
    );

    assert_eq!(extract_ws_bearer_token(&headers).as_deref(), Some("protocol-token"));
}

#[test]
fn parse_ws_private_event_uses_plain_payload_defaults() {
    let progress = format!(
        "{DRAFT_WS_EVENT_SENTINEL}{}",
        serde_json::json!({
            "event": "tool_result",
            "success": true,
            "duration_secs": 7,
            "tool_call_id": "call_plain"
        })
    );

    assert_eq!(
        parse_ws_private_event(&progress),
        Some(WsDeltaEvent::ToolResult {
            name: "tool".to_string(),
            success: true,
            duration_secs: Some(7),
            tool_call_id: Some("call_plain".to_string()),
            result: None,
        })
    );
}

#[test]
fn parse_ws_private_event_prefers_explicit_name_over_result_tool_id() {
    let result = ToolResultDto {
        tool_use_id: Some("call_result".to_string()),
        tool_id: Some("file_read".into()),
        success: Some(false),
        content: vec![ToolResultContentDto::Text { text: "denied".to_string() }],
        data: serde_json::Value::Null,
        model_result: serde_json::Value::Null,
        render_hint: None,
        permission_request: None,
        context_updates: Vec::new(),
        extra_messages: Vec::new(),
        telemetry: None,
    };
    let progress = format!(
        "{DRAFT_WS_EVENT_SENTINEL}{}",
        serde_json::json!({
            "event": "tool_result",
            "name": "display_name",
            "result": result,
        })
    );

    assert_eq!(
        parse_ws_private_event(&progress),
        Some(WsDeltaEvent::ToolResult {
            name: "display_name".to_string(),
            success: false,
            duration_secs: None,
            tool_call_id: Some("call_result".to_string()),
            result: Some(result),
        })
    );
}

#[test]
fn parse_ws_delta_event_maps_empty_hint_to_none_and_bad_duration_to_none() {
    assert_eq!(
        parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}⏳ shell:   ")),
        Some(WsDeltaEvent::ToolCall { name: "shell".to_string(), hint: None })
    );
    assert_eq!(
        parse_ws_delta_event(&format!("{DRAFT_PROGRESS_SENTINEL}✅ shell (later)")),
        Some(WsDeltaEvent::ToolResult {
            name: "shell".to_string(),
            success: true,
            duration_secs: None,
            tool_call_id: None,
            result: None,
        })
    );
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
