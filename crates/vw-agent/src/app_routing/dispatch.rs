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

const JSON: &str = "application/json";
const SSE: &str = "text/event-stream";

/// The publication owner supplies the destination, credential and stored request
/// options for the same selected provider.
pub(crate) struct JsonTarget {
    #[cfg(target_os = "macos")]
    pub(crate) managed: Option<std::sync::Arc<super::adapter_selection::ManagedCredential>>,
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
    #[cfg(target_os = "macos")]
    if app == AppId::ClaudeDesktop && request.uri().path() == "/claude-desktop/v1/models" {
        if request.uri().query().is_some() {
            return Err(StatusCode::NOT_FOUND.into());
        }
        if request.method() != Method::GET {
            return Err(StatusCode::METHOD_NOT_ALLOWED.into());
        }
        if request.headers().contains_key(header::TRANSFER_ENCODING)
            || request.headers().get(header::CONTENT_LENGTH).is_some_and(|value| value != "0")
        {
            return Err(StatusCode::BAD_REQUEST.into());
        }
        if let Some(disconnect) = request.extensions().get::<ClientDisconnect>() {
            disconnect.request_body_read();
        }
        let buffer = limits.try_reserve().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let permit = route.capacity.acquire(buffer).await.ok_or(StatusCode::TOO_MANY_REQUESTS)?;
        let catalog = route.desktop_discovery().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let bytes = body_limits::encode_json(&catalog, MAX_JSON_BYTES)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut response = Response::new(body_limits::buffered_response(bytes.into(), permit));
        response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON));
        response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        return Ok(response);
    }
    let gemini = (app == AppId::Gemini)
        .then(|| crate::model_relay::native_gemini::Request::parse(request.uri()))
        .flatten();
    let compact = crate::model_relay::compaction::is_path(request.uri().path());
    let responses = compact || crate::model_relay::native_responses::is_path(request.uri().path());
    let supported = match app {
        AppId::Codex => {
            crate::model_relay::native_chat::is_path(request.uri().path())
                || responses
        }
        AppId::Gemini => gemini.is_some(),
        AppId::Claude => crate::model_relay::native_messages::is_path(request.uri().path()),
        AppId::ClaudeDesktop => request.uri().path() == "/claude-desktop/v1/messages",
        _ => false,
    };
    if !supported || (gemini.is_none() && request.uri().query().is_some()) {
        return Err(StatusCode::NOT_FOUND.into());
    }
    if responses && request.method() == Method::GET {
        use axum::response::IntoResponse;
        return Ok((StatusCode::UPGRADE_REQUIRED, axum::Json(serde_json::json!({"error":{"type":"invalid_request_error","code":"websocket_not_supported","message":"Use HTTP for Responses"}}))).into_response());
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
    let streaming = if let Some(gemini) = &gemini {
        gemini.streaming
    } else {
        match body.get("stream") {
            Some(Value::Bool(streaming)) => *streaming,
            None => false,
            _ => return Err(StatusCode::BAD_REQUEST.into()),
        }
    };

    if compact && streaming {
        return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
    }

    // Inspect original Messages history before conversion, overrides or any retry owner.
    if matches!(app, AppId::Claude | AppId::ClaudeDesktop)
        && vw_agent_apps::routing::classifier_guard::classifier_retry_loop(&body)
    {
        return Err(RouteError::ClassifierLoop);
    }

    #[cfg(target_os = "macos")]
    let failover = super::failover_execution::prepare(Arc::clone(route)).await?;
    #[cfg(target_os = "macos")]
    let failover_body = failover.as_ref().map(|_| body.clone());

    let client_model = gemini
        .as_ref()
        .map(|gemini| gemini.model.clone())
        .or_else(|| body.get("model").and_then(Value::as_str).map(str::to_owned));
    let (target, model, cancelled) = route.select_json(client_model.as_deref())?;

    #[cfg(target_os = "macos")]
    let mapped_model = if model.is_none() {
        client_model.as_deref().map(|model| {
            route.provider.as_ref().map_or_else(
                || vw_agent_apps::routing::target_model::resolve(app, &Value::Null, model),
                |provider| provider.model(model),
            )
        })
    } else {
        None
    };
    #[cfg(target_os = "macos")]
    let model =
        model.or(mapped_model.as_deref().filter(|mapped| Some(*mapped) != client_model.as_deref()));

    // Wait only after the upload so disconnect detection can cancel a queued
    // request, and keep the listener-wide buffer reservation while it waits.
    let permit = route.capacity.acquire(buffer).await.ok_or(StatusCode::TOO_MANY_REQUESTS)?;
    #[cfg(target_os = "macos")]
    route.check_provider(listener).await?;
    let messages_responses = app == AppId::Claude
        && target.url.path().trim_end_matches('/').ends_with("/responses");
    let hosted_search = if messages_responses {
        crate::model_relay::hosted_search::request(&body)?.2
    } else {
        Default::default()
    };
    let messages_gemini = matches!(app, AppId::Claude | AppId::ClaudeDesktop)
        && (target.url.path().ends_with(":generateContent")
            || target.url.path().ends_with(":streamGenerateContent"));
    #[cfg(target_os = "macos")]
    let native_messages = if app == AppId::ClaudeDesktop {
        route.desktop_native_messages(client_model.as_deref())?
    } else {
        app == AppId::Claude
            && !messages_responses
            && !target.url.path().trim_end_matches('/').ends_with("/chat/completions")
    };
    #[cfg(not(target_os = "macos"))]
    let native_messages = app == AppId::Claude
        && !messages_responses
        && !target.url.path().trim_end_matches('/').ends_with("/chat/completions");
    let native_messages = native_messages && !messages_gemini;
    let compact_url = compact
        .then(|| crate::model_relay::compaction::url(&target.url))
        .transpose()?;
    let native_responses =
        responses && (compact || target.url.path().trim_end_matches('/').ends_with("/responses"));
    if app == AppId::Codex
        && !responses
        && target.url.path().trim_end_matches('/').ends_with("/responses")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
    }
    let gemini_shadow = if messages_gemini {
        Some(crate::model_relay::gemini_shadow::Context {
            store: Arc::clone(&route.gemini_shadow),
            scope: route.response_scope(target)?,
            cancelled: cancelled.clone(),
            session: body.pointer("/metadata/user_id").and_then(Value::as_str).unwrap_or("").to_owned(),
        })
    } else { None };
    let response_scope = if native_responses { Some(route.response_scope(target)?) } else { None };
    let (mut outgoing, bridge) = if native_responses {
        crate::model_relay::native_responses::check(
            &body,
            &route.references,
            response_scope.as_ref().ok_or(StatusCode::FORBIDDEN)?,
        )?;
        (prepare_request(&body, target, model)?, None)
    } else if responses {
        let anthropic = target.url.path().trim_end_matches('/').ends_with("/messages");
        if !anthropic && !target.url.path().trim_end_matches('/').ends_with("/chat/completions") {
            return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
        }
        let mut body = body;
        let history = crate::model_relay::responses_history::Context::prepare(
            Arc::clone(&route.responses_history), route.response_scope(target)?, &mut body,
        )?;
        let bridge = if anthropic {
            responses_compaction::Request::from_responses_anthropic(body)?
        } else {
            responses_compaction::Request::from_responses(body)?
        }.with_history(history);
        let outgoing = prepare_request_inner(&bridge.chat, target, model, Some(&bridge))?;
        let mut outgoing = outgoing;
        if anthropic {
            outgoing.headers_mut().insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            if !target.authorization.as_bytes().starts_with(b"Bearer ") {
                outgoing.headers_mut().remove(header::AUTHORIZATION);
                outgoing.headers_mut().insert("x-api-key", target.authorization.clone());
            }
        }
        (outgoing, Some(bridge))
    } else {
        #[cfg(target_os = "macos")]
        let body = if app == AppId::ClaudeDesktop && !messages_gemini {
            super::desktop_routing::request(&body, native_messages)?
        } else if messages_gemini {
            body
        } else if messages_responses {
            crate::model_relay::messages_responses_request::to_responses(&body)?
        } else if app == AppId::Claude && !native_messages {
            crate::model_relay::messages_chat_request::to_chat(&body)?
        } else {
            body
        };
        let body = if gemini.is_some() {
            crate::model_relay::native_gemini::prepare(&body)?
        } else {
            body
        };
        let outgoing = prepare_request(&body, target, if gemini.is_some() { None } else { model })?;
        drop(body);
        (outgoing, None)
    };
    apply_cache(&mut outgoing, route, target, &parts.headers)?;
    if let Some(shadow) = &gemini_shadow {
        let bytes = outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        shadow.replay(&mut body)?;
        *outgoing.body_mut() = Some(body_limits::encode_json(&body, MAX_JSON_BYTES).map_err(request_body_status)?.into());
    }
    if native_responses {
        let bytes = outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        crate::model_relay::xai_responses::prepare(&mut body, &target.url);
        *outgoing.body_mut() = Some(body_limits::encode_json(&body, MAX_JSON_BYTES).map_err(request_body_status)?.into());
    }
    #[cfg(target_os = "macos")]
    if let Some(provider) = &route.provider {
        let bytes = outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        let effective_model = body.get("model").and_then(Value::as_str)
            .or(model).or(client_model.as_deref()).unwrap_or("").to_owned();
        if provider.sanitize_images(&mut body, &effective_model) > 0 {
            *outgoing.body_mut() = Some(body_limits::encode_json(&body, MAX_JSON_BYTES).map_err(request_body_status)?.into());
            outgoing.headers_mut().remove(header::CONTENT_LENGTH);
        }
    }
    if let Some(scope) = &response_scope {
        let bytes =
            outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        crate::model_relay::native_responses::check(&body, &route.references, scope)?;
    }
    let gemini_url = if messages_gemini && streaming {
        Some(crate::model_relay::messages_gemini_stream::stream_url(&target.url)?)
    } else {
        gemini
            .as_ref()
            .map(|gemini| gemini.url(&target.url, model.unwrap_or(&gemini.model)))
            .transpose()?
    };
    if compact {
        let bytes = outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
        let body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        if body.get("stream").is_some_and(|value| value != &Value::Bool(false)) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
        }
    }
    let endpoint_url = compact_url.as_ref().or(gemini_url.as_ref());
    if let Some(url) = endpoint_url {
        *outgoing.url_mut() = url.clone();
    }
    let authorized_url = endpoint_url.unwrap_or(&target.url);
    let mut wire = parts.extensions.remove::<HeaderWire>().unwrap_or_default();
    wire.retain_allowed(&parts.headers, outgoing.headers_mut());
    #[cfg(target_os = "macos")]
    route.provider_headers(outgoing.headers_mut(), client_model.as_deref())?;
    #[cfg(target_os = "macos")]
    if app == AppId::ClaudeDesktop {
        super::desktop_routing::headers(native_messages, outgoing.headers_mut());
    }
    #[cfg(target_os = "macos")]
    let timeouts = route.timeouts().await?;
    #[cfg(target_os = "macos")]
    if let Some(credential) = &target.managed {
        route.validate_managed(credential)?;
        let selected = credential.resolve().await.map_err(|_| StatusCode::UNAUTHORIZED)?;
        // Endpoint changes need a new authorized publication, never a silent redirect.
        if selected.url != target.url {
            return Err(StatusCode::CONFLICT.into());
        }
        for name in ["authorization", "x-api-key", "x-goog-api-key", "chatgpt-account-id"] {
            outgoing.headers_mut().remove(name);
        }
        if credential.account_adapter.provider_type()
            == super::adapter_selection::ProviderType::CodexOAuth
        {
            crate::model_relay::codex_oauth_transport::apply(&mut outgoing, &selected.headers)?;
        }
        if credential.account_adapter.provider_type()
            == super::adapter_selection::ProviderType::XaiOAuth
        {
            crate::model_relay::xai_oauth_transport::apply(&mut outgoing, &selected.headers)?;
        }
        outgoing.headers_mut().extend(selected.headers);
    }

    #[cfg(not(target_os = "macos"))]
    let timeouts = timeouts::AppTimeouts::default();
    if streaming {
        // The deadline observes decoded event data. Buffered JSON keeps its
        // existing coding negotiation and independent wire/decoded limits.
        outgoing
            .headers_mut()
            .insert(header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    }
    // Final authentication follows the authorized destination/source after overrides.
    // Managed OAuth was resolved above by the account owner; never refresh here.
    let managed_gemini = {
        #[cfg(target_os = "macos")]
        { target.managed.as_ref().is_some_and(|credential| {
            credential.account_adapter.provider_type()
                == super::adapter_selection::ProviderType::GeminiCli
        }) }
        #[cfg(not(target_os = "macos"))]
        { false }
    };
    if managed_gemini {
        crate::model_relay::gemini_oauth_transport::apply_oauth(&mut outgoing)?;
    } else if outgoing.url().path().ends_with(":generateContent")
        || outgoing.url().path().ends_with(":streamGenerateContent")
    {
        crate::model_relay::gemini_oauth_transport::apply_api_key(
            &mut outgoing, &target.authorization,
        )?;
    }
    #[cfg(target_os = "macos")]
    let selected_egress = route.egress(egress)?;
    let transport = &target.transport;
    #[cfg(target_os = "macos")]
    let transport = selected_egress.as_ref().map_or(transport, |egress| &egress.transport);
    let started = Instant::now();
    let forward = async {
        #[cfg(target_os = "macos")]
        if let (Some(plan), Some(body)) = (&failover, &failover_body) {
            let duration = if streaming {
                timeouts.first_byte_timeout_secs
            } else {
                timeouts.non_streaming_timeout_secs
            };
            return plan
                .forward(super::failover_execution::AttemptRequest {
                    route,
                    body,
                    headers: &parts.headers,
                    wire: &wire,
                    transport: selected_egress.as_ref().map(|egress| &egress.transport),
                    listener,
                    deadline: started + Duration::from_secs(duration.into()),
                })
                .await;
        }
        if compact {
            Ok(transport.forward_ordered(authorized_url, outgoing, wire, cancelled.clone(), listener).await?)
        } else {
            forward_rectified(transport, authorized_url, outgoing, wire, cancelled.clone(), listener).await
        }
    };
    if streaming {
        let deadline = started + Duration::from_secs(timeouts.first_byte_timeout_secs.into());
        #[cfg(target_os = "macos")]
        let response = if failover.is_some() {
            // The retry owner awaits headers and the first valid event under
            // this same deadline, recording its circuit outcome before return.
            forward.await?
        } else {
            timeouts::before(deadline, forward).await??
        };
        #[cfg(not(target_os = "macos"))]
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
        let body = crate::model_relay::sse_framing::resident_body(body);
        let body = if messages_gemini {
            crate::model_relay::messages_gemini_stream::response_body_with_shadow(
                body,
                client_model.clone().ok_or(StatusCode::BAD_REQUEST)?,
                cancelled.clone(),
                gemini_shadow.clone(),
            )
        } else {
            body
        };
        let body = if native_responses
            && !matches!(target.url.host_str(), Some("api.openai.com" | "chatgpt.com" | "chat.openai.com"))
        {
            crate::model_relay::late_arguments::body(body)
        } else {
            body
        };
        // The Responses bridge validates its Chat source in the shared decoder.
        let body = if bridge.is_none() {
            crate::model_relay::stream_observer::validated_body(
                body,
                if gemini.is_some() {
                    crate::model_relay::generation::GenerationProtocol::Gemini
                } else if native_messages || messages_gemini {
                    crate::model_relay::generation::GenerationProtocol::Messages
                } else if native_responses || messages_responses {
                    crate::model_relay::generation::GenerationProtocol::Responses
                } else {
                    crate::model_relay::generation::GenerationProtocol::Chat
                },
                cancelled.clone(),
            )
        } else {
            body
        };
        let protocol = if gemini.is_some() {
            Protocol::Gemini
        } else if native_messages || messages_gemini {
            Protocol::Messages
        } else if native_responses || messages_responses {
            Protocol::Responses
        } else {
            Protocol::Chat
        };
        let body = if let Some(scope) = response_scope {
            crate::model_relay::native_responses::stream(body, Arc::clone(&route.references), scope)
        } else {
            body
        };
        let body = timeouts::stream_body(body, deadline, idle, permit, protocol).await?;
        let body = match bridge {
            Some(bridge) => bridge.response_body(body, cancelled.clone()),
            None => body,
        };
        parts.headers.clear();
        parts.headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(SSE));
        if messages_gemini {
            return Ok(Response::from_parts(parts, body));
        }
        return finish_messages(
            Response::from_parts(parts, body),
            app,
            client_model,
            true,
            native_messages,
            messages_responses,
            hosted_search.clone(),
            cancelled,
        )
        .await;
    }

    let deadline = started + Duration::from_secs(timeouts.non_streaming_timeout_secs.into());
    timeouts::before(deadline, async {
        let response = forward.await?;
        let response = buffer_json_response(
            response,
            permit,
            compact,
            response_scope.as_ref().map(|scope| (&*route.references, scope)),
        )
        .await?;
        if let Some(bridge) = bridge {
            let (parts, body) = response.into_parts();
            let bytes = axum::body::to_bytes(body, MAX_JSON_BYTES)
                .await.map_err(|_| StatusCode::BAD_GATEWAY)?;
            let value: Value = serde_json::from_slice(&bytes)
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
            let converted = bridge.response_json(&value)?;
            Ok(Response::from_parts(parts, Body::from(converted.to_string())))
        } else if messages_gemini {
            crate::model_relay::messages_gemini_result::response_with_shadow(
                response,
                &client_model.ok_or(StatusCode::BAD_REQUEST)?,
                cancelled.clone(),
                MAX_JSON_BYTES,
                gemini_shadow,
            )
            .await
            .map_err(Into::into)
        } else {
            finish_messages(
                response,
                app,
                client_model,
                false,
                native_messages,
                messages_responses,
                hosted_search.clone(),
                cancelled.clone(),
            )
            .await
        }
    })
    .await?
}

