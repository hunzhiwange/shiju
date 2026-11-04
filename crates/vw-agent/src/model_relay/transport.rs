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
}
impl ProviderRelayBackend {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: upstream_http_client()? })
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
        if request.path != "/v1/chat/completions" {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let info = provider::get_provider(&identity.provider)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?
            .ok_or(StatusCode::FORBIDDEN)?;
        // Exact catalog key required: API-id alias lookup must not authorize another model.
        let model = info.models.get(&identity.model).ok_or(StatusCode::FORBIDDEN)?;
        let body = chat::prepare(model, &identity.model, &request.body)?;
        let url = chat_url(model)?;
        let key = bearer(&info)?;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| StatusCode::UNAUTHORIZED)?,
        );
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
