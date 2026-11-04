//! Provider-only HTTP transport; no agent loop, tool dispatcher, history or usage writes.
use super::{RelayBackend, RelayIdentity, RelayRequest, chat, upstream_http_client};
use crate::{auth, provider::provider};
use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

/// Resolves current configuration and credentials for every request. Contains no session secrets.
pub struct ProviderRelayBackend {
    client: reqwest::Client,
    references: std::sync::Arc<super::responses::References>,
}
impl ProviderRelayBackend {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: upstream_http_client()?, references: Default::default() })
    }
}

fn bearer(info: &provider::Info) -> Result<String, StatusCode> {
    credential(info, auth::get(&info.id))
}

pub(super) fn credential(
    info: &provider::Info,
    current_auth: Option<auth::Info>,
) -> Result<String, StatusCode> {
    let real_key = |key: &str| !key.trim().is_empty() && key.trim() != auth::OAUTH_DUMMY_KEY;
    if let Some(key) = info.key.as_deref().filter(|v| {
        real_key(v)
            && matches!(
                info.source,
                provider::ProviderSource::Config | provider::ProviderSource::Custom
            )
    }) {
        return Ok(key.trim().into());
    }
    for name in &info.env {
        if let Some(key) = crate::env::get(name).filter(|v| real_key(v)) {
            return Ok(key.trim().into());
        }
    }
    match current_auth {
        Some(auth::Info::Api(api)) if real_key(&api.key) => Ok(api.key.trim().into()),
        Some(auth::Info::Oauth(oauth))
            if real_key(&oauth.access) && oauth.expires > chrono::Utc::now().timestamp_millis() =>
        {
            Ok(oauth.access)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn chat_url(model: &provider::Model) -> Result<reqwest::Url, StatusCode> {
    if !matches!(model.api.adapter.as_str(), "openai" | "openai-compatible") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut url =
        reqwest::Url::parse(model.api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let path = url.path().trim_end_matches('/');
    if path.ends_with("/messages")
        || path.ends_with("/responses")
        || path.ends_with(":generateContent")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if !path.ends_with("/chat/completions") {
        let path = if path.is_empty() { "/v1" } else { path };
        url.set_path(&format!("{path}/chat/completions"));
    }
    Ok(url)
}

fn messages_url(model: &provider::Model) -> Result<reqwest::Url, StatusCode> {
    let mut url =
        reqwest::Url::parse(model.api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let path = url.path().trim_end_matches('/');
    if path.ends_with("/chat/completions")
        || path.ends_with("/responses")
        || path.contains(":generateContent")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if !path.ends_with("/messages") {
        let path = if path.is_empty() { "/v1" } else { path };
        url.set_path(&format!("{path}/messages"));
    }
    Ok(url)
}

fn responses_url(model: &provider::Model) -> Result<reqwest::Url, StatusCode> {
    let mut url =
        reqwest::Url::parse(model.api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let path = url.path().trim_end_matches('/');
    if path.ends_with("/messages")
        || path.ends_with("/chat/completions")
        || path.contains(":generateContent")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if !path.ends_with("/responses") {
        let path = if path.is_empty() { "/v1" } else { path };
        url.set_path(&format!("{path}/responses"));
    }
    Ok(url)
}

#[async_trait::async_trait]
impl RelayBackend for ProviderRelayBackend {
    async fn execute(
        &self,
        identity: &RelayIdentity,
        request: RelayRequest,
        cancel: CancellationToken,
    ) -> Result<Response, StatusCode> {
        if crate::session::ui_store::current_tenant_scope() != identity.tenant
            || crate::project::instance::project().is_none_or(|p| p.id != identity.project)
        {
            return Err(StatusCode::FORBIDDEN);
        }
        if !matches!(request.path, "/v1/chat/completions" | "/v1/messages" | "/v1/responses") {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let info = provider::get_provider(&identity.provider)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?
            .ok_or(StatusCode::FORBIDDEN)?;
        // Exact catalog key required: API-id alias lookup must not authorize another model.
        let model = info.models.get(&identity.model).ok_or(StatusCode::FORBIDDEN)?;
        let messages_request = request.path == "/v1/messages";
        let responses_request = request.path == "/v1/responses";
        let native_responses = responses_request && super::responses::is_native(model);
        let native = messages_request && model.api.adapter == "anthropic";
        let body = if native_responses {
            super::responses::native(model, identity, &request.body, &self.references)?
        } else if responses_request {
            let input =
                serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let converted = super::responses::to_chat(&input)?;
            chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else if native {
            super::messages::native(model, &identity.model, &request.body)?
        } else if messages_request {
            let input =
                serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let converted = super::messages::to_chat(&input)?;
            chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else {
            chat::prepare(model, &identity.model, &request.body)?
        };
        let streaming = body["stream"] == true;
        let url = if native_responses {
            responses_url(model)?
        } else if native {
            messages_url(model)?
        } else {
            chat_url(model)?
        };
        let key = bearer(&info)?;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| StatusCode::UNAUTHORIZED)?,
        );
        if native {
            headers.remove(header::AUTHORIZATION);
            headers.insert(
                "x-api-key",
                header::HeaderValue::from_str(&key).map_err(|_| StatusCode::UNAUTHORIZED)?,
            );
            headers.insert("anthropic-version", header::HeaderValue::from_static("2023-06-01"));
        }
        for (name, value) in &model.headers {
            let name = header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            if matches!(
                name.as_str(),
                "host" | "connection" | "content-length" | "transfer-encoding" | "content-type"
            ) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let value = header::HeaderValue::from_str(value)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            headers.insert(name, value);
        }
        let outgoing = self.client.post(url).headers(headers).json(&body);
        // No automatic retries: a partially emitted completion is never billed or executed twice.
        let response = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(StatusCode::REQUEST_TIMEOUT),
            response = outgoing.send() => response.map_err(|_| StatusCode::BAD_GATEWAY)?,
        };
        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if responses_request {
            return super::responses_stream::response(
                response,
                super::responses_stream::Output {
                    identity: identity.clone(),
                    native: native_responses,
                    streaming,
                    store: native_responses && body["store"] != false,
                    references: self.references.clone(),
                },
                cancel,
            )
            .await;
        }
        if messages_request {
            return super::messages_stream::response(
                response,
                identity.model.clone(),
                streaming,
                native,
                cancel,
            )
            .await;
        }
        let mut result = Response::builder().status(response.status());
        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            result = result.header(header::CONTENT_TYPE, content_type);
        }
        // Preserve SSE deltas, parallel call indexes, finish reasons and usage byte-for-byte.
        let stream = futures_util::stream::unfold(
            (response.bytes_stream(), cancel),
            |(mut stream, cancel)| async move {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => None,
                    next = stream.next() => next.map(|value| (value.map_err(|_| std::io::Error::other("upstream stream failed")), (stream, cancel))),
                }
            },
        );
        result.body(Body::from_stream(stream)).map_err(|_| StatusCode::BAD_GATEWAY)
    }
}