async fn finish_messages(
    response: Response,
    app: AppId,
    model: Option<String>,
    streaming: bool,
    native: bool,
    responses: bool,
    hosted_search: crate::model_relay::hosted_search::Search,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Response, RouteError> {
    if (app == AppId::ClaudeDesktop || app == AppId::Claude) && !native && !streaming {
        let model = model.ok_or(StatusCode::BAD_REQUEST)?;
        let (parts, body) = response.into_parts();
        let bytes = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT.into()),
            bytes = axum::body::to_bytes(body, MAX_JSON_BYTES) => bytes.map_err(|_| StatusCode::BAD_GATEWAY)?,
        };
        let body: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)?;
        let converted = if responses {
            crate::model_relay::messages_responses_result::with_search(&body, &model, &hosted_search)?
        } else {
            crate::model_relay::messages_chat_result::from_chat(&body, &model)?
        };
        Ok(Response::from_parts(parts, Body::from(converted.to_string())))
    } else if (app == AppId::ClaudeDesktop || app == AppId::Claude) && !native && streaming {
        let model = model.ok_or(StatusCode::BAD_REQUEST)?;
        if responses {
            crate::model_relay::messages_responses_stream::with_search(response, model, cancel, hosted_search)
                .await
                .map_err(Into::into)
        } else {
            crate::model_relay::messages_stream::resident_response_body(response, model, cancel)
                .await
                .map_err(Into::into)
        }
    } else if app == AppId::ClaudeDesktop || app == AppId::Claude && !native {
        crate::model_relay::messages_stream::response_body(
            response,
            model.ok_or(StatusCode::BAD_REQUEST)?,
            streaming,
            native,
            cancel,
        )
        .await
        .map_err(Into::into)
    } else {
        Ok(response)
    }
}

