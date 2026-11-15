//! 渠道二维码授权客户端，共享网关传输及连接池。

#[derive(Debug, Clone)]
pub struct GatewayChannelQrClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayChannelQrClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod channel_qr_api;
pub use channel_qr_api::{ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod channel_qr_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
