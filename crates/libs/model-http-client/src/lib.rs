//! Shared HTTP client defaults for model relay and voice providers.
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Common upstream client for relay transports: credentials must never follow redirects.
pub fn upstream_http_client() -> Result<reqwest::Client, reqwest::Error> {
    upstream_http_client_builder().build()
}

pub fn upstream_http_client_builder() -> reqwest::ClientBuilder {
    upstream_http_client_builder_without_total_timeout().timeout(REQUEST_TIMEOUT)
}

pub fn upstream_http_client_builder_without_total_timeout() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
}
