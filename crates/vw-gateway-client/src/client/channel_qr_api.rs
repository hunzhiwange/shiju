use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::GatewayClient;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ChannelQrFetchResponseDto {
    pub channel: String,
    pub qrcode_png_base64: String,
    pub handle: String,
    pub expires_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelQrStatusDto {
    Pending,
    Scanned,
    Authorized,
    Expired,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ChannelQrStatusResponseDto {
    pub status: ChannelQrStatusDto,
    #[serde(default)]
    pub credentials: Option<Map<String, Value>>,
}

impl GatewayClient {
    pub async fn channel_qr_fetch(
        &self,
        channel: &str,
    ) -> Result<ChannelQrFetchResponseDto, String> {
        let channel = required_segment("channel", channel)?;
        self.post_json(&format!("/v1/channels/{channel}/qrcode"), &[], &()).await
    }

    pub async fn channel_qr_status(
        &self,
        channel: &str,
        handle: &str,
    ) -> Result<ChannelQrStatusResponseDto, String> {
        let channel = required_segment("channel", channel)?;
        let handle = handle.trim();
        if handle.is_empty() {
            return Err("QR authorization handle must not be empty".to_string());
        }
        let query = [("handle".to_string(), handle.to_string())];
        self.get_json(&format!("/v1/channels/{channel}/qrcode/status"), &query).await
    }
}

fn required_segment<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if !value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!("{name} contains unsupported characters"));
    }
    Ok(value)
}
