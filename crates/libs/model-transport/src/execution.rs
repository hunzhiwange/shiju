//! Authorized request execution and byte-preserving Chat responses.
use axum::{http::{StatusCode, header}, response::Response};
use tokio_util::sync::CancellationToken;
use crate::upstream;

pub async fn send(
    client: &reqwest::Client,
    url: reqwest::Url,
    mut headers: header::HeaderMap,
    body: &serde_json::Value,
    model_headers: &std::collections::HashMap<String, String>,
    cancel: &CancellationToken,
) -> Result<reqwest::Response, StatusCode> {
    for (name, value) in model_headers {
        let name = header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if matches!(
            name.as_str(),
            "host"
                | "connection"
                | "content-length"
                | "transfer-encoding"
                | "content-type"
                | "authorization"
                | "x-api-key"
                | "x-goog-api-key"
                | "api-key"
        ) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let value = header::HeaderValue::from_str(value)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        headers.insert(name, value);
    }
    let outgoing = client
        .post(url.clone())
        .headers(headers)
        .json(body)
        .build()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let response = upstream::execute(client, &url, outgoing, cancel).await?;
    if !response.status().is_success() {
        // Keep authentication, access and request failures distinguishable from
        // transient provider failures without exposing upstream bodies or headers.
        return Err(if response.status().is_client_error() {
            response.status()
        } else {
            StatusCode::BAD_GATEWAY
        });
    }
    Ok(response)
}

pub fn chat_response(
    response: reqwest::Response,
    streaming: bool,
    cancel: CancellationToken,
) -> Result<Response, StatusCode> {
    if streaming
        && !response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let mut result = Response::builder().status(response.status());
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        result = result.header(header::CONTENT_TYPE, content_type);
    }
    // Preserve SSE deltas, parallel call indexes, finish reasons and usage byte-for-byte.
    let body = upstream::response_body(response, cancel.clone());
    let body = if streaming {
        crate::stream_observer::validated_body(
            body,
            model_protocol::generation::GenerationProtocol::Chat,
            cancel,
        )
    } else {
        body
    };
    result.body(body).map_err(|_| StatusCode::BAD_GATEWAY)
}
