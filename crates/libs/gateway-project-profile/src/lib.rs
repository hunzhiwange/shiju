//! 授权网关主机上的项目档案客户端。

pub mod project_profile_api;
pub use project_profile_api::{
    ProjectProfileApiError, ProjectProfileApiScope, ProjectProfileCurrent,
    ProjectProfileDeleted, ProjectProfileStoredResult,
};

/// 显式复用网关传输及其连接池。
#[derive(Debug, Clone)]
pub struct GatewayProjectProfileClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayProjectProfileClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}
