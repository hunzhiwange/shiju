//! 工作流应用、运行与 SSE 领域客户端。
#[cfg(not(target_arch = "wasm32"))]
pub mod tool_provider;

#[derive(Debug, Clone)]
pub struct GatewayWorkflowClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayWorkflowClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod workflow_api;
mod types;
pub use types::{GatewayWorkflowDefaultStreamRequest, GatewayWorkflowResponseMode, GatewayWorkflowRunOutcome, GatewayWorkflowStreamError, GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest};

#[cfg(test)]
mod test_support;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_17_tests;
