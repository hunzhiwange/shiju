//! Native HTTP discovery for an authorized application provider.

use std::{collections::BTreeMap, future::Future};

use reqwest::{
    Url,
    header::{AUTHORIZATION, HeaderMap, HeaderName, USER_AGENT},
};
use vw_api_types::app_providers::{ProviderProtocol, endpoints::AppProviderEndpointKind};
pub use vw_provider_resolver::remote_models::DiscoveredModel;
use vw_provider_resolver::remote_models::{discover_catalog_models, is_real_bearer};

use super::{discovery_url::build_models_url_candidates, user_agent_presets::custom_user_agent};
use crate::providers::validation::{header_value, protected_header};

/// Resolve these inputs from the authorized host and provider before calling discovery.
/// A same-origin override is provider configuration. A cross-origin override must also
/// match an exact endpoint in `authorized_overrides` for that provider.
pub struct HttpDiscoveryRequest<'a> {
    pub base_url: &'a str,
    pub endpoint_kind: AppProviderEndpointKind,
    pub protocol: ProviderProtocol,
    pub models_url: Option<&'a str>,
    pub authorized_overrides: &'a [Url],
    pub api_key: &'a str,
    pub user_agent: Option<&'a str>,
    pub request_headers: &'a BTreeMap<String, String>,
}

/// Pass the upper-layer token's cancellation future. Cancelling drops the HTTP
/// future, including any in-flight body read, without starting a detached worker.
/// Errors are stable categories suitable for mapping to public discovery failures.
pub async fn discover(
    request: HttpDiscoveryRequest<'_>,
    cancelled: impl Future<Output = ()>,
) -> Result<Vec<DiscoveredModel>, &'static str> {
    tokio::select! {
        biased;
        () = cancelled => Err("cancelled"),
        result = discover_authorized(request) => result,
    }
}

async fn discover_authorized(
    request: HttpDiscoveryRequest<'_>,
) -> Result<Vec<DiscoveredModel>, &'static str> {
    let candidates = build_models_url_candidates(
        request.base_url,
        request.endpoint_kind,
        request.protocol,
        request.models_url,
        request.authorized_overrides,
    )
    .map_err(|_| "invalid_endpoint")?;
    let mut headers = custom_headers(request.request_headers)?;

    if !is_real_bearer(request.api_key) || request.api_key.chars().any(char::is_control) {
        return Err("invalid_credential");
    }
    let key = request.api_key.trim();
    let (name, value) = match request.protocol {
        ProviderProtocol::OpenaiResponses => {
            (AUTHORIZATION, header_value(&format!("Bearer {key}")))
        }
        ProviderProtocol::Anthropic => (HeaderName::from_static("x-api-key"), header_value(key)),
        ProviderProtocol::Gemini => (HeaderName::from_static("x-goog-api-key"), header_value(key)),
    };
    let mut value = value.map_err(|_| "invalid_credential")?;
    value.set_sensitive(true);
    headers.insert(name, value);
    if let Some(mut user_agent) =
        custom_user_agent(request.user_agent).map_err(|_| "invalid_headers")?
    {
        user_agent.set_sensitive(true);
        headers.insert(USER_AGENT, user_agent);
    }

    discover_catalog_models(&candidates, &headers).await.map_err(|error| error.category())
}

fn custom_headers(input: &BTreeMap<String, String>) -> Result<HeaderMap, &'static str> {
    const EXTRA_PROTECTED: &[&str] = &[
        "cookie",
        "cookie2",
        "expect",
        "keep-alive",
        "origin",
        "range",
        "referer",
        "user-agent",
        "via",
        "x-http-method",
        "x-http-method-override",
        "x-method-override",
        "x-original-host",
        "x-original-url",
        "x-rewrite-url",
    ];
    if input.len() > 64 {
        return Err("invalid_headers");
    }
    let mut headers = HeaderMap::new();
    let mut bytes = 0;
    for (name, value) in input {
        let name = name.trim();
        if name.len() > 256 {
            return Err("invalid_headers");
        }
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| "invalid_headers")?;
        let mut value = header_value(value).map_err(|_| "invalid_headers")?;
        bytes += name.as_str().len() + value.len();
        let key = name.as_str();
        if bytes > 128 * 1024
            || protected_header(key)
            || EXTRA_PROTECTED.contains(&key)
            || key.starts_with("proxy-")
            || key.starts_with("x-forwarded-")
            || headers.contains_key(&name)
        {
            return Err("invalid_headers");
        }
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    Ok(headers)
}
