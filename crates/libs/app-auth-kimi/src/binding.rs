//! Pure Kimi native OAuth declaration validation.

use crate::protocol::endpoints;
use serde_json::Value;
use vw_api_app_auth::{AuthErrorCode, AuthRegion};

pub fn official(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("kimi")
        && node.pointer("/oauth/key").and_then(Value::as_str).is_some_and(|k| !k.trim().is_empty())
}
pub fn binding_region(provider: &Value) -> Option<AuthRegion> {
    [AuthRegion::MainlandCn, AuthRegion::Global]
        .into_iter()
        .find(|&region| oauth_slot(provider, region).is_ok())
}

pub fn oauth_slot(provider: &Value, region: AuthRegion) -> Result<&str, AuthErrorCode> {
    if !official(provider) {
        return Err(AuthErrorCode::Unsupported);
    }
    let endpoint = endpoints(region);
    let oauth = &provider["oauth"];
    let oauth_host = oauth.get("oauth_host").and_then(Value::as_str);
    if provider["base_url"].as_str().map(|url| url.trim_end_matches('/')) != Some(endpoint.api)
        || match oauth.get("oauth_host") {
            None => region != AuthRegion::MainlandCn,
            Some(_) => oauth_host != Some(endpoint.oauth),
        }
    {
        return Err(AuthErrorCode::IdentityMismatch);
    }
    if oauth.get("storage").is_some_and(|storage| storage.as_str() != Some("file"))
        || provider.get("api_key_env").is_some()
        || provider.get("env").is_some()
        || provider.get("custom_headers").is_some()
        || provider.get("api_key").is_some_and(|key| key.as_str() != Some(""))
    {
        return Err(AuthErrorCode::Unsupported);
    }
    let key = oauth["key"].as_str().ok_or(AuthErrorCode::InvalidInput)?;
    let slot = key.strip_prefix("oauth/").unwrap_or(key);
    if slot.is_empty()
        || slot.len() > 128
        || !slot.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err(AuthErrorCode::InvalidInput);
    }
    Ok(slot)
}
