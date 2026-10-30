//! Keep raw JSON and SSE tool results before the rmcp HTTP decoder narrows content types.
use futures_util::StreamExt;
use rmcp::{
    model::ClientJsonRpcMessage,
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
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct HttpClient(
    pub(super) reqwest_012::Client,
    pub(super) Arc<crate::plugins::RawMcpResponse>,
);

fn events(
    stream: BoxedSseResponse,
    responses: Arc<crate::plugins::RawMcpResponse>,
) -> BoxedSseResponse {
    stream
        .map(move |event| {
            event.map(|mut event| {
                if matches!(event.event.as_deref(), None | Some("") | Some("message")) {
                    if let Some(data) = &event.data {
                        if let Ok(mut value) = serde_json::from_str::<Value>(data) {
                            responses.capture(&value);
                            crate::plugins::preserve_mcp_wire_result(&mut value);
                            event.data = Some(value.to_string());
                        }
                    }
                }
                event
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
        let mut request =
            self.0.post(uri.as_ref()).header("accept", "text/event-stream, application/json");
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
                let message = super::wire_stdio::decode(response.json::<Value>().await?, &self.1)?;
                Ok(StreamableHttpPostResponse::Json(message, session))
            }
            Some(kind) if kind.starts_with("text/event-stream") => {
                let stream =
                    sse_stream::SseStream::from_byte_stream(response.bytes_stream()).boxed();
                Ok(StreamableHttpPostResponse::Sse(events(stream, self.1.clone()), session))
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
        StreamableHttpClient::delete_session(&self.0, uri, session, auth).await
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        last: Option<String>,
        auth: Option<String>,
    ) -> Result<BoxedSseResponse, StreamableHttpError<Self::Error>> {
        StreamableHttpClient::get_stream(&self.0, uri, session, last, auth)
            .await
            .map(|stream| events(stream, self.1.clone()))
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
        SseClient::post_message(&self.0, uri, message, auth).await
    }

    async fn get_stream(
        &self,
        uri: axum::http::Uri,
        last: Option<String>,
        auth: Option<String>,
    ) -> Result<BoxedSseResponse, SseTransportError<Self::Error>> {
        SseClient::get_stream(&self.0, uri, last, auth)
            .await
            .map(|stream| events(stream, self.1.clone()))
    }
}
