//! Authorized Chat request path for the application listener.

use super::{
    SharedTransport,
    body_limits::{self, BodyError, BodyLimits, MAX_JSON_BYTES},
    cancellation::ClientDisconnect,
    decode_request::{ContentCodings, DecodeError},
    decode_response,
    header_wire::{self, HeaderWire},
    override_apply,
    route_auth::AuthorizedRoute,
    timeouts,
};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    response::Response,
};
use hyper::body::Incoming;
use serde_json::Value;
use std::time::Duration;
use tokio::{sync::OwnedSemaphorePermit, time::Instant};
use vw_agent_apps::routing::private_params::filter_private_params;
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};

const CHAT_PATH: &str = "/v1/chat/completions";
const JSON: &str = "application/json";
const SSE: &str = "text/event-stream";

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
    limits: &BodyLimits,
) -> Result<Response, StatusCode> {
    let (app, target, cancelled) = route.json_target().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if app != AppId::Codex || request.uri().path() != CHAT_PATH || request.uri().query().is_some() {
        return Err(StatusCode::NOT_FOUND);
    }
    if request.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    if request.extensions().get::<HeaderWire>().is_some_and(HeaderWire::has_conflicting_framing) {
        return Err(StatusCode::BAD_REQUEST);
    }
    body_limits::check_declared_length(request.headers(), MAX_JSON_BYTES)
        .map_err(request_body_status)?;
    // Inspect the entity coding before removing client-nominated hop headers.
    let codings = ContentCodings::from_headers(request.headers()).map_err(decode_status)?;
    header_wire::strip_hop_headers(request.headers_mut());
    if !is_content_type(request.headers(), JSON) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let (mut parts, incoming) = request.into_parts();
    let permit = limits.try_reserve().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let disconnect = parts.extensions.remove::<ClientDisconnect>();
    let bytes = tokio::time::timeout(Duration::from_secs(10), async {
        let bytes = body_limits::collect(Body::new(incoming), MAX_JSON_BYTES)
            .await
            .map_err(request_body_status)?;
        if let Some(disconnect) = disconnect {
            disconnect.request_body_read();
        }
        codings.decode(bytes, MAX_JSON_BYTES).await.map_err(decode_status)
    })
    .await
    .map_err(|_| StatusCode::REQUEST_TIMEOUT)??;
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    drop(bytes);
    if !body.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let streaming = match body.get("stream") {
        Some(Value::Bool(streaming)) => *streaming,
        None => false,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Native Chat uses the parsed document unchanged at the conversion step.
    let mut outgoing = prepare_request(&body, target)?;
    drop(body);
    let mut wire = parts.extensions.remove::<HeaderWire>().unwrap_or_default();
    wire.retain_allowed(&parts.headers, outgoing.headers_mut());
    #[cfg(target_os = "macos")]
    let timeouts = route.timeouts().await?;
    #[cfg(not(target_os = "macos"))]
    let timeouts = timeouts::AppTimeouts::default();
    if streaming {
        // The deadline observes decoded event data. Buffered JSON keeps its
        // existing coding negotiation and independent wire/decoded limits.
        outgoing
            .headers_mut()
            .insert(header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    }
    let forward = target.transport.forward_ordered(&target.url, outgoing, wire, cancelled);
    let started = Instant::now();
    if streaming {
        let deadline = started + Duration::from_secs(timeouts.first_byte_timeout_secs.into());
        let response = timeouts::before(deadline, forward).await??;
        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let (mut parts, body) = response.into_parts();
        let codings =
            ContentCodings::from_headers(&parts.headers).map_err(upstream_decode_status)?;
        header_wire::strip_hop_headers(&mut parts.headers);
        if !codings.is_identity() || !is_content_type(&parts.headers, SSE) {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let idle = Duration::from_secs(timeouts.idle_timeout_secs.into());
        let body = timeouts::stream_body(body, deadline, idle, permit).await?;
        parts.headers.clear();
        parts.headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(SSE));
        return Ok(Response::from_parts(parts, body));
    }

    let deadline = started + Duration::from_secs(timeouts.non_streaming_timeout_secs.into());
    timeouts::before(deadline, async {
        let response = forward.await?;
        buffer_json_response(response, permit).await
    })
    .await?
}

async fn buffer_json_response(
    response: Response,
    permit: OwnedSemaphorePermit,
) -> Result<Response, StatusCode> {
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let (mut parts, body) = response.into_parts();
    body_limits::check_declared_length(&parts.headers, MAX_JSON_BYTES)
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    // Read the actual coding before Connection nominations strip any fields.
    let codings = ContentCodings::from_headers(&parts.headers).map_err(upstream_decode_status)?;
    header_wire::strip_hop_headers(&mut parts.headers);
    if !is_content_type(&parts.headers, JSON) {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let body = decode_response::decode(body, codings, MAX_JSON_BYTES)
        .await
        .map_err(upstream_decode_status)?;
    parts.headers.clear();
    parts.headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
    Ok(Response::from_parts(parts, body_limits::buffered_response(body, permit)))
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
        Some(body_limits::encode_json(&body, MAX_JSON_BYTES).map_err(request_body_status)?.into());
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

fn request_body_status(error: BodyError) -> StatusCode {
    match error {
        BodyError::Malformed => StatusCode::BAD_REQUEST,
        BodyError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
    }
}

fn upstream_decode_status(error: DecodeError) -> StatusCode {
    tracing::debug!(reason = ?error, "Rejected upstream response body");
    StatusCode::BAD_GATEWAY
}

fn is_content_type(headers: &HeaderMap, expected: &str) -> bool {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    values.next().is_some_and(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(';').next().is_some_and(|media| media.trim().eq_ignore_ascii_case(expected))
        })
    }) && values.next().is_none()
}
