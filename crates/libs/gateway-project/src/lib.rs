//! 项目与工作树的网关领域客户端。

mod project_api;
mod worktree_management_api;

// Project profile routes share this path encoding contract.
pub use project_api::encode_path_segment;

/// 显式复用网关传输及其连接池。
#[derive(Debug, Clone)]
pub struct GatewayProjectClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayProjectClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;
