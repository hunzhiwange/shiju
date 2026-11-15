//! 工具目录、问题、权限及网关运维交互客户端。

#[derive(Debug, Clone)]
pub struct GatewayInteractionClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayInteractionClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod tools_api;
mod question_api;
mod permission_api;
mod control_api;
mod operations_api;
pub use permission_api::{PendingPermissionReplyDto, PendingPermissionRequestDto, PendingPermissionToolDto};
#[cfg(test)]
mod test_support;
