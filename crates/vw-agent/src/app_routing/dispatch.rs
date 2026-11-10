//! Minimal authorized JSON request path for the application listener.

use super::{SharedTransport, route_auth::AuthorizedRoute};
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    response::Response,
};
use hyper::body::Incoming;
use serde_json::Value;
use std::time::Duration;
use vw_agent_apps::routing::private_params::filter_private_params;
use vw_api_types::agent_apps::AppId;

const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const CHAT_PATH: &str = "/v1/chat/completions";
const JSON: &str = "application/json";

/// The publication owner supplies this exact destination and its provider
/// credential. The listener never derives either from a client request.
pub(super) struct JsonTarget {
    pub(super) url: reqwest::Url,
    pub(super) authorization: HeaderValue,
    pub(super) transport: SharedTransport,
}

pub(super) async fn forward_json(
    request: Request<Incoming>,
    route: &AuthorizedRoute,
) -> Result<Response, StatusCode> {
    let (app, target, cancelled) = route.json_target().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if app != AppId::Codex || request.uri().path() != CHAT_PATH || request.uri().query().is_some() {
        return Err(StatusCode::NOT_FOUND);
    }
    if request.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    if !is_json(request.headers()) || request.headers().contains_key(header::CONTENT_ENCODING) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let bytes = tokio::time::timeout(
        Duration::from_secs(10),
        to_bytes(Body::new(request.into_body()), MAX_JSON_BYTES),
    )
    .await
    .map_err(|_| StatusCode::REQUEST_TIMEOUT)?
    .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let mut body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !body.is_object() || body.get("stream").is_some_and(|value| value.as_bool() != Some(false)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    filter_private_params(&mut body);
    let mut outgoing = reqwest::Request::new(Method::POST, target.url.clone());
    *outgoing.body_mut() =
        Some(serde_json::to_vec(&body).map_err(|_| StatusCode::BAD_REQUEST)?.into());
    outgoing.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
    let mut authorization = target.authorization.clone();
    authorization.set_sensitive(true);
    outgoing.headers_mut().insert(header::AUTHORIZATION, authorization);

    let mut response = target.transport.forward(&target.url, outgoing, cancelled).await?;
    if !response.status().is_success() || !is_json(response.headers()) {
        return Err(StatusCode::BAD_GATEWAY);
    }
    response.headers_mut().clear();
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
    Ok(response)
}

fn is_json(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    values.next().is_some_and(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(';').next().is_some_and(|media| media.trim().eq_ignore_ascii_case(JSON))
        })
    }) && values.next().is_none()
}
