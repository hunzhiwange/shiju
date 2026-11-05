//! Native session tool hosts. Binding tokens are capabilities, never URL parameters.
use futures_util::StreamExt;
use reqwest::{Method, RequestBuilder, Response, header::HeaderValue};
use vw_api_types::tools::{
    DynamicToolCallRequest, DynamicToolHostBinding, DynamicToolResponse,
    ReplaceDynamicToolsRequest, ReplaceDynamicToolsResponse,
};

use super::GatewayClient;
use crate::{
    http::{
        apply_request_context, directory_query, parse_json_response, response_error,
        transport_error,
    },
    stream::{GatewayStreamAttemptResult, parse_sse_frame, take_next_sse_event},
};

impl GatewayClient {
    /// Establish a server-owned binding. Registration does not grant tool permissions.
    pub async fn register_dynamic_tool_host(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<DynamicToolHostBinding, String> {
        let path = format!("{}/hosts", tools_path(session_id)?);
        let request = self.host_request(Method::POST, &path, directory, None)?;
        let response = self.send_host_request("POST", &path, request).await?;
        parse_json_response("POST", &self.endpoint, &path, response).await
    }

    /// Atomically replace the host catalog; the server remains the policy authority.
    pub async fn replace_dynamic_tools(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        body: &ReplaceDynamicToolsRequest,
    ) -> Result<ReplaceDynamicToolsResponse, String> {
        let path = tools_path(session_id)?;
        let request = self.host_request(Method::PUT, &path, directory, Some(binding))?.json(body);
        let response = self.send_host_request("PUT", &path, request).await?;
        parse_json_response("PUT", &self.endpoint, &path, response).await
    }

    /// Submit once. A transport error has an unknown outcome: do not blindly retry it.
    pub async fn respond_dynamic_tool_call(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        call_id: &str,
        body: &DynamicToolResponse,
    ) -> Result<(), String> {
        let path = format!("{}/calls/{}/response", tools_path(session_id)?, segment(call_id)?);
        let request = self.host_request(Method::POST, &path, directory, Some(binding))?.json(body);
        self.send_host_request("POST", &path, request).await?;
        Ok(())
    }

    /// Consume one SSE connection without reconnecting or replaying calls.
    ///
    /// Returning false, dropping this future, or a parse/transport error releases the
    /// response connection; the server then expires the host and cancels pending calls.
    /// EOF returns Incomplete, including EOF inside a frame. Re-register explicitly
    /// after disconnect. The shared gateway stream timeout bounds this connection.
    pub async fn stream_dynamic_tool_calls(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        mut on_call: impl FnMut(DynamicToolCallRequest) -> bool,
    ) -> Result<GatewayStreamAttemptResult, String> {
        let path = format!("{}/calls", tools_path(session_id)?);
        let request = self
            .host_request(Method::GET, &path, directory, Some(binding))?
            .header("accept", "text/event-stream")
            .timeout(std::time::Duration::from_secs(crate::stream::STREAM_REQUEST_TIMEOUT_SECS));
        let response = self.send_host_request("GET", &path, request).await?;
        if response.headers().get("content-type").and_then(|value| value.to_str().ok()).is_none_or(
            |value| value.split(';').next().unwrap_or_default().trim() != "text/event-stream",
        ) {
            return Err("dynamic tool calls require text/event-stream".into());
        }
        let mut stream = response.bytes_stream();
        let mut pending = Vec::new();
        let mut buffer = String::new();
        while let Some(chunk) = stream.next().await {
            pending.extend_from_slice(&chunk.map_err(|error| error.to_string())?);
            // Preserve UTF-8 code points split across HTTP chunks.
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                Err(_) => return Err("invalid UTF-8 in dynamic tool calls".into()),
            };
            buffer.push_str(
                std::str::from_utf8(&pending[..valid]).map_err(|error| error.to_string())?,
            );
            pending.drain(..valid);
            if buffer.len() + pending.len() > 4 * 1024 * 1024 {
                return Err("dynamic tool call frame exceeds 4 MiB".into());
            }
            while let Some(raw) = take_next_sse_event(&mut buffer) {
                let Some(frame) = parse_sse_frame(&raw)? else { continue };
                let event = raw
                    .lines()
                    .filter_map(|line| line.strip_prefix("event:"))
                    .map(str::trim)
                    .next_back();
                if event != Some("dynamic_tool.call") {
                    return Err("unsupported dynamic tool SSE event".into());
                }
                let call = serde_json::from_str(&frame.data).map_err(|error| error.to_string())?;
                if !on_call(call) {
                    return Ok(GatewayStreamAttemptResult::CallerStopped);
                }
            }
        }
        Ok(GatewayStreamAttemptResult::Incomplete)
    }

    fn host_request(
        &self,
        method: Method,
        path: &str,
        directory: Option<&str>,
        binding: Option<&DynamicToolHostBinding>,
    ) -> Result<RequestBuilder, String> {
        let mut request = apply_request_context(
            self.dynamic_host_client
                .request(method, format!("{}{}", self.endpoint.base_url(), path))
                .query(&directory_query(directory)),
            &self.endpoint,
        );
        if let Some(binding) = binding {
            if binding.binding_token.is_empty() {
                return Err("empty dynamic tool host binding".into());
            }
            let mut token = HeaderValue::from_str(&binding.binding_token)
                .map_err(|_| "invalid dynamic tool host binding header".to_string())?;
            token.set_sensitive(true);
            request = request.header("x-vibewindow-tool-host", token);
        }
        Ok(request)
    }

    async fn send_host_request(
        &self,
        method: &str,
        path: &str,
        request: RequestBuilder,
    ) -> Result<Response, String> {
        let response = request
            .send()
            .await
            .map_err(|error| transport_error(method, &self.endpoint, path, error))?;
        if !response.status().is_success() {
            return Err(response_error(method, &self.endpoint, path, response).await);
        }
        Ok(response)
    }
}

fn tools_path(session_id: &str) -> Result<String, String> {
    Ok(format!("/v1/session/{}/dynamic-tools", segment(session_id)?))
}

fn segment(value: &str) -> Result<String, String> {
    if value.trim().is_empty() || matches!(value, "." | "..") {
        return Err("dynamic tool session/call ID must not be empty or a dot segment".into());
    }
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte))
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    Ok(encoded)
}
