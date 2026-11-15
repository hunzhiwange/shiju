//! 模型提供商列表与 OAuth 认证访问。

mod provider_api;
pub use provider_api::{
    ProviderAuthMethodDto, ProviderAuthMethodsResponse, ProviderListResponse,
    ProviderOAuthAuthorizationDto,
};

/// 复用共享 HTTP 连接池的模型提供商客户端。
#[derive(Debug, Clone)]
pub struct GatewayModelProviderClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayModelProviderClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;
