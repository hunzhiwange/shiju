//! Provider-only HTTP transport; no agent loop, tool dispatcher, history or usage writes.
use super::{RelayBackend, RelayIdentity, RelayRequest, chat, upstream};
use crate::{app_routing::reference_scope::References, auth, provider::provider};
use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use tokio_util::sync::CancellationToken;

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

pub(super) fn configure_messages_chat(body: &mut serde_json::Value, model: &provider::Model) {
    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};

    // Claude's Chat launch disables thinking. Apply the same mode upstream so
    // provider defaults cannot introduce signed thinking state into this bridge.
    if official_deepseek_messages_url(&model.api).is_some() {
        body["thinking"] = serde_json::json!({"type": "disabled"});
        return;
    }
    let api = &model.api;
    let profile = reasoning_profile(&api.id, &api.adapter, &model.provider_id, &api.url);
    let levels = reasoning_levels(
        &api.id,
        &api.adapter,
        &model.provider_id,
        &api.url,
        model.capabilities.reasoning_options.as_deref(),
    );
    let supports_disabled = match &model.capabilities.reasoning_options {
        Some(options) => options.iter().any(|option| match option {
            vw_shared::provider::reasoning::ReasoningOption::Toggle => true,
            vw_shared::provider::reasoning::ReasoningOption::Effort { values } => {
                values.iter().any(|value| value == "none")
            }
            _ => false,
        }),
        None => profile.levels.contains(&"none"),
    };
    if supports_disabled {
        match profile.format {
            ReasoningFormat::DashScope => body["enable_thinking"] = false.into(),
            ReasoningFormat::DeepSeek => {
                body["thinking"] = serde_json::json!({"type": "disabled"});
            }
            ReasoningFormat::OpenAi
                if profile.levels.contains(&"none")
                    || levels.iter().any(|level| level == "none") =>
            {
                body["reasoning_effort"] = "none".into();
            }
            _ => {}
        }
    }
}

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
        let gemini_request = request.path.starts_with("/v1beta/");
        let gemini_streaming = request.path == "/v1beta/streamGenerateContent";
        let native_gemini = gemini_request && model.api.adapter == "google";
        if matches!(request.path, "/v1beta/models" | "/v1beta/model") {
            bearer(&info)?;
            if !matches!(model.api.adapter.as_str(), "google" | "openai" | "openai-compatible") {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
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
        let native_responses = responses_request && super::responses::is_native(model);
        let codex_chat =
            responses_request && !native_responses && identity.session.ends_with(":codex");
        let mut response_tools = super::responses::ChatTools::default();
        let deepseek_messages = (messages_request
            && model.capabilities.reasoning
            && identity.session.ends_with(":claude"))
        .then(|| official_deepseek_messages_url(&model.api))
        .flatten();
        let alibaba_messages = (messages_request && identity.session.ends_with(":claude"))
            .then(|| official_alibaba_messages_url(&model.api))
            .flatten();
        let native = messages_request
            && (model.api.adapter == "anthropic"
                || deepseek_messages.is_some()
                || alibaba_messages.is_some());
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
            let effort =
                if codex_chat { super::responses::codex_chat_effort(&mut input)? } else { None };
            let (mut converted, tools) = super::responses::to_chat(&input, codex_chat)?;
            response_tools = tools;
            if codex_chat {
                super::responses::configure_codex_chat(&mut converted, model, effort.as_deref());
            }
            chat::prepare(
                model,
                &identity.model,
                &serde_json::to_vec(&converted).map_err(|_| StatusCode::BAD_REQUEST)?,
            )?
        } else if native {
            let mut body = super::messages::native(model, &identity.model, &request.body)?;
            if deepseek_messages.is_some() {
                super::messages::prepare_deepseek_native(&mut body)?;
            } else if alibaba_messages.is_some() {
                super::messages::prepare_hosted_native(&mut body)?;
            }
            body
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
        let streaming = if gemini_request { gemini_streaming } else { body["stream"] == true };
        let url = if native_gemini {
            gemini_url(&model.api, streaming)?
        } else if native_responses {
            responses_url(&model.api)?
        } else if let Some(url) = deepseek_messages.or(alibaba_messages) {
            url
        } else if native {
            messages_url(&model.api)?
        } else {
            chat_url(&model.api)?
        };
        if messages_request && !native {
            configure_messages_chat(&mut body, model);
        }
        let key = bearer(&info)?;
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| StatusCode::UNAUTHORIZED)?,
        );
        if native_gemini {
            headers.remove(header::AUTHORIZATION);
            headers.insert(
                "x-goog-api-key",
                header::HeaderValue::from_str(&key).map_err(|_| StatusCode::UNAUTHORIZED)?,
            );
        }
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
        let outgoing = self
            .client
            .post(url.clone())
            .headers(headers)
            .json(&body)
            .build()
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let response = upstream::execute(&self.client, &url, outgoing, &cancel).await?;
        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
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
        result.body(upstream::response_body(response, cancel)).map_err(|_| StatusCode::BAD_GATEWAY)
    }
}
