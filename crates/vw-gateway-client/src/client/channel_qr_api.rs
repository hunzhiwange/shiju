//! 渠道授权兼容门面。
use super::GatewayClient;
use gateway_channel_qr::GatewayChannelQrClient;
pub use gateway_channel_qr::{ChannelQrFetchResponseDto, ChannelQrStatusDto, ChannelQrStatusResponseDto};

impl GatewayClient {
    pub async fn channel_qr_fetch(&self, channel: &str) -> Result<ChannelQrFetchResponseDto, String> {
        GatewayChannelQrClient::new(self.transport.clone()).channel_qr_fetch(channel).await
    }

    pub async fn channel_qr_status(
        &self,
        channel: &str,
        handle: &str,
    ) -> Result<ChannelQrStatusResponseDto, String> {
        GatewayChannelQrClient::new(self.transport.clone()).channel_qr_status(channel, handle).await
    }
}
