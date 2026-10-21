use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::RequestBuilder;
use uuid::Uuid;

/// 与腾讯官方 iLink 客户端兼容的协议版本。
pub(crate) const CHANNEL_VERSION: &str = "2.4.10";
pub(crate) const APP_CLIENT_VERSION: &str = "132106";

pub(crate) fn with_application_headers(request: RequestBuilder) -> RequestBuilder {
    request
        .header("iLink-App-Id", "bot")
        .header("iLink-App-ClientVersion", APP_CLIENT_VERSION)
}

pub(crate) fn with_json_headers(request: RequestBuilder, bot_token: Option<&str>) -> RequestBuilder {
    let bytes = Uuid::new_v4().into_bytes();
    let uin = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let request = with_application_headers(request)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("AuthorizationType", "ilink_bot_token")
        .header("X-WECHAT-UIN", STANDARD.encode(uin.to_string()));
    match bot_token.map(str::trim).filter(|token| !token.is_empty()) {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}
