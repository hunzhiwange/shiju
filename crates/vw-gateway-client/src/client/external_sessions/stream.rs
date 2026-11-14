use super::super::{AgentAppsReadError, GatewayClient};
use vw_api_types::external_session::{SessionStreamFrame, SessionStreamRequest, TranscriptChunk};

/// Dropping this reader closes the HTTP body and cancels the host read.
#[cfg(not(target_arch = "wasm32"))]
pub struct ExternalSessionStream {
    response: reqwest::Response,
    pending: Vec<u8>,
    read_id: String,
    sequence: usize,
    total: Option<usize>,
    received: usize,
    terminal: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl ExternalSessionStream {
    /// Validate framing and contiguous message offsets before exposing a chunk.
    pub async fn next_chunk(&mut self) -> Result<Option<TranscriptChunk>, AgentAppsReadError> {
        if self.terminal {
            return Ok(None);
        }
        let result = self.next_inner().await;
        if result.is_err() {
            self.terminal = true;
        }
        result
    }
    async fn next_inner(&mut self) -> Result<Option<TranscriptChunk>, AgentAppsReadError> {
        loop {
            if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                let line: Vec<_> = self.pending.drain(..=end).collect();
                let frame: SessionStreamFrame = serde_json::from_slice(&line).map_err(|_| {
                    AgentAppsReadError::Decode("invalid history stream frame".into())
                })?;
                if frame.read_id != self.read_id || frame.sequence != self.sequence {
                    return Err(AgentAppsReadError::Decode(
                        "history stream sequence mismatch".into(),
                    ));
                }
                self.sequence += 1;
                match &frame.chunk {
                    TranscriptChunk::Header { total, .. }
                        if self.total.is_none() && self.received == 0 && frame.sequence == 0 =>
                    {
                        self.total = Some(*total)
                    }
                    TranscriptChunk::Messages { start, messages }
                        if self.total.is_some()
                            && *start == self.received
                            && !messages.is_empty() =>
                    {
                        self.received =
                            self.received.checked_add(messages.len()).ok_or_else(|| {
                                AgentAppsReadError::Decode("history stream size overflow".into())
                            })?;
                        if self.received > self.total.unwrap_or(0) {
                            return Err(AgentAppsReadError::Decode(
                                "history stream exceeds header".into(),
                            ));
                        }
                    }
                    TranscriptChunk::Done { .. } if self.total == Some(self.received) => {
                        self.terminal = true
                    }
                    TranscriptChunk::Error { .. } => self.terminal = true,
                    _ => {
                        return Err(AgentAppsReadError::Decode(
                            "invalid history stream order".into(),
                        ));
                    }
                }
                return Ok(Some(frame.chunk));
            }
            let bytes = self
                .response
                .chunk()
                .await
                .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?
                .ok_or_else(|| {
                    AgentAppsReadError::Decode("history stream ended without terminal frame".into())
                })?;
            if self.pending.len().saturating_add(bytes.len()) > 128 * 1024 * 1024 {
                return Err(AgentAppsReadError::Decode("history stream frame too large".into()));
            }
            self.pending.extend_from_slice(&bytes);
        }
    }
}
impl GatewayClient {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn external_session_stream(
        &self,
        request: &SessionStreamRequest,
        token: &str,
    ) -> Result<ExternalSessionStream, AgentAppsReadError> {
        let response = self.external_session_stream_request("stream", request, token).await?;
        Ok(ExternalSessionStream {
            response,
            pending: Vec::new(),
            read_id: request.read_id.clone(),
            sequence: 0,
            total: None,
            received: 0,
            terminal: false,
        })
    }
    /// Resolves after host parsing has exited; dropping the reader also cancels.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn cancel_external_session_stream(
        &self,
        request: &SessionStreamRequest,
        token: &str,
    ) -> Result<(), AgentAppsReadError> {
        self.external_session_stream_request("stream/cancel", request, token).await?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    async fn external_session_stream_request(
        &self,
        action: &str,
        request: &SessionStreamRequest,
        token: &str,
    ) -> Result<reqwest::Response, AgentAppsReadError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| AgentAppsReadError::Transport("invalid application credential".into()))?;
        credential.set_sensitive(true);
        let pending = self
            .dynamic_host_client
            .post(format!("{}/v1/external-sessions/{action}", self.endpoint.base_url()))
            .header("x-agent-apps-token", credential)
            .json(request);
        let response = crate::http::apply_request_context(pending, &self.endpoint)
            .send()
            .await
            .map_err(|e| AgentAppsReadError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.json().await.unwrap_or(serde_json::Value::Null);
            return Err(AgentAppsReadError::Http { status, body });
        }
        Ok(response)
    }
}
