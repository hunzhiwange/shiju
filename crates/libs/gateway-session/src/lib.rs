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
