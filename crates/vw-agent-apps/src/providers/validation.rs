//! Common write validation. Errors contain no request data, paths or upstream parser details.
use reqwest::header::{HeaderName, HeaderValue, USER_AGENT};
use std::collections::BTreeMap;
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};
const PROTECTED: &[&str] = &[
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "proxy-authorization",
    "proxy-authenticate",
    "te",
    "trailer",
    "upgrade",
    "accept-encoding",
    "content-type",
    "authorization",
    "x-api-key",
    "x-goog-api-key",
    "chatgpt-account-id",
    "session_id",
    "x-client-request-id",
    "x-codex-window-id",
    "x-forwarded-host",
    "x-forwarded-port",
    "x-forwarded-proto",
    "forwarded",
    "cf-connecting-ip",
    "cf-ipcountry",
    "cf-ray",
    "cf-visitor",
    "true-client-ip",
    "fastly-client-ip",
    "x-azure-clientip",
    "x-azure-fdid",
    "x-azure-ref",
    "akamai-origin-hop",
    "x-akamai-config-log-detail",
    "x-request-id",
    "x-correlation-id",
    "x-trace-id",
    "x-amzn-trace-id",
    "x-b3-traceid",
    "x-b3-spanid",
    "x-b3-parentspanid",
    "x-b3-sampled",
    "traceparent",
    "tracestate",
];
pub(crate) fn protected_header(name: &str) -> bool {
    PROTECTED.contains(&name)
}

pub fn endpoint(raw: &str) -> Result<(), String> {
    if raw.is_empty()
        || raw.len() > 2048
        || raw.chars().any(char::is_whitespace)
        || raw.chars().any(char::is_control)
        || raw.contains('\\')
        || !raw.contains("://")
    {
        return Err("invalid application provider endpoint".into());
    }
    let url = reqwest::Url::parse(raw).map_err(|_| "invalid application provider endpoint")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid application provider endpoint".into());
    }
    Ok(())
}
pub(crate) fn header_value(value: &str) -> Result<HeaderValue, &'static str> {
    if value.len() > 8192 {
        return Err("invalid provider request header value");
    }
    HeaderValue::from_bytes(value.as_bytes()).map_err(|_| "invalid provider request header value")
}

/// Resolve only an explicitly supplied value. An absent or blank value leaves the request alone.
pub fn custom_user_agent(value: Option<&str>) -> Result<Option<HeaderValue>, &'static str> {
    // Keep illegal control characters at the edges so the header parser can reject them.
    let value = value.map(|value| {
        value.trim_matches(|c: char| c.is_whitespace() && (!c.is_ascii_control() || c == '\t'))
    });
    value
        .filter(|value| !value.is_empty())
        .map(|value| header_value(value).map_err(|_| "invalid custom User-Agent"))
        .transpose()
}

fn headers(input: &BTreeMap<String, String>, proxy: bool) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    if input.len() > 64 {
        return Err("too many provider request headers".into());
    }
    for (raw, value) in input {
        let name = raw.trim();
        if raw.starts_with("draft-header:")
            || name.len() > 256
            || HeaderName::from_bytes(name.as_bytes()).is_err()
            || header_value(value).is_err()
            || !seen.insert(name.to_ascii_lowercase())
            || (proxy && protected_header(&name.to_ascii_lowercase()))
        {
            return Err("invalid or protected provider request header".into());
        }
    }
    Ok(())
}
pub fn validate(app: AppId, options: &ProviderRequestOptions) -> Result<(), String> {
    if let Some(values) = &options.request_headers {
        headers(values, false)?;
    }
    if options.api_key_field.is_some() && !matches!(app, AppId::Claude | AppId::Codex) {
        return Err("application does not support this key field selection".into());
    }
    if let Some(overrides) = &options.local_proxy_request_overrides {
        if !matches!(app, AppId::Claude | AppId::Codex) {
            return Err("application does not support proxy request overrides".into());
        }
        if let Some(values) = &overrides.headers {
            headers(values, true)?;
        }
        if overrides.body.as_ref().is_some_and(|body| body.contains_key("stream")) {
            return Err("proxy body cannot override protocol stream field".into());
        }
    }
    if serde_json::to_vec(options).map_err(|_| "invalid provider request options")?.len()
        > 128 * 1024
    {
        return Err("provider request options too large".into());
    }
    Ok(())
}
/// Partial updates preserve each omitted common member; override objects replace atomically.
pub fn merge(
    app: AppId,
    old: Option<&ProviderRequestOptions>,
    incoming: &ProviderRequestOptions,
) -> Result<ProviderRequestOptions, String> {
    validate(app, incoming)?;
    let mut result = old.cloned().unwrap_or_default();
    if let Some(enabled) = incoming.cache_injection {
        result.cache_injection = Some(enabled);
    }
    if let Some(headers) = &incoming.request_headers {
        result.request_headers = Some(headers.clone());
    }
    if let Some(field) = incoming.api_key_field {
        result.api_key_field = Some(field);
    }
    if let Some(overrides) = &incoming.local_proxy_request_overrides {
        result.local_proxy_request_overrides = Some(overrides.clone());
    }
    validate(app, &result)?;
    let normalize = |values: &mut BTreeMap<String, String>| {
        *values = std::mem::take(values)
            .into_iter()
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v))
            .collect();
    };
    if let Some(values) = &mut result.request_headers {
        normalize(values);
    }
    if let Some(overrides) = &mut result.local_proxy_request_overrides {
        // Keep an explicit clear as {}, without persisting empty override members.
        overrides.body = overrides.body.take().filter(|body| !body.is_empty());
        if let Some(values) = &mut overrides.headers {
            normalize(values);
            let name = USER_AGENT.as_str();
            if let Some(value) = values.get(name)
                && custom_user_agent(Some(value))?.is_none()
            {
                values.remove(name);
            }
        }
        overrides.headers = overrides.headers.take().filter(|values| !values.is_empty());
    }
    Ok(result)
}
/// Frozen source meta can be observed by the editor; validate only common members,
/// retaining all application-owned metadata without inventing its schema.
pub fn metadata(app: AppId, meta: Option<&serde_json::Value>) -> Result<(), String> {
    let Some(meta) = meta.and_then(serde_json::Value::as_object) else {
        return Ok(());
    };
    let mut fields = serde_json::Map::new();
    for name in ["requestHeaders", "apiKeyField", "localProxyRequestOverrides"] {
        if let Some(value) = meta.get(name) {
            fields.insert(name.into(), value.clone());
        }
    }
    let options = serde_json::from_value(serde_json::Value::Object(fields))
        .map_err(|_| "invalid provider request metadata")?;
    validate(app, &options)
}

/// Validate common fields from an already selected native request table.
/// Selection, model semantics and all other application schema stay in projection owners.
pub(super) fn native_request(
    url: Option<&toml_edit::Item>,
    values: Option<&toml_edit::Item>,
) -> Result<(), String> {
    if let Some(url) = url {
        endpoint(url.as_str().ok_or("invalid native provider endpoint")?)?;
    }
    if let Some(values) = values {
        let values = values.as_table_like().ok_or("invalid native provider headers")?;
        let values = values
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.to_string(),
                    v.as_str().ok_or("invalid native provider header value")?.to_string(),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        headers(&values, false)?;
    }
    Ok(())
}
