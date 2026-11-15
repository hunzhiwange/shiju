//! Validated protocol endpoints for an explicitly selected provider.
use axum::http::StatusCode;

pub fn chat_url(api: &model_types::ApiInfo) -> Result<reqwest::Url, StatusCode> {
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

pub fn official_deepseek_messages_url(api: &model_types::ApiInfo) -> Option<reqwest::Url> {
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

pub fn official_alibaba_messages_url(api: &model_types::ApiInfo) -> Option<reqwest::Url> {
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

pub fn messages_url(api: &model_types::ApiInfo) -> Result<reqwest::Url, StatusCode> {
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

pub fn responses_url(api: &model_types::ApiInfo) -> Result<reqwest::Url, StatusCode> {
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

pub fn gemini_url(
    api: &model_types::ApiInfo,
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