async fn buffer_json_response(
    response: Response,
    permit: RequestPermit,
    preserve_status: bool,
    references: Option<(
        &super::reference_scope::References,
        &super::reference_scope::ReferenceScope,
    )>,
) -> Result<Response, RouteError> {
    if !preserve_status {
        RouteError::check_upstream(response.status())?;
    }
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
    if let Some((references, scope)) = references.filter(|_| parts.status.is_success()) {
        let value: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_GATEWAY)?;
        crate::model_relay::native_responses::remember(&value, references, scope)?;
    }
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
    // The authorized target owns the Gemini model/action. Only Messages
    // payloads use this resident conversion; native Gemini has contents.
    if body.get("messages").is_some()
        && (target.url.path().ends_with(":generateContent")
            || target.url.path().ends_with(":streamGenerateContent"))
    {
        body = crate::model_relay::messages_gemini_request::convert(&body)?;
    }
    vw_agent_apps::routing::thinking_options::normalize_messages(
        &mut body,
        target.url.host_str().unwrap_or_default(),
        target.url.path(),
    );
    if body.get("input").is_some()
        && (target.url.path().trim_end_matches('/').ends_with("/responses")
            || target.url.path().trim_end_matches('/').ends_with("/responses/compact"))
    {
        let official = matches!(target.url.host_str(),
            Some("api.openai.com" | "chatgpt.com" | "chat.openai.com"));
        vw_agent_apps::routing::compaction_replay::prepare_native(&mut body, official);
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

pub(super) fn is_content_type(headers: &HeaderMap, expected: &str) -> bool {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    values.next().is_some_and(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(';').next().is_some_and(|media| media.trim().eq_ignore_ascii_case(expected))
        })
    }) && values.next().is_none()
}

