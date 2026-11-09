//! Host-authorized HTTP for usage scripts.

use std::{collections::BTreeMap, fmt, net::IpAddr, time::Duration};

use reqwest::{Client, Method, Request, StatusCode, Url, header::HeaderName, redirect::Policy};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::apps_usage::UsageQueryErrorCode;

use super::{
    config::{PreparedUsageScript, UsageTemplate},
    runtime::{MEMORY_LIMIT, RuntimeError},
};
use crate::providers::validation::header_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    Runtime(RuntimeError),
    InvalidRequest,
    DestinationDenied,
    Redirect,
    Upstream(UsageQueryErrorCode),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => error.fmt(f),
            Self::InvalidRequest => f.write_str("invalid usage script HTTP request"),
            Self::DestinationDenied => f.write_str("usage script destination is not authorized"),
            Self::Redirect => f.write_str("usage script HTTP redirect denied"),
            Self::Upstream(code) => write!(f, "usage script HTTP request failed ({code:?})"),
        }
    }
}

impl std::error::Error for NetworkError {}

#[derive(Deserialize)]
struct RequestConfig {
    url: String,
    method: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    body: Option<String>,
}

impl PreparedUsageScript<'_> {
    /// Execute the script's request against an exact origin granted by the
    /// execution host for this query and its credentials. The caller supplies
    /// grants from trusted policy, independent of the script and its parameters.
    /// The response stays on the host for the script's extraction stage.
    pub async fn send_request(
        &self,
        timeout_secs: u64,
        authorized_origins: &[Url],
    ) -> Result<String, NetworkError> {
        let candidate = self.build_request().await.map_err(NetworkError::Runtime)?;
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(transport_error)?;
        let request =
            build_http_request(&client, self, candidate, timeout_secs, authorized_origins)?;
        let mut response = client.execute(request).await.map_err(transport_error)?;

        use UsageQueryErrorCode::*;
        match response.status() {
            status if status.is_success() => {}
            status if status.is_redirection() => return Err(NetworkError::Redirect),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(NetworkError::Upstream(UpstreamUnauthorized));
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(NetworkError::Upstream(UpstreamRateLimited));
            }
            status if status.is_server_error() => {
                return Err(NetworkError::Upstream(UpstreamUnavailable));
            }
            _ => return Err(NetworkError::Upstream(UpstreamRejected)),
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if chunk.len() >= MEMORY_LIMIT - body.len() {
                return Err(NetworkError::Upstream(InvalidResponse));
            }
            body.extend_from_slice(&chunk);
        }
        String::from_utf8(body).map_err(|_| NetworkError::Upstream(InvalidResponse))
    }
}

fn build_http_request(
    client: &Client,
    script: &PreparedUsageScript<'_>,
    candidate: Value,
    timeout_secs: u64,
    authorized_origins: &[Url],
) -> Result<Request, NetworkError> {
    use NetworkError::{DestinationDenied, InvalidRequest};

    let config: RequestConfig = serde_json::from_value(candidate).map_err(|_| InvalidRequest)?;
    let method: Method = config.method.parse().map_err(|_| InvalidRequest)?;
    let custom = script.template() == UsageTemplate::Custom;
    let url = parse_endpoint(&config.url, custom)?;
    let origin = url.origin();
    if !custom
        && let Some(base) = script.base_url()
        && parse_endpoint(base, false)?.origin() != origin
    {
        return Err(DestinationDenied);
    }
    if !authorized_origins.iter().any(|allowed| allowed.origin() == origin) {
        return Err(DestinationDenied);
    }

    let mut request = client
        .request(method, url)
        .timeout(Duration::from_secs(timeout_secs.clamp(2, 30)))
        .build()
        .map_err(|_| InvalidRequest)?;
    for (name, value) in config.headers {
        let name: HeaderName = name.parse().map_err(|_| InvalidRequest)?;
        // The client must control the destination and frame the request body.
        if matches!(
            name.as_str(),
            "host"
                | "content-length"
                | "transfer-encoding"
                | "connection"
                | "te"
                | "trailer"
                | "upgrade"
                | "forwarded"
                | "x-original-host"
                | "x-original-url"
                | "x-rewrite-url"
        ) || name.as_str().starts_with("proxy-")
            || name.as_str().starts_with("x-forwarded-")
            || request.headers().contains_key(&name)
        {
            return Err(InvalidRequest);
        }
        let mut value = header_value(&value).map_err(|_| InvalidRequest)?;
        value.set_sensitive(true);
        request.headers_mut().insert(name, value);
    }
    *request.body_mut() = config.body.map(Into::into);
    Ok(request)
}

fn parse_endpoint(raw: &str, custom: bool) -> Result<Url, NetworkError> {
    let url = Url::parse(raw).map_err(|_| NetworkError::InvalidRequest)?;
    let host = url.host_str().ok_or(NetworkError::InvalidRequest)?;
    let loopback = host == "localhost"
        || host.trim_matches(['[', ']']).parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if !(url.scheme() == "https" || url.scheme() == "http" && (custom || loopback))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(NetworkError::InvalidRequest);
    }
    Ok(url)
}

fn transport_error(error: reqwest::Error) -> NetworkError {
    NetworkError::Upstream(if error.is_timeout() {
        UsageQueryErrorCode::Timeout
    } else {
        UsageQueryErrorCode::Transport
    })
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;
