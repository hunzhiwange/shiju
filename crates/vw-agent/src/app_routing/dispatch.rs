//! Authorized Chat and Codex compaction paths for the application listener.

use super::{
    SharedTransport,
    body_limits::{self, BodyError, BodyLimits, MAX_JSON_BYTES},
    cancellation::ClientDisconnect,
    capacity::RequestPermit,
    decode_request::{ContentCodings, DecodeError},
    decode_response,
    header_wire::{self, HeaderWire},
    override_apply,
    protocol_errors::{Protocol, RouteError},
    route_auth::AuthorizedRoute,
    timeouts,
};
use crate::model_relay::responses_compaction;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    response::Response,
};
use hyper::body::Incoming;
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::Instant;
use vw_agent_apps::routing::private_params::filter_private_params;
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};

const CHAT_PATH: &str = "/v1/chat/completions";
const RESPONSES_PATH: &str = "/v1/responses";
const JSON: &str = "application/json";
const SSE: &str = "text/event-stream";

/// The publication owner supplies the destination, credential and stored request
/// options for the same selected provider.
pub(crate) struct JsonTarget {
    pub(crate) url: reqwest::Url,
    pub(crate) authorization: HeaderValue,
    pub(crate) transport: SharedTransport,
    pub(crate) request_options: ProviderRequestOptions,
}

pub(super) async fn forward_json(
    mut request: Request<Incoming>,
    route: &Arc<AuthorizedRoute>,
    limits: &BodyLimits,
    listener: SocketAddr,
    #[cfg(target_os = "macos")] egress: &super::outbound_config::EgressState,
) -> Result<Response, RouteError> {
    #[cfg(target_os = "macos")]
    route.check_provider(listener).await?;
    let app = route.json_app().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let responses = request.uri().path() == RESPONSES_PATH;
    let supported = match app {
        AppId::Codex => matches!(request.uri().path(), CHAT_PATH | RESPONSES_PATH),
        AppId::Claude => request.uri().path() == "/v1/messages",
        _ => false,
    };
    if !supported || request.uri().query().is_some() {
        return Err(StatusCode::NOT_FOUND.into());
    }
    if request.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED.into());
    }
    if request.extensions().get::<HeaderWire>().is_some_and(HeaderWire::has_conflicting_framing) {
        return Err(StatusCode::BAD_REQUEST.into());
    }
    body_limits::check_declared_length(request.headers(), MAX_JSON_BYTES)
        .map_err(request_body_status)?;
    // Inspect the entity coding before removing client-nominated hop headers.
    let codings = ContentCodings::from_headers(request.headers()).map_err(decode_status)?;
    header_wire::strip_hop_headers(request.headers_mut());
    if !is_content_type(request.headers(), JSON) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into());
    }
    let (mut parts, incoming) = request.into_parts();
    let buffer = limits.try_reserve().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
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
        return Err(StatusCode::BAD_REQUEST.into());
    }
    let streaming = match body.get("stream") {
        Some(Value::Bool(streaming)) => *streaming,
        None => false,
        _ => return Err(StatusCode::BAD_REQUEST.into()),
    };

    let (target, model, cancelled) =
        route.select_json(body.get("model").and_then(Value::as_str))?;

    // Wait only after the upload so disconnect detection can cancel a queued
    // request, and keep the listener-wide buffer reservation while it waits.
    let permit = route.capacity.acquire(buffer).await.ok_or(StatusCode::TOO_MANY_REQUESTS)?;
    #[cfg(target_os = "macos")]
    route.check_provider(listener).await?;
    let (mut outgoing, bridge) = if responses {
        if !target.url.path().trim_end_matches('/').ends_with("/chat/completions") {
            return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
        }
        let bridge = responses_compaction::Request::from_responses(body)?;
        let outgoing = prepare_request_inner(&bridge.chat, target, model, Some(&bridge))?;
        (outgoing, Some(bridge))
    } else {
        let outgoing = prepare_request(&body, target, model)?;
        drop(body);
        (outgoing, None)
    };
    let mut wire = parts.extensions.remove::<HeaderWire>().unwrap_or_default();
    wire.retain_allowed(&parts.headers, outgoing.headers_mut());
    #[cfg(target_os = "macos")]
    route.provider_headers(outgoing.headers_mut());
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
    #[cfg(target_os = "macos")]
    let selected_egress = route.egress(egress)?;
    let transport = &target.transport;
    #[cfg(target_os = "macos")]
    let transport = selected_egress.as_ref().map_or(transport, |egress| &egress.transport);
    let forward =
        transport.forward_ordered(&target.url, outgoing, wire, cancelled.clone(), listener);
    let started = Instant::now();
    if streaming {
        let deadline = started + Duration::from_secs(timeouts.first_byte_timeout_secs.into());
        let response = timeouts::before(deadline, forward).await??;
        RouteError::check_upstream(response.status())?;
        let (mut parts, body) = response.into_parts();
        let codings =
            ContentCodings::from_headers(&parts.headers).map_err(upstream_decode_status)?;
        header_wire::strip_hop_headers(&mut parts.headers);
        if !codings.is_identity() || !is_content_type(&parts.headers, SSE) {
            return Err(StatusCode::BAD_GATEWAY.into());
        }
        let idle = Duration::from_secs(timeouts.idle_timeout_secs.into());
        // The Responses bridge validates its Chat source in the shared decoder.
        let body = if bridge.is_none() {
            crate::model_relay::stream_observer::validated_body(
                body,
                if app == AppId::Claude {
                    crate::model_relay::generation::GenerationProtocol::Messages
                } else {
                    crate::model_relay::generation::GenerationProtocol::Chat
                },
                cancelled.clone(),
            )
        } else {
            body
        };
        let protocol = if app == AppId::Claude { Protocol::Messages } else { Protocol::Chat };
        let body = timeouts::stream_body(body, deadline, idle, permit, protocol).await?;
        let body = match bridge {
            Some(bridge) => bridge.response_body(body, cancelled),
            None => body,
        };
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
    permit: RequestPermit,
) -> Result<Response, RouteError> {
    RouteError::check_upstream(response.status())?;
    let (mut parts, body) = response.into_parts();
    body_limits::check_declared_length(&parts.headers, MAX_JSON_BYTES)
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    // Read the actual coding before Connection nominations strip any fields.
    let codings = ContentCodings::from_headers(&parts.headers).map_err(upstream_decode_status)?;
    header_wire::strip_hop_headers(&mut parts.headers);
    if !is_content_type(&parts.headers, JSON) {
        return Err(StatusCode::BAD_GATEWAY.into());
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
    exact_model: Option<&str>,
) -> Result<reqwest::Request, StatusCode> {
    prepare_request_inner(converted, target, exact_model, None)
}

fn prepare_request_inner(
    converted: &Value,
    target: &JsonTarget,
    exact_model: Option<&str>,
    bridge: Option<&responses_compaction::Request>,
) -> Result<reqwest::Request, StatusCode> {
    let mut body = converted.clone();
    filter_private_params(&mut body);
    let mut outgoing = reqwest::Request::new(Method::POST, target.url.clone());
    if let Some(overrides) = &target.request_options.local_proxy_request_overrides {
        override_apply::apply(&mut body, outgoing.headers_mut(), overrides)?;
    }
    if let Some(bridge) = bridge {
        bridge.apply_wire_controls(body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?);
        filter_private_params(&mut body);
    }
    if let Some(model) = exact_model {
        body["model"] = Value::String(model.into());
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
