//! Gateway PTY 生命周期与 WebSocket 连接接口。

use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use super::GatewayClient;
use crate::http::directory_query;

/// PTY 运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtyStatusDto {
    Running,
    Exited,
}

/// PTY 会话详情。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyInfoDto {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: PtyStatusDto,
    pub pid: u32,
}

/// 创建 PTY 的显式配置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// PTY 终端尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySizeDto {
    pub rows: u16,
    pub cols: u16,
}

/// 更新 PTY 标题或尺寸。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtyUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<PtySizeDto>,
}

/// PTY WebSocket 收到的协议事件。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PtyWebSocketEvent {
    Text { data: String },
    Binary { data: Vec<u8> },
    Ping { data: Vec<u8> },
    Pong { data: Vec<u8> },
    Closed { code: Option<u16>, reason: Option<String> },
}

#[cfg(not(target_arch = "wasm32"))]
impl PtyWebSocketEvent {
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

/// 已升级的 PTY WebSocket 连接。
#[cfg(not(target_arch = "wasm32"))]
pub struct PtyConnection {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

#[cfg(not(target_arch = "wasm32"))]
impl PtyConnection {
    /// 向远程 PTY 写入文本。
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), String> {
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Text(text.into().into()))
            .await
            .map_err(|error| format!("PTY WebSocket send failed: {error}"))
    }

    /// 读取下一个 PTY WebSocket 事件。
    pub async fn next_event(&mut self) -> Result<Option<PtyWebSocketEvent>, String> {
        use tokio_tungstenite::tungstenite::Message;

        loop {
            let Some(message) = self.socket.next().await else {
                return Ok(None);
            };
            let event = match message.map_err(|error| format!("PTY WebSocket receive failed: {error}"))?
            {
                Message::Text(data) => PtyWebSocketEvent::Text { data: data.to_string() },
                Message::Binary(data) => PtyWebSocketEvent::Binary { data: data.to_vec() },
                Message::Ping(data) => PtyWebSocketEvent::Ping { data: data.to_vec() },
                Message::Pong(data) => PtyWebSocketEvent::Pong { data: data.to_vec() },
                Message::Close(frame) => PtyWebSocketEvent::Closed {
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
    pub async fn pty_list(&self, directory: Option<&str>) -> Result<Vec<PtyInfoDto>, String> {
        self.get_json("/v1/pty", &directory_query(directory)).await
    }

    pub async fn pty_create(
        &self,
        input: &PtyCreateRequest,
        directory: Option<&str>,
    ) -> Result<PtyInfoDto, String> {
        self.post_json("/v1/pty", &directory_query(directory), input).await
    }

    pub async fn pty_get(
        &self,
        pty_id: &str,
        directory: Option<&str>,
    ) -> Result<PtyInfoDto, String> {
        let path = pty_path(pty_id)?;
        self.get_json(&path, &directory_query(directory)).await
    }

    pub async fn pty_update(
        &self,
        pty_id: &str,
        input: &PtyUpdateRequest,
        directory: Option<&str>,
    ) -> Result<PtyInfoDto, String> {
        let path = pty_path(pty_id)?;
        self.put_json(&path, &directory_query(directory), input).await
    }

    pub async fn pty_delete(
        &self,
        pty_id: &str,
        directory: Option<&str>,
    ) -> Result<bool, String> {
        let path = pty_path(pty_id)?;
        self.delete_json(&path, &directory_query(directory), &serde_json::Value::Null).await
    }

    /// 使用真实 WebSocket 协议连接 PTY，不回退为轮询。
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn pty_connect(
        &self,
        pty_id: &str,
        directory: Option<&str>,
        cursor: Option<i64>,
    ) -> Result<PtyConnection, String> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::http::HeaderValue;
        use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

        let pty_id = required_pty_id(pty_id)?;
        let mut url = reqwest::Url::parse(&format!(
            "ws://{}:{}/v1/pty/{}/connect",
            self.endpoint.normalized_host(),
            self.endpoint.port,
            encode_path_segment(pty_id)
        ))
        .map_err(|error| format!("invalid PTY WebSocket URL: {error}"))?;
        let directory = directory.filter(|value| !value.trim().is_empty());
        if directory.is_some() || cursor.is_some() {
            let mut query = url.query_pairs_mut();
            if let Some(directory) = directory {
                query.append_pair("directory", directory);
            }
            if let Some(cursor) = cursor {
                query.append_pair("cursor", &cursor.to_string());
            }
        }
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|error| format!("invalid PTY WebSocket request: {error}"))?;
        let headers = request.headers_mut();
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
            .map_err(|error| format!("PTY WebSocket connection failed: {error}"))?;
        Ok(PtyConnection { socket })
    }
}

fn pty_path(pty_id: &str) -> Result<String, String> {
    Ok(format!("/v1/pty/{}", encode_path_segment(required_pty_id(pty_id)?)))
}

fn required_pty_id(pty_id: &str) -> Result<&str, String> {
    let pty_id = pty_id.trim();
    if pty_id.is_empty() {
        return Err("PTY id must not be empty".to_string());
    }
    Ok(pty_id)
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
#[path = "pty_api_tests.rs"]
mod pty_api_tests;
