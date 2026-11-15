//! Copilot authority and account-bound endpoint rules.

#![cfg(not(target_arch = "wasm32"))]

use reqwest::Url;
use serde::Deserialize;
use vw_api_app_auth::AuthErrorCode;

pub const PUBLIC_AUTHORITY: &str = "github.com";

pub fn canonical_authority(input: Option<&str>) -> Result<String, AuthErrorCode> {
    let input = input.unwrap_or(PUBLIC_AUTHORITY).trim();
    let (host, port) = input.split_once(':').unwrap_or((input, "443"));
    if !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AuthErrorCode::InvalidInput);
    }
    let port =
        port.parse::<u16>().ok().filter(|port| *port != 0).ok_or(AuthErrorCode::InvalidInput)?;
    if !host.contains('.')
        || host.len() > 253
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(AuthErrorCode::InvalidInput);
    }
    let host = host.to_ascii_lowercase();
    let url =
        Url::parse(&format!("https://{host}:{port}/")).map_err(|_| AuthErrorCode::InvalidInput)?;
    if url.domain() != Some(host.as_str()) || (host == PUBLIC_AUTHORITY && port != 443) {
        return Err(AuthErrorCode::InvalidInput);
    }
    if port == 443 { Ok(host) } else { Ok(format!("{host}:{port}")) }
}

#[derive(Deserialize)]
pub struct Endpoints {
    api: String,
}

pub fn session_endpoint(authority: &str, discovered: Option<Endpoints>) -> Result<Url, AuthErrorCode> {
    let default = if authority == PUBLIC_AUTHORITY {
        "https://api.githubcopilot.com".to_owned()
    } else {
        format!("https://copilot-api.{authority}")
    };
    let endpoint = match discovered.as_ref() {
        Some(Endpoints { api }) => api.strip_suffix('/').unwrap_or(api),
        None => default.as_str(),
    };
    let allowed = endpoint == default
        || (authority == PUBLIC_AUTHORITY
            && matches!(
                endpoint,
                "https://api.individual.githubcopilot.com"
                    | "https://api.business.githubcopilot.com"
                    | "https://api.enterprise.githubcopilot.com"
            ));
    if !allowed {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Url::parse(endpoint).map_err(|_| AuthErrorCode::UpstreamFailure)
}
