//! 推理、JSON SSE 与 WebSocket chat 客户端。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_types::agent_scheduler::AgentSchedulerScopeDto;

use super::GatewayClient;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayAgentRequest {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayAgentResponse {
    pub response: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayChatWebSocketEvent {
    Text { data: Value },
    Binary { data: Vec<u8> },
    Ping { data: Vec<u8> },
    Pong { data: Vec<u8> },
    Closed { code: Option<u16>, reason: Option<String> },
}

impl GatewayChatWebSocketEvent {
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct GatewayChatWebSocket {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

#[cfg(not(target_arch = "wasm32"))]
impl GatewayChatWebSocket {
    pub async fn send_json(&mut self, value: &Value) -> Result<(), String> {
        use futures_util::SinkExt;
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Text(value.to_string().into()))
            .await
            .map_err(|error| format!("chat WebSocket send failed: {error}"))
    }

    pub async fn next_event(&mut self) -> Result<Option<GatewayChatWebSocketEvent>, String> {
        use futures_util::StreamExt;
        use tokio_tungstenite::tungstenite::Message;

        loop {
            let Some(message) = self.socket.next().await else {
                return Ok(None);
            };
            let event = match message
                .map_err(|error| format!("chat WebSocket receive failed: {error}"))?
            {
                Message::Text(data) => GatewayChatWebSocketEvent::Text {
                    data: serde_json::from_str(&data)
                        .map_err(|error| format!("invalid chat WebSocket JSON event: {error}"))?,
                },
                Message::Binary(data) => GatewayChatWebSocketEvent::Binary { data: data.to_vec() },
                Message::Ping(data) => GatewayChatWebSocketEvent::Ping { data: data.to_vec() },
                Message::Pong(data) => GatewayChatWebSocketEvent::Pong { data: data.to_vec() },
                Message::Close(frame) => GatewayChatWebSocketEvent::Closed {
                    code: frame.as_ref().map(|frame| frame.code.into()),
                    reason: frame.map(|frame| frame.reason.to_string()),
                },
                Message::Frame(_) => continue,
            };
            return Ok(Some(event));
        }
    }
}

impl GatewayClient {
    pub async fn inference_agent(
        &self,
        request: &GatewayAgentRequest,
    ) -> Result<GatewayAgentResponse, String> {
        if request.message.trim().is_empty() {
            return Err("agent message must not be empty".to_string());
        }
        self.post_json("/v1/agent", &[], request).await
    }

    pub async fn inference_openai_chat(&self, request: &Value) -> Result<Value, String> {
        if !request.is_object() {
            return Err("OpenAI chat request must be a JSON object".to_string());
        }
        self.post_json("/v1/chat/completions", &[], request).await
    }

    pub async fn stream_gateway_events(
        &self,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        crate::stream::stream_json_sse(&self.endpoint, "/v1/events", &[], on_event).await
    }

    pub async fn stream_global_events(
        &self,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        crate::stream::stream_json_sse(&self.endpoint, "/v1/global/event", &[], on_event).await
    }

    pub async fn stream_instance_events(
        &self,
        directory: Option<&str>,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        let query = directory
            .filter(|value| !value.trim().is_empty())
            .map(|value| vec![("directory".to_string(), value.to_string())])
            .unwrap_or_default();
        crate::stream::stream_json_sse(&self.endpoint, "/v1/event", &query, on_event).await
    }

    pub async fn stream_agent_scheduler_events(
        &self,
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        if scope.project_scope().tenant_scope() != self.endpoint.tenant_scope() {
            return Err("tenant scope does not match gateway endpoint tenant scope".to_string());
        }
        let path = format!(
            "/v1/agent-scheduler/projects/{}/events/stream",
            encode_path_segment(scope.project_scope().project_id().as_ref())
        );
        let query = after_cursor
            .map(|cursor| vec![("after_cursor".to_string(), cursor.to_string())])
            .unwrap_or_default();
        crate::stream::stream_json_sse(&self.endpoint, &path, &query, on_event).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn connect_chat_websocket(&self) -> Result<GatewayChatWebSocket, String> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::http::HeaderValue;
        use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

        let url =
            format!("ws://{}:{}/v1/ws/chat", self.endpoint.normalized_host(), self.endpoint.port);
        let mut request = url
            .into_client_request()
            .map_err(|error| format!("invalid chat WebSocket request: {error}"))?;
        let headers = request.headers_mut();
        headers.insert("sec-websocket-protocol", HeaderValue::from_static("vibewindow.v1"));
        headers.insert(
            PLATFORM_ID_HEADER,
            HeaderValue::from_str(self.endpoint.tenant_scope().platform_id().as_ref())
                .map_err(|error| format!("invalid platform tenant header: {error}"))?,
        );
        headers.insert(
            COMPANY_ID_HEADER,
            HeaderValue::from_str(self.endpoint.tenant_scope().company_id().as_ref())
                .map_err(|error| format!("invalid company tenant header: {error}"))?,
        );
        if let Some(skey) = self
            .endpoint
            .auth
            .as_ref()
            .and_then(|auth| auth.skey.as_deref())
            .filter(|value| !value.trim().is_empty())
        {
            headers.insert(
                "authorization",
                HeaderValue::from_str(&format!("Bearer {skey}"))
                    .map_err(|error| format!("invalid Gateway authorization header: {error}"))?,
            );
        }
        let (socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|error| format!("chat WebSocket connection failed: {error}"))?;
        Ok(GatewayChatWebSocket { socket })
    }
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "inference_api_tests.rs"]
mod inference_api_tests;
