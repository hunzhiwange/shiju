//! 会话生命周期、消息与 UI 持久化领域客户端。
mod session_api;
mod ui_load;
pub use ui_load::SessionUiLoadResult;

#[derive(Debug, Clone)]
pub struct GatewaySessionClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewaySessionClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;

/// Archived tree validation shared by session handlers.
pub mod archived_delete;

/// Pending raw artifacts awaiting durable UI snapshot acknowledgement.
#[cfg(not(target_arch = "wasm32"))]
pub mod raw_commit;

#[cfg(not(target_arch = "wasm32"))]
pub mod message_host;
#[cfg(not(target_arch = "wasm32"))]
pub mod message_ops;
