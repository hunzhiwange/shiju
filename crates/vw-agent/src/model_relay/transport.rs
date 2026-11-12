//! Provider-only HTTP transport; no agent loop, tool dispatcher, history or usage writes.
use super::{
    RelayBackend, RelayIdentity, RelayRequest, chat, generation::GenerationProtocol as P, upstream,
};
use crate::{app_routing::reference_scope::References, auth, provider::provider};
use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use tokio_util::sync::CancellationToken;
use vw_provider_resolver::protocol_routes::{ConversionSupport, RouteCapability as C};

/// Resolves current configuration and credentials for every request. Contains no session secrets.
pub struct ProviderRelayBackend {
    client: reqwest::Client,
    references: std::sync::Arc<References>,
}
impl ProviderRelayBackend {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self { client: upstream::upstream_http_client()?, references: Default::default() })
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

pub(crate) fn chat_url(api: &provider::ApiInfo) -> Result<reqwest::Url, StatusCode> {
    if !matches!(api.adapter.as_str(), "openai" | "openai-compatible") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let mut url =
        reqwest::Url::parse(api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
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

#[cfg(test)]
pub(super) use super::messages_thinking_config::configure_disabled as configure_messages_chat;

/// The official DeepSeek Chat and Messages endpoints share model IDs and credentials.
pub(crate) fn official_deepseek_messages_url(api: &provider::ApiInfo) -> Option<reqwest::Url> {
    if !matches!(api.adapter.as_str(), "openai" | "openai-compatible")
        || !matches!(api.id.as_str(), "deepseek-flash" | "deepseek-v4-pro")
    {
        return None;
    }
    let mut url = reqwest::Url::parse(api.url.trim()).ok()?;
    if url.scheme() != "https"
        || url.host_str() != Some("api.deepseek.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(
            url.path().trim_end_matches('/'),
            "" | "/v1" | "/chat/completions" | "/v1/chat/completions"
        )
    {
        return None;
    }
    url.set_path("/anthropic/v1/messages");
    Some(url)
}

/// Token Plan exposes both protocols on the same official origin and credentials.
pub(crate) fn official_alibaba_messages_url(api: &provider::ApiInfo) -> Option<reqwest::Url> {
    if !matches!(api.adapter.as_str(), "openai" | "openai-compatible") {
        return None;
    }
    let mut url = reqwest::Url::parse(api.url.trim()).ok()?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some(
                "token-plan.cn-beijing.maas.aliyuncs.com"
                    | "token-plan.ap-southeast-1.maas.aliyuncs.com"
            )
        )
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(
            url.path().trim_end_matches('/'),
            "/compatible-mode/v1" | "/compatible-mode/v1/chat/completions"
        )
    {
        return None;
    }
    url.set_path("/apps/anthropic/v1/messages");
    Some(url)
}

pub(super) fn messages_url(api: &provider::ApiInfo) -> Result<reqwest::Url, StatusCode> {
    let mut url =
        reqwest::Url::parse(api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
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

pub(super) fn responses_url(api: &provider::ApiInfo) -> Result<reqwest::Url, StatusCode> {
    let mut url =
        reqwest::Url::parse(api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
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

pub(super) fn gemini_url(
    api: &provider::ApiInfo,
    streaming: bool,
) -> Result<reqwest::Url, StatusCode> {
    let mut url =
        reqwest::Url::parse(api.url.trim()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let path = url.path().trim_end_matches('/');
    let base = if let Some((base, _)) = path.split_once("/models/") { base } else { path };
    if !(base.is_empty() || base.ends_with("/v1beta") || base.ends_with("/v1")) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let base = if base.is_empty() { "/v1beta" } else { base };
    let id = api.id.strip_prefix("models/").unwrap_or(&api.id);
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    url.set_path(&format!(
        "{base}/models/{id}:{}",
        if streaming { "streamGenerateContent" } else { "generateContent" }
    ));
    if streaming {
        url.set_query(Some("alt=sse"));
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
        if !matches!(
            request.path,
            "/v1/chat/completions"
                | "/v1/messages"
                | "/v1/responses"
                | "/v1beta/generateContent"
                | "/v1beta/streamGenerateContent"
                | "/v1beta/models"
                | "/v1beta/model"
        ) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let info = provider::get_provider(&identity.provider)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?
            .ok_or(StatusCode::FORBIDDEN)?;
        // Exact catalog key required: API-id alias lookup must not authorize another model.
        let model = info.models.get(&identity.model).ok_or(StatusCode::FORBIDDEN)?;
        if info.id != identity.provider || model.provider_id != identity.provider {
            return Err(StatusCode::FORBIDDEN);
        }
        let gemini_request = request.path.starts_with("/v1beta/");
        let gemini_streaming = request.path == "/v1beta/streamGenerateContent";

        if matches!(request.path, "/v1beta/models" | "/v1beta/model") {
            let capabilities = [C::Text, C::Streaming];
            let conversions = [ConversionSupport {
                source: P::Gemini,
                target: P::Chat,
                capabilities: &capabilities,
            }];
            let route = super::routing::select(
                model,
                P::Gemini,
                &capabilities,
                &conversions,
                super::routing::Execution::Relay,
            )?;
            if !matches!(
                route.endpoint.authentication,
                vw_shared::provider::protocols::ProtocolAuthentication::None {}
            ) {
                bearer(&info)?;
            }
            let metadata = serde_json::json!({"name":format!("models/{}",identity.model),"baseModelId":identity.model,"version":"relay","displayName":identity.model,"inputTokenLimit":model.limit.input.unwrap_or(model.limit.context),"outputTokenLimit":model.limit.output,"supportedGenerationMethods":["generateContent","streamGenerateContent"]});
            let output = if request.path == "/v1beta/models" {
                serde_json::json!({"models":[metadata]})
            } else {
                metadata
            };
            return Response::builder()
                .header("content-type", "application/json")
                .body(Body::from(output.to_string()))
                .map_err(|_| StatusCode::BAD_GATEWAY);
        }
        let messages_request = request.path == "/v1/messages";
        let responses_request = request.path == "/v1/responses";
        let source = if gemini_request {
            P::Gemini
        } else if messages_request {
            P::Messages
        } else if responses_request {
            P::Responses
        } else {
            P::Chat
        };
        let input: serde_json::Value =
            serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
        let streaming_requested =
            if gemini_request { gemini_streaming } else { input["stream"] == true };
        let required = super::routing::required(&input, streaming_requested);
        let conversions = super::routing::relay_conversions(identity.session.ends_with(":codex"));
        let route = super::routing::select(
            model,
            source,
            &required,
            &conversions,
            super::routing::Execution::Relay,
        )?;
        let native_gemini = source == P::Gemini && !route.conversion_required;
        let native_responses = source == P::Responses && !route.conversion_required;
        let native = source == P::Messages && !route.conversion_required;
        let key = if matches!(
            route.endpoint.authentication,
            vw_shared::provider::protocols::ProtocolAuthentication::None {}
        ) {
            String::new()
        } else {
            bearer(&info)?
        };
        let thinking = (messages_request && !native && model.capabilities.reasoning).then(|| {
            // Bind receipts to the actual deployment and current credential without retaining it.
            let credential =
                hex::encode(ring::digest::digest(&ring::digest::SHA256, key.as_bytes()));
            let deployment = serde_json::json!([
                model.api.id,
                model.api.url,
                route.endpoint.url,
                route.endpoint.deployment,
                credential,
                model.headers.iter().collect::<std::collections::BTreeMap<_, _>>(),
                route.endpoint.authentication
            ])
            .to_string();
            super::messages_thinking::Context::new(identity, deployment, self.references.clone())
        });
        let codex_chat =
            responses_request && !native_responses && identity.session.ends_with(":codex");
        let mut response_tools = super::responses::ChatTools::default();
        let mut body = if gemini_request {
            let input =
                serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
            if native_gemini {
                super::gemini::native(&input, model)?
            } else {
                let converted = super::gemini::to_chat(&input)?;
                chat::prepare(
                    model,
                    &identity.model,
                    &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
                )?
            }
        } else if native_responses {
            super::responses::native(model, identity, &request.body, &self.references)?
        } else if responses_request {
            let mut input =
                serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let effort = if codex_chat {
                super::responses::codex_chat_effort(&mut input)?
            } else {
                super::responses::chat_effort(&mut input, model)?
            };
            let (mut converted, tools) = super::responses::to_chat(&input, codex_chat)?;
            response_tools = tools;
            if codex_chat || effort.is_some() {
                super::responses::configure_codex_chat(&mut converted, model, effort.as_deref());
            }
            chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else if native {
            let mut body = super::messages::native(model, &identity.model, &request.body)?;
            if official_deepseek_messages_url(&model.api).is_some() {
                super::messages::prepare_deepseek_native(&mut body)?;
            } else if official_alibaba_messages_url(&model.api).is_some() {
                super::messages::prepare_hosted_native(&mut body)?;
            }
            body
        } else if messages_request {
            let input =
                serde_json::from_slice(&request.body).map_err(|_| StatusCode::BAD_REQUEST)?;
            let converted = match &thinking {
                Some(context) => context.request(&input)?,
                None => super::messages::to_chat(&input)?,
            };
            chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else {
            chat::prepare(model, &identity.model, &request.body)?
        };
        let streaming = if gemini_request { gemini_streaming } else { body["stream"] == true };
        let mut url = reqwest::Url::parse(&route.endpoint.url)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if native_gemini && streaming {
            let path = url
                .path()
                .strip_suffix(":generateContent")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let path = format!("{path}:streamGenerateContent");
            url.set_path(&path);
            url.query_pairs_mut().append_pair("alt", "sse");
        }
        if messages_request && !native {
            super::messages_thinking::configure(&mut body, &input, model)?;
        }
        // This conversion carries text and tools, not replayable thinking. A missing
        // Gemini thinkingConfig must not enable a Chat deployment's default reasoning.
        // to_chat already rejects explicit thinking requests that cannot be preserved.
        if gemini_request && !native_gemini {
            super::messages_thinking::configure(
                &mut body,
                &serde_json::json!({"thinking":{"type":"disabled"}}),
                model,
            )?;
        }
        let mut headers = super::generation::endpoint_headers(&route.endpoint, &key)?;
        for (name, value) in &model.headers {
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
        let outgoing = self
            .client
            .post(url.clone())
            .headers(headers)
            .json(&body)
            .build()
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let response = upstream::execute(&self.client, &url, outgoing, &cancel).await?;
        if !response.status().is_success() {
            // Keep authentication, access and request failures distinguishable from
            // transient provider failures without exposing upstream bodies or headers.
            return Err(if response.status().is_client_error() {
                response.status()
            } else {
                StatusCode::BAD_GATEWAY
            });
        }
        if gemini_request {
            return super::gemini_stream::response(
                response,
                identity.model.clone(),
                streaming,
                native_gemini,
                cancel,
            )
            .await;
        }
        if responses_request {
            return super::responses_stream::response(
                response,
                super::responses_stream::Output {
                    identity: identity.clone(),
                    native: native_responses,
                    codex_chat,
                    tool_names: response_tools,
                    streaming,
                    store: native_responses && body["store"] != false,
                    references: self.references.clone(),
                },
                cancel,
            )
            .await;
        }
        if messages_request {
            if let Some(context) = thinking {
                return super::messages_thinking::response(
                    response,
                    context,
                    &identity.model,
                    streaming,
                    cancel,
                )
                .await;
            }
            return super::messages_stream::response(
                response,
                identity.model.clone(),
                streaming,
                native,
                cancel,
            )
            .await;
        }
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
            super::stream_observer::validated_body(
                body,
                super::generation::GenerationProtocol::Chat,
                cancel,
            )
        } else {
            body
        };
        result.body(body).map_err(|_| StatusCode::BAD_GATEWAY)
    }
}