/// Inspect only bounded HTTP error bodies before any client output. The repaired
/// request keeps the exact authorized URL, credentials, transport and cancellation.
async fn forward_rectified(
    transport: &SharedTransport,
    url: &reqwest::Url,
    outgoing: reqwest::Request,
    wire: HeaderWire,
    cancelled: tokio_util::sync::CancellationToken,
    listener: SocketAddr,
) -> Result<Response, RouteError> {
    let retry = outgoing.try_clone();
    let response = transport.forward_ordered(url, outgoing, wire.clone(), cancelled.clone(), listener).await?;
    if !matches!(response.status().as_u16(), 400 | 404 | 415 | 422 | 501 | 502) {
        return Ok(response);
    }
    let Some(mut retry) = retry else { return Ok(response); };
    let Some(bytes) = retry.body().and_then(reqwest::Body::as_bytes) else { return Ok(response); };
    let input: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let (mut parts, body) = response.into_parts();
    let codings = ContentCodings::from_headers(&parts.headers).map_err(upstream_decode_status)?;
    let bytes = decode_response::decode(body, codings, MAX_JSON_BYTES).await.map_err(upstream_decode_status)?;
    let repaired = crate::model_relay::signature_rectify::repair(&input, parts.status, &bytes, url);
    let repaired = match repaired {
        Ok(value) => value,
        Err(()) => return Err(RouteError::UnreadableAgentTask),
    };
    // Share the existing single replay allowance with signature repair.
    // Inspect the decoded error message only for the Messages endpoint.
    let repaired = repaired.or_else(|| {
        url.path().trim_end_matches('/').ends_with("/messages").then(|| {
            vw_agent_apps::routing::budget_rectify::repair(&input, parts.status.as_u16(), &bytes)
        }).flatten()
    });
    let repaired = repaired.or_else(|| {
        vw_agent_apps::routing::media_sanitize::repair(&input, parts.status.as_u16(), &bytes)
    });
    let Some(repaired) = repaired else {
        parts.headers.remove(header::CONTENT_ENCODING);
        parts.headers.remove(header::CONTENT_LENGTH);
        return Ok(Response::from_parts(parts, Body::from(bytes)));
    };
    *retry.body_mut() = Some(body_limits::encode_json(&repaired, MAX_JSON_BYTES).map_err(request_body_status)?.into());
    retry.headers_mut().remove(header::CONTENT_LENGTH);
    // Deliberately no loop: a second rejection is returned without another replay.
    Ok(transport.forward_ordered(url, retry, wire, cancelled, listener).await?)
}

