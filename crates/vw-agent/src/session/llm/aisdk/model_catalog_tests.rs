//! Exercise the projected catalog at the native request boundary against a loopback upstream.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Json, Router, extract::State, routing::post};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelCatalogEntry, ModelCatalogIdentity, ModelMetadataDto, ScopedModelCatalogIdentity,
    },
};

use crate::{
    provider::{model_catalog::project_model, provider},
    session::{
        llm::types::{Error, StreamEvent},
        message::AssistantError,
    },
    tools::ToolSpec,
};

const MODEL_ID: &str = "qwen3-coder-plus";
const IMAGE: &str = "data:image/png;base64,aW1hZ2U=";

struct Outcome {
    result: Result<(), Error>,
    events: Vec<StreamEvent>,
    requests: Vec<Value>,
}

impl Outcome {
    fn assert_rejected(self, capability: &str) {
        assert_eq!(self.requests.len(), 0, "unsupported capability reached upstream");
        let Err(Error::Api(AssistantError::Unknown { message: error })) = self.result else {
            panic!("expected capability error, got {:?}", self.result);
        };
        assert!(error.contains(MODEL_ID) && error.contains(capability), "{error}");
        assert!(self.events.is_empty());
    }

    fn assert_forwarded(mut self) -> Value {
        self.result.unwrap();
        assert!(
            self.events
                .iter()
                .any(|event| { matches!(event, StreamEvent::Delta(text) if text == "ready") })
        );
        assert_eq!(self.requests.len(), 1);
        let request = self.requests.pop().unwrap();
        assert_eq!(request["model"], MODEL_ID);
        request
    }
}

async fn send(metadata: ModelMetadataDto, messages: Value, tool: bool, options: Value) -> Outcome {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(capture_request))
        .with_state(Arc::clone(&requests));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let row = ModelCatalogEntry {
        identity: ScopedModelCatalogIdentity {
            scope: TenantScope::new("catalog", "execution").unwrap(),
            deployment: ModelCatalogIdentity {
                target_host_id: "mac-fixture".into(),
                app_id: AppId::Codex,
                provider_id: "saved-fixture".into(),
                endpoint,
            },
            model_id: MODEL_ID.into(),
        },
        display_alias: Some("测试模型".into()),
        metadata,
    };
    let model = project_model("fixture", "openai-compatible", &row);
    let provider = provider::Info {
        id: model.provider_id.clone(),
        name: "Fixture".into(),
        source: provider::ProviderSource::Custom,
        env: vec![],
        key: Some("fictional-test-key".into()),
        options: HashMap::new(),
        models: HashMap::new(),
    };
    let mut tools = HashMap::new();
    if tool {
        tools.insert(
            "read".into(),
            ToolSpec::new("read", "Read a fixture", json!({"type": "object", "properties": {}})),
        );
    }

    let mut events = Vec::new();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        super::do_stream_request_aisdk(
            &provider,
            None,
            &HashMap::new(),
            &options,
            &model,
            &messages,
            &tools,
            None,
            None,
            Some(64),
            0,
            None,
            &mut |event| events.push(event),
        ),
    )
    .await;
    server.abort();
    let _ = server.await;
    let requests = std::mem::take(&mut *requests.lock().unwrap());
    Outcome { result: result.expect("loopback request timed out"), events, requests }
}

async fn capture_request(
    State(requests): State<Arc<Mutex<Vec<Value>>>>,
    Json(request): Json<Value>,
) -> ([(&'static str, &'static str); 1], &'static str) {
    requests.lock().unwrap().push(request);
    (
        [("content-type", "text/event-stream")],
        concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ready\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        ),
    )
}

fn image_message() -> Value {
    json!([{"role": "user", "content": [
        {"type": "text", "text": "Describe the image"},
        {"type": "image_url", "image_url": {"url": IMAGE}}
    ]}])
}

fn text_message() -> Value {
    json!([{"role": "user", "content": "Hello"}])
}

