//! 通道普通 HTTP 请求的期限；长轮询和媒体传输在请求处显式覆盖。

use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn client_builder() -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder();
    #[cfg(not(target_arch = "wasm32"))]
    let builder = builder.timeout(REQUEST_TIMEOUT).connect_timeout(CONNECT_TIMEOUT);
    builder
}

pub(super) fn proxied_client(service_key: &str) -> reqwest::Client {
    crate::app::agent::config::build_runtime_proxy_client_with_timeouts(
        service_key,
        REQUEST_TIMEOUT.as_secs(),
        CONNECT_TIMEOUT.as_secs(),
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "http_tests.rs"]
mod tests;