/// Apply once to the final wire body, also for each independently selected retry.
pub(super) fn apply_cache(
    outgoing: &mut reqwest::Request,
    route: &AuthorizedRoute,
    target: &JsonTarget,
    headers: &HeaderMap,
) -> Result<(), StatusCode> {
    if target.request_options.cache_injection != Some(true) { return Ok(()); }
    let bytes = outgoing.body().and_then(reqwest::Body::as_bytes).ok_or(StatusCode::BAD_REQUEST)?;
    let mut body: Value = serde_json::from_slice(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let path = target.url.path().trim_end_matches('/');
    if path.ends_with("/messages") {
        vw_agent_apps::routing::cache_policy::inject_messages(&mut body, true);
    } else if path.ends_with("/chat/completions") || path.ends_with("/responses") {
        let known = target.url.host_str() == Some("api.openai.com")
            || (target.url.host_str() == Some("api.kimi.com") && path.starts_with("/coding/"));
        if known {
            let super::reference_scope::ReferenceScope::Application(scope) = route.response_scope(target)? else {
                return Err(StatusCode::FORBIDDEN);
            };
            let scope = serde_json::to_vec(&serde_json::json!([
                scope.project, scope.host, scope.app, scope.account,
                scope.provider, scope.credential_generation
            ])).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let session = headers.get("session_id").and_then(|v| v.to_str().ok());
            vw_agent_apps::routing::cache_policy::inject_key(&mut body, true, &scope, session);
        }
    }
    *outgoing.body_mut() = Some(body_limits::encode_json(&body, MAX_JSON_BYTES).map_err(request_body_status)?.into());
    outgoing.headers_mut().remove(header::CONTENT_LENGTH);
    Ok(())
}