#[tokio::test]
async fn catalog_execution_gate_blocks_before_network() {
    send(
        ModelMetadataDto { input_modalities: Some(vec!["text".into()]), ..Default::default() },
        image_message(),
        false,
        json!({}),
    )
    .await
    .assert_rejected("image input");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_image_markers() {
    send(
        ModelMetadataDto { input_modalities: Some(vec!["text".into()]), ..Default::default() },
        json!([{"role": "user", "content": format!("Describe [IMAGE:{IMAGE}]")}]),
        false,
        json!({}),
    )
    .await
    .assert_rejected("image input");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_unsupported_audio_parts() {
    send(
        ModelMetadataDto { input_modalities: Some(vec!["text".into()]), ..Default::default() },
        json!([{"role": "user", "content": [
            {"type": "text", "text": "Transcribe"},
            {"type": "input_audio", "input_audio": {"data": "YXVkaW8=", "format": "wav"}}
        ]}]),
        false,
        json!({}),
    )
    .await
    .assert_rejected("audio input");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_unsupported_tool_definitions() {
    send(
        ModelMetadataDto { tool_calls: Unsupported, ..Default::default() },
        text_message(),
        true,
        json!({}),
    )
    .await
    .assert_rejected("tool calls");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_unsupported_tool_history() {
    send(
        ModelMetadataDto { tool_calls: Unsupported, ..Default::default() },
        json!([
            {"role": "assistant", "tool_calls": [
                {"id": "call-1", "type": "function", "function": {"name": "read", "arguments": "{}"}}
            ]},
            {"role": "tool", "tool_call_id": "call-1", "content": "fixture"},
            {"role": "user", "content": "Continue"}
        ]),
        false,
        json!({}),
    )
    .await
    .assert_rejected("tool calls");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_undeclared_effort_level() {
    send(
        ModelMetadataDto {
            reasoning: Supported,
            reasoning_efforts: Some(vec!["low".into()]),
            ..Default::default()
        },
        text_message(),
        false,
        json!({"reasoning_effort": "high"}),
    )
    .await
    .assert_rejected("reasoning_effort=high");
}

#[tokio::test]
async fn catalog_execution_gate_blocks_unsupported_reasoning() {
    send(
        ModelMetadataDto {
            reasoning: Unsupported,
            reasoning_efforts: Some(vec!["high".into()]),
            ..Default::default()
        },
        text_message(),
        false,
        json!({"reasoning_effort": "high"}),
    )
    .await
    .assert_rejected("reasoning_effort=high");
}

#[tokio::test]
async fn catalog_execution_gate_preserves_explicitly_supported_request() {
    let body = send(
        ModelMetadataDto {
            input_modalities: Some(vec!["text".into(), "image".into()]),
            tool_calls: Supported,
            reasoning: Supported,
            reasoning_efforts: Some(vec!["high".into()]),
            ..Default::default()
        },
        image_message(),
        true,
        json!({"reasoning_effort": "high"}),
    )
    .await
    .assert_forwarded();
    assert_eq!(body["messages"][0]["content"][1]["image_url"]["url"], IMAGE);
    assert_eq!(body["tools"][0]["function"]["name"], "read");
    assert_eq!(body["reasoning_effort"], "high");
}

#[tokio::test]
async fn catalog_execution_gate_preserves_unknown_media_and_tool_policy() {
    let body = send(ModelMetadataDto::default(), image_message(), true, json!({}))
        .await
        .assert_forwarded();
    assert_eq!(body["messages"][0]["content"][1]["image_url"]["url"], IMAGE);
    assert_eq!(body["tools"][0]["function"]["name"], "read");
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn catalog_execution_gate_requires_evidence_for_explicit_effort() {
    send(ModelMetadataDto::default(), text_message(), false, json!({"reasoning_effort": "low"}))
        .await
        .assert_rejected("reasoning_effort=low");
}
