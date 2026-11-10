//! Keep raw JSON and SSE tool results before the rmcp HTTP decoder narrows content types.
use futures_util::StreamExt;
use rmcp::{
    model::{ClientJsonRpcMessage, ClientRequest, RequestId, ServerInfo},
    transport::{
        common::client_side_sse::BoxedSseResponse,
        sse_client::{SseClient, SseTransportError},
        streamable_http_client::{
            AuthRequiredError, StreamableHttpClient, StreamableHttpError,
            StreamableHttpPostResponse,
        },
    },
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct HttpProtocol {
    initialize_id: Option<RequestId>,
    version: Option<String>,
}

impl HttpProtocol {
    /// Track the negotiated version and signal a JSON-RPC error for initialize.
    fn capture(&mut self, value: &Value) -> bool {
        let id = value.get("id").and_then(|id| serde_json::from_value(id.clone()).ok());
        if self.initialize_id.is_none() || self.initialize_id != id {
            return false;
        }
        if value.get("error").is_some() {
            return true;
        }
        let Some(result) = value.get("result") else { return false };
        let Ok(info) = serde_json::from_value::<ServerInfo>(result.clone()) else { return false };
        if crate::plugins::validate_mcp_server_protocol(Some(&info)).is_ok() {
            self.version = Some(info.protocol_version.to_string());
        }
        self.initialize_id = None;
        false
    }
}

#[derive(Clone)]
pub(super) struct HttpClient {
    client: reqwest_012::Client,
    responses: Arc<crate::plugins::RawMcpResponse>,
    protocol: Arc<Mutex<HttpProtocol>>,
    check_origin: Option<url::Origin>,
}

impl HttpClient {
    pub(super) fn new(
        client: reqwest_012::Client,
        responses: Arc<crate::plugins::RawMcpResponse>,
    ) -> Self {
        Self { client, responses, protocol: Default::default(), check_origin: None }
    }

    pub(super) fn with_check_origin(mut self, origin: url::Origin) -> Self {
        self.check_origin = Some(origin);
        self
    }

    fn request(&self, request: reqwest_012::RequestBuilder) -> reqwest_012::RequestBuilder {
        let protocol = self.protocol.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match &protocol.version {
            Some(version) => request.header("mcp-protocol-version", version),
            None => request,
        }
    }
}

fn events(
    stream: BoxedSseResponse,
    responses: Arc<crate::plugins::RawMcpResponse>,
    protocol: Arc<Mutex<HttpProtocol>>,
    reject_initialize_error: bool,
) -> BoxedSseResponse {
    stream
        .map(move |event| {
            event.and_then(|mut event| {
                if matches!(event.event.as_deref(), None | Some("") | Some("message")) {
                    if let Some(data) = &event.data {
                        if let Ok(mut value) = serde_json::from_str::<Value>(data) {
                            let initialize_error = protocol
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .capture(&value);
                            if initialize_error && reject_initialize_error {
                                // The streamable HTTP worker otherwise skips error events
                                // while waiting for the initialize result.
                                return Err(sse_stream::Error::Body(
                                    std::io::Error::other("MCP initialization rejected").into(),
                                ));
                            }
                            responses.capture(&value);
                            crate::plugins::preserve_mcp_wire_result(&mut value);
                            event.data = Some(value.to_string());
                        }
                    }
                }
                Ok(event)
            })
        })
        .boxed()
}

impl StreamableHttpClient for HttpClient {
    type Error = reqwest_012::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session: Option<Arc<str>>,
        auth: Option<String>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        if let ClientJsonRpcMessage::Request(request) = &message {
            if matches!(request.request, ClientRequest::InitializeRequest(_)) {
                let mut protocol =
                    self.protocol.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                protocol.initialize_id = Some(request.id.clone());
                protocol.version = None;
            }
        }
        let mut request = self.request(
            self.client.post(uri.as_ref()).header("accept", "text/event-stream, application/json"),
        );
        if let Some(session) = session {
            request = request.header("mcp-session-id", session.as_ref());
        }
        if let Some(auth) = auth {
            request = request.bearer_auth(auth);
        }
        let response = request.json(&message).send().await?;
        if response.status() == reqwest_012::StatusCode::UNAUTHORIZED {
            if let Some(header) = response.headers().get("www-authenticate") {
                let header = header.to_str().map_err(|_| {
                    StreamableHttpError::UnexpectedServerResponse(
                        "invalid www-authenticate header".into(),
                    )
                })?;
                return Err(StreamableHttpError::AuthRequired(AuthRequiredError {
                    www_authenticate_header: header.to_owned(),
                }));
            }
        }
        let status = response.status();
        let response = response.error_for_status()?;
        if matches!(status, reqwest_012::StatusCode::ACCEPTED | reqwest_012::StatusCode::NO_CONTENT)
        {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let kind = response
            .headers()
            .get("content-type")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
        match kind.as_deref() {
            Some(kind) if kind.starts_with("application/json") => {
                let value = response.json::<Value>().await?;
                self.protocol
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .capture(&value);
                let message = super::wire_stdio::decode(value, &self.responses)?;
                Ok(StreamableHttpPostResponse::Json(message, session))
            }
            Some(kind) if kind.starts_with("text/event-stream") => {
                let stream =
                    sse_stream::SseStream::from_byte_stream(response.bytes_stream()).boxed();
                Ok(StreamableHttpPostResponse::Sse(
                    events(
                        stream,
                        self.responses.clone(),
                        self.protocol.clone(),
                        self.check_origin.is_some(),
                    ),
                    session,
                ))
            }
            _ => Err(StreamableHttpError::UnexpectedContentType(kind)),
        }
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        auth: Option<String>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut request = self
            .request(self.client.delete(uri.as_ref()).header("mcp-session-id", session.as_ref()));
        if let Some(auth) = auth {
            request = request.bearer_auth(auth);
        }
        let response = request.send().await?;
        if response.status() != reqwest_012::StatusCode::METHOD_NOT_ALLOWED {
            response.error_for_status()?;
        }
        Ok(())
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        last: Option<String>,
        auth: Option<String>,
    ) -> Result<BoxedSseResponse, StreamableHttpError<Self::Error>> {
        let mut request = self.request(
            self.client
                .get(uri.as_ref())
                .header("accept", "text/event-stream")
                .header("mcp-session-id", session.as_ref()),
        );
        if let Some(last) = last {
            request = request.header("last-event-id", last);
        }
        if let Some(auth) = auth {
            request = request.bearer_auth(auth);
        }
        let response = request.send().await?;
        if response.status() == reqwest_012::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        let response = response.error_for_status()?;
        let kind = response
            .headers()
            .get("content-type")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
        if !kind.as_ref().is_some_and(|kind| kind.starts_with("text/event-stream")) {
            return Err(StreamableHttpError::UnexpectedContentType(kind));
        }
        let stream = sse_stream::SseStream::from_byte_stream(response.bytes_stream()).boxed();
        Ok(events(stream, self.responses.clone(), self.protocol.clone(), false))
    }
}

impl SseClient for HttpClient {
    type Error = reqwest_012::Error;

    async fn post_message(
        &self,
        uri: axum::http::Uri,
        message: ClientJsonRpcMessage,
        auth: Option<String>,
    ) -> Result<(), SseTransportError<Self::Error>> {
        if let Some(origin) = &self.check_origin {
            let allowed = reqwest_012::Url::parse(&uri.to_string()).is_ok_and(|url| {
                url.origin() == *origin && url.username().is_empty() && url.password().is_none()
            });
            if !allowed {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "MCP SSE message endpoint must stay on the configured origin",
                )
                .into());
            }
        }
        SseClient::post_message(&self.client, uri, message, auth).await
    }

    async fn get_stream(
        &self,
        uri: axum::http::Uri,
        last: Option<String>,
        auth: Option<String>,
    ) -> Result<BoxedSseResponse, SseTransportError<Self::Error>> {
        SseClient::get_stream(&self.client, uri, last, auth)
            .await
            .map(|stream| events(stream, self.responses.clone(), self.protocol.clone(), false))
    }
}
