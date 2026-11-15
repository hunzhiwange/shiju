//! 网关配置访问与租户范围校验。

mod config_api;

/// 复用共享 HTTP 连接池的配置领域客户端。
#[derive(Debug, Clone)]
pub struct GatewayConfigClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayConfigClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;

#[cfg(not(target_arch = "wasm32"))]
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
