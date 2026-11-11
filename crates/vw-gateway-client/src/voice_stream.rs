//! Native, single-connection voice transport. Dropping the socket ends the gateway lease.
use crate::http::apply_request_context;
use crate::{GatewayEndpoint, VoiceApiError};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message,
        protocol::{Role, WebSocketConfig},
    },
};
use vw_api_types::voice::*;
type Result<T> = std::result::Result<T, VoiceApiError>;

/// Bound to the endpoint and tenant passed at creation. The desktop must close/drop
/// this connection before switching tenants; it must never reuse it for another chat.
pub struct VoiceStream {
    socket: WebSocketStream<reqwest::Upgraded>,
    session: VoiceSessionStarted,
    generation: VoiceCounter,
    sent: u64,
    received: u64,
    closed: bool,
}
impl VoiceStream {
    /// A fresh desktop connection identifier prevents channels sharing a skey from mixing.
    /// The existing endpoint is HTTP-only, so recordings are restricted to loopback.
    pub async fn connect(endpoint: &GatewayEndpoint, start: &VoiceSessionStart) -> Result<Self> {
        if !endpoint.is_loopback() {
            return Err(VoiceApiError::Service(VoiceErrorCode::UnsupportedCapability));
        }
        if start.generation != VoiceCounter::FIRST {
            return Err(VoiceApiError::Service(VoiceErrorCode::InvalidRequest));
        }
        let work = async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .http1_only()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .build()
                .map_err(|_| VoiceApiError::Transport)?;
            let connection = tokio_tungstenite::tungstenite::handshake::client::generate_key();
            let response = apply_request_context(
                client.post(format!("{}/api/voice/sessions", endpoint.base_url())),
                endpoint,
            )
            .header("x-shiju-voice-connection-id", &connection)
            .json(start)
            .send()
            .await
            .map_err(|_| VoiceApiError::Transport)?;
            let status = response.status();
            let mut bytes = Vec::new();
            let mut chunks = response.bytes_stream();
            while let Some(chunk) = chunks.next().await {
                let chunk = chunk.map_err(|_| VoiceApiError::Transport)?;
                if bytes.len() + chunk.len() > MAX_METADATA_BYTES {
                    return Err(VoiceApiError::InvalidResponse);
                }
                bytes.extend_from_slice(&chunk);
            }
            if !status.is_success() {
                let error: VoiceErrorResponse =
                    serde_json::from_slice(&bytes).map_err(|_| VoiceApiError::InvalidResponse)?;
                if error.error.code.http_status() != status.as_u16() {
                    return Err(VoiceApiError::InvalidResponse);
                }
                return Err(VoiceApiError::Service(error.error.code));
            }
            let session: VoiceSessionStarted =
                serde_json::from_slice(&bytes).map_err(|_| VoiceApiError::InvalidResponse)?;
            // Never follow a service-supplied host, path, redirect or query token.
            let id = session.voice_session_id.as_ref();
            if !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || session.stream_path != format!("/api/voice/sessions/{id}/stream")
                || session.request_id != start.request_id
                || session.chat_session_id != start.chat_session_id
                || session.generation != start.generation
            {
                return Err(VoiceApiError::InvalidResponse);
            }
            let key = tokio_tungstenite::tungstenite::handshake::client::generate_key();
            let response = apply_request_context(
                client.get(format!("{}{}", endpoint.base_url(), session.stream_path)),
                endpoint,
            )
            .header("x-shiju-voice-connection-id", connection)
            .header("connection", "Upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", &key)
            .send()
            .await
            .map_err(|_| VoiceApiError::Transport)?;
            if response.status().as_u16() != 101 {
                return Err(VoiceApiError::Service(match response.status().as_u16() {
                    401 => VoiceErrorCode::Unauthorized,
                    403 => VoiceErrorCode::Forbidden,
                    _ => VoiceErrorCode::ProviderUnavailable,
                }));
            }
            let headers = response.headers();
            if headers.get("sec-websocket-accept").and_then(|h| h.to_str().ok())
                != Some(
                    tokio_tungstenite::tungstenite::handshake::derive_accept_key(key.as_bytes())
                        .as_str(),
                )
                || !headers
                    .get("upgrade")
                    .is_some_and(|h| h.as_bytes().eq_ignore_ascii_case(b"websocket"))
                || !headers
                    .get("connection")
                    .and_then(|h| h.to_str().ok())
                    .is_some_and(|h| h.split(',').any(|v| v.trim().eq_ignore_ascii_case("upgrade")))
                || headers.contains_key("sec-websocket-extensions")
                || headers.contains_key("sec-websocket-protocol")
            {
                return Err(VoiceApiError::InvalidResponse);
            }
            let socket = WebSocketStream::from_raw_socket(
                response.upgrade().await.map_err(|_| VoiceApiError::Transport)?,
                Role::Client,
                Some(
                    WebSocketConfig::default()
                        .write_buffer_size(0)
                        .max_message_size(Some(MAX_FRAME_BYTES))
                        .max_frame_size(Some(MAX_FRAME_BYTES))
                        .max_write_buffer_size(MAX_FRAME_BYTES * 2),
                ),
            )
            .await;
            let mut stream = Self {
                socket,
                generation: session.generation,
                session,
                sent: 0,
                received: 0,
                closed: false,
            };
            let ready = stream.next().await?;
            if let VoiceEvent::Error(error) = &ready.event {
                if error.request_id != start.request_id {
                    return Err(VoiceApiError::InvalidResponse);
                }
                return Err(VoiceApiError::Service(error.code));
            }
            if !matches!(ready.event, VoiceEvent::SessionReady { format, voice } if format == stream.session.format && voice == stream.session.voice)
            {
                return Err(VoiceApiError::InvalidResponse);
            }
            Ok(stream)
        };
        tokio::time::timeout(Duration::from_secs(30), work)
            .await
            .map_err(|_| VoiceApiError::Service(VoiceErrorCode::Timeout))?
    }
    pub fn session(&self) -> &VoiceSessionStarted {
        &self.session
    }
    pub fn generation(&self) -> VoiceCounter {
        self.generation
    }
    /// Preserve the generation attached before queuing capture/control data; stale
    /// samples must never be retagged after an interruption. UI/device queues must
    /// stay bounded and prioritize stop/interrupt.
    pub async fn send(&mut self, generation: VoiceCounter, event: VoiceEvent) -> Result<()> {
        if self.closed || generation != self.generation {
            return Err(VoiceApiError::Service(VoiceErrorCode::Cancelled));
        }
        if !matches!(
            event,
            VoiceEvent::ScreenShare(_)
                | VoiceEvent::AudioAppend(_)
                | VoiceEvent::InputCommit {}
                | VoiceEvent::InputMute { .. }
                | VoiceEvent::ResponseInterrupt { .. }
                | VoiceEvent::PlaybackProgress { .. }
                | VoiceEvent::SessionStop {}
        ) {
            return Err(VoiceApiError::Service(VoiceErrorCode::InvalidRequest));
        }
        event.validate().map_err(|_| VoiceApiError::Service(VoiceErrorCode::InvalidRequest))?;
        let seq =
            VoiceCounter::try_from(self.sent + 1).map_err(|_| VoiceApiError::InvalidResponse)?;
        let frame = VoiceFrame {
            voice_session_id: self.session.voice_session_id.clone(),
            generation,
            seq,
            event,
        };
        let text = serde_json::to_string(&frame).map_err(|_| VoiceApiError::InvalidResponse)?;
        if text.len() > MAX_FRAME_BYTES {
            return Err(VoiceApiError::Service(VoiceErrorCode::PayloadTooLarge));
        }
        self.sent += 1;
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            self.socket.send(Message::Text(text.into())),
        )
        .await;
        if !matches!(result, Ok(Ok(()))) {
            self.close().await;
            return Err(VoiceApiError::Transport);
        }
        Ok(())
    }
    pub async fn next(&mut self) -> Result<VoiceFrame> {
        if self.closed {
            return Err(VoiceApiError::Service(VoiceErrorCode::Cancelled));
        }
        loop {
            let message = match self.socket.next().await {
                Some(Ok(message)) => message,
                _ => {
                    self.close().await;
                    return Err(VoiceApiError::Transport);
                }
            };
            let frame = match message {
                Message::Text(text) => {
                    VoiceFrame::parse(text.as_bytes()).map_err(|_| VoiceApiError::InvalidResponse)
                }
                Message::Ping(_) | Message::Pong(_) => continue,
                _ => Err(VoiceApiError::InvalidResponse),
            };
            let Ok(frame) = frame else {
                self.close().await;
                return Err(VoiceApiError::InvalidResponse);
            };
            if frame.voice_session_id != self.session.voice_session_id
                || frame.seq.get() != self.received + 1
                || frame.generation < self.generation
                || (frame.generation != self.generation
                    && (!matches!(frame.event, VoiceEvent::GenerationChanged { .. })
                        || frame.generation.get() != self.generation.get() + 1))
                || matches!(
                    frame.event,
                    VoiceEvent::ScreenShare(_)
                        | VoiceEvent::AudioAppend(_)
                        | VoiceEvent::InputCommit {}
                        | VoiceEvent::InputMute { .. }
                        | VoiceEvent::ResponseInterrupt { .. }
                        | VoiceEvent::PlaybackProgress { .. }
                        | VoiceEvent::SessionStop {}
                )
            {
                self.close().await;
                return Err(VoiceApiError::InvalidResponse);
            }
            self.received = frame.seq.get();
            self.generation = frame.generation;
            if matches!(frame.event, VoiceEvent::SessionClosed { .. } | VoiceEvent::Error(_)) {
                self.close().await;
            }
            return Ok(frame);
        }
    }
    pub async fn close(&mut self) {
        self.closed = true;
        let _ = tokio::time::timeout(Duration::from_secs(2), self.socket.close(None)).await;
    }
}
#[cfg(test)]
#[path = "voice_stream_tests.rs"]
mod tests;
