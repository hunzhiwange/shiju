//! Cron 调度、预览与历史领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayCronClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayCronClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod types;
pub use types::*;
mod cron_api;
#[cfg(test)]
mod test_support;
