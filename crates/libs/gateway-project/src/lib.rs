//! 项目与工作树的网关领域客户端。

#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
mod routes;
#[cfg(not(target_arch = "wasm32"))]
pub mod project_profiles;
#[cfg(not(target_arch = "wasm32"))]
pub mod profile_store;
#[cfg(not(target_arch = "wasm32"))]
pub mod profile_operations;
#[cfg(not(target_arch = "wasm32"))]
pub use host::ProjectHost;
#[cfg(not(target_arch = "wasm32"))]
pub use routes::{router, load_project, project_scope, project_context_directory};
#[cfg(all(not(target_arch = "wasm32"), feature = "test-support"))]
#[doc(hidden)]
pub mod testing { pub use super::routes::*; }

mod project_api;
mod git_api;
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_16_tests;
