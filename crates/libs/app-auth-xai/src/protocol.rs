//! Host-side xAI OAuth protocol boundaries.

use app_auth_core::response::{read_json_limited, send};
use reqwest::{Client, Request, Url};
use serde::Deserialize;
use std::time::Duration;
use vw_api_app_auth::AuthErrorCode;

const ISSUER: &str = "https://auth.x.ai";
const DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
pub const CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub const SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
pub struct Discovery {
    issuer: String,
    token_endpoint: String,
    pub device_authorization_endpoint: String,
}

impl Discovery {
    pub fn token_endpoint(&self) -> Result<Url, AuthErrorCode> {
        trusted_url(&self.token_endpoint, &["auth.x.ai"])
    }
}

pub fn oauth_client() -> Result<Client, AuthErrorCode> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .user_agent("cc-switch-xai-oauth")
        .build()
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)
}

pub async fn discover(
    client: &Client,
    route: &mut impl FnMut(&mut Request),
) -> Result<Discovery, AuthErrorCode> {
    let response = send(client, client.get(DISCOVERY_URL), route).await?;
    if !response.status().is_success() {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    let discovery: Discovery = read_json_limited(response, MAX_RESPONSE_BYTES).await?;
    if discovery.issuer.trim_end_matches('/') != ISSUER {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(discovery)
}

pub fn trusted_url(value: &str, allowed_hosts: &[&str]) -> Result<Url, AuthErrorCode> {
    let url = Url::parse(value).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if url.scheme() != "https"
        || !url.host_str().is_some_and(|host| allowed_hosts.contains(&host))
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(url)
}

pub fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic())
}
