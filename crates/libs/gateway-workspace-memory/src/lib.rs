//! 授权执行主机的工作区文件与每日记忆客户端。
#[derive(Debug, Clone)]
pub struct GatewayWorkspaceMemoryClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayWorkspaceMemoryClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod workspace_memory_api;
pub use workspace_memory_api::{WorkspaceMemoryScope, WorkspaceMemoryError, WorkspaceSearch, WorkspaceSearchResponse};
mod workspace_directory_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod workspace_memory_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
