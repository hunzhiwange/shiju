//! 应用额度读取、刷新操作与脚本设置客户端。
#[derive(Debug, Clone)]
pub struct GatewayAppUsageClient {
    transport: gateway_http::GatewayTransport,
}
impl GatewayAppUsageClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self { Self { transport } }
    pub fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint { self.transport.endpoint() }
}
mod apps_usage_read_api;
pub use apps_usage_read_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_read_api_tests;
mod apps_usage_write_api;
pub use apps_usage_write_api::*;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod apps_usage_write_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
