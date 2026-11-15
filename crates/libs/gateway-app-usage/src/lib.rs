//! 应用额度读取、刷新操作与脚本设置客户端。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct GatewayAppUsageClient {
    transport: gateway_http::GatewayTransport,
}
#[cfg(not(target_arch = "wasm32"))]
impl GatewayAppUsageClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self { Self { transport } }
    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint { self.transport.endpoint() }
}
#[cfg(not(target_arch = "wasm32"))]
mod apps_usage_read_api;
#[cfg(not(target_arch = "wasm32"))]
pub use apps_usage_read_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_read_api_tests;
#[cfg(not(target_arch = "wasm32"))]
mod apps_usage_write_api;
#[cfg(not(target_arch = "wasm32"))]
pub use apps_usage_write_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_write_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
