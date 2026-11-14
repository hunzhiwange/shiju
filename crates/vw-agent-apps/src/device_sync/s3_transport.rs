//! Gateway-authorized S3 connection probe. Does not persist or create remote data.

use std::{fmt, time::{Duration, SystemTime}};

use chrono::{DateTime, Utc};
use reqwest::{Client, Method, StatusCode, header::HeaderMap, redirect::Policy};

use super::{s3_config::S3Config, s3_signing::sign_request};

/// Safe to display: excludes credentials, endpoint, response body and source errors.
#[derive(Debug)]
pub struct S3Error {
    pub status: Option<StatusCode>,
    pub message: &'static str,
}

impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S3 HEAD: {}", self.message)?;
        if let Some(status) = self.status {
            write!(f, " (HTTP {})", status.as_u16())?;
        }
        Ok(())
    }
}

impl std::error::Error for S3Error {}

fn local_error(message: &'static str) -> S3Error {
    S3Error { status: None, message }
}

/// Sends one signed HEAD to the bucket, with no retries or redirects.
/// The host must authorize network access and resolve explicit credentials first.
/// An explicit probe may use a disabled draft; all required fields are still checked.
/// Success confirms bucket access only, not write permission or an empty bucket.
pub async fn test_connection(mut config: S3Config) -> Result<(), S3Error> {
    config.enabled = true;
    let config = config.normalize().map_err(local_error)?;
    let client = Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| local_error("无法创建 S3 客户端"))?;
    let now: DateTime<Utc> = SystemTime::now().into();
    let request = sign_request(
        &config, Method::HEAD, "", &[], HeaderMap::new(), Vec::new(), now,
    ).map_err(local_error)?;
    let response = client.execute(request).await.map_err(|error| {
        local_error(if error.is_timeout() { "请求超时" } else { "网络请求失败" })
    })?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let message = match status {
        StatusCode::UNAUTHORIZED => "认证失败：请检查访问密钥与签名配置",
        StatusCode::FORBIDDEN => "访问被禁止：请检查凭据及桶访问权限",
        StatusCode::NOT_FOUND => "桶不存在：请检查桶名称与端点配置",
        s if s.is_redirection() => "服务器返回重定向：请检查区域与最终端点配置",
        _ => "服务器拒绝 S3 桶连接请求",
    };
    Err(S3Error { status: Some(status), message })
}
