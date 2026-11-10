//! Minimal authorized JSON request path for the application listener.

use super::{
    SharedTransport,
    decode_request::{ContentCodings, DecodeError},
    header_wire::{self, HeaderWire},
    override_apply,
    route_auth::AuthorizedRoute,
};
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    response::Response,
};
use hyper::body::Incoming;
use serde_json::Value;
use std::time::Duration;
use vw_agent_apps::routing::private_params::filter_private_params;
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};

const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const CHAT_PATH: &str = "/v1/chat/completions";
const JSON: &str = "application/json";

/// The publication owner supplies the destination, credential and stored request
/// options for the same selected provider.
pub(super) struct JsonTarget {
    pub(super) url: reqwest::Url,
    pub(super) authorization: HeaderValue,
    pub(super) transport: SharedTransport,
    pub(super) request_options: ProviderRequestOptions,
}

pub(super) async fn forward_json(
    mut request: Request<Incoming>,
    route: &AuthorizedRoute,
) -> Result<Response, StatusCode> {
    let (app, target, cancelled) = route.json_target().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if app != AppId::Codex || request.uri().path() != CHAT_PATH || request.uri().query().is_some() {
        return Err(StatusCode::NOT_FOUND);
    }
    if request.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    // Inspect the entity coding before removing client-nominated hop headers.
    let codings = ContentCodings::from_headers(request.headers()).map_err(decode_status)?;
    header_wire::strip_hop_headers(request.headers_mut());
    if !is_json(request.headers()) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let (mut parts, incoming) = request.into_parts();
    let bytes = tokio::time::timeout(Duration::from_secs(10), async {
        let bytes = to_bytes(Body::new(incoming), MAX_JSON_BYTES)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        codings.decode(bytes, MAX_JSON_BYTES).await.map_err(decode_status)
    })
    .await
    .map_err(|_| StatusCode::REQUEST_TIMEOUT)??;
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !body.is_object() || body.get("stream").is_some_and(|value| value.as_bool() != Some(false)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Native Chat uses the parsed document unchanged at the conversion step.
    let mut outgoing = prepare_request(&body, target)?;
    let mut wire = parts.extensions.remove::<HeaderWire>().unwrap_or_default();
    wire.retain_allowed(&parts.headers, outgoing.headers_mut());
    let mut response =
        target.transport.forward_ordered(&target.url, outgoing, wire, cancelled).await?;
    if !response.status().is_success()
        || response
            .headers()
            .get_all(header::CONTENT_ENCODING)
            .iter()
            .any(|value| !value.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    header_wire::strip_hop_headers(response.headers_mut());
    if !is_json(response.headers()) {
        return Err(StatusCode::BAD_GATEWAY);
    }
    response.headers_mut().clear();
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
    Ok(response)
}

pub(super) fn prepare_request(
    converted: &Value,
    target: &JsonTarget,
) -> Result<reqwest::Request, StatusCode> {
    let mut body = converted.clone();
    filter_private_params(&mut body);
    let mut outgoing = reqwest::Request::new(Method::POST, target.url.clone());
    if let Some(overrides) = &target.request_options.local_proxy_request_overrides {
        override_apply::apply(&mut body, outgoing.headers_mut(), overrides)?;
    }
    header_wire::strip_hop_headers(outgoing.headers_mut());
    header_wire::strip_rebuilt_entity_headers(outgoing.headers_mut());
    *outgoing.body_mut() =
        Some(serde_json::to_vec(&body).map_err(|_| StatusCode::BAD_REQUEST)?.into());
    // The router owns the media type and the credential for this destination.
    outgoing.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
    let mut authorization = target.authorization.clone();
    authorization.set_sensitive(true);
    outgoing.headers_mut().insert(header::AUTHORIZATION, authorization);

    Ok(outgoing)
}

fn decode_status(error: DecodeError) -> StatusCode {
    match error {
        DecodeError::UnsupportedCoding => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        DecodeError::Corrupt => StatusCode::BAD_REQUEST,
        DecodeError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
    }
}

fn is_json(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    values.next().is_some_and(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(';').next().is_some_and(|media| media.trim().eq_ignore_ascii_case(JSON))
        })
    }) && values.next().is_none()
}
