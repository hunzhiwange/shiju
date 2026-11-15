//! Gateway-authorized S3 transport. Never persists local settings.

pub mod download;
pub use download::{DownloadedArtifact, RemoteMetadata, S3DownloadError, S3DownloadTarget};

use std::{fmt, time::{Duration, SystemTime}};

use chrono::{DateTime, Utc};
use reqwest::{Client, Method, StatusCode, header::{self, HeaderMap, HeaderValue}, redirect::Policy};

use super::{manifest, s3_config::S3Config, s3_signing::sign_request};

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

/// Safe to display: never retains credentials, URLs, response bodies or sources.
#[derive(Debug)]
pub struct S3UploadError {
    pub operation: &'static str,
    pub status: Option<StatusCode>,
    pub message: &'static str,
}

impl fmt::Display for S3UploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S3 {}: {}", self.operation, self.message)?;
        if let Some(status) = self.status {
            write!(f, " (HTTP {})", status.as_u16())?;
        }
        Ok(())
    }
}

impl std::error::Error for S3UploadError {}

fn upload_error(message: &'static str) -> S3UploadError {
    S3UploadError { operation: "PUT", status: None, message }
}

/// Closed artifact names prevent arbitrary object destinations.
#[derive(Clone, Copy)]
pub enum UploadArtifact {
    Config,
    Skills,
    Manifest,
}

impl UploadArtifact {
    fn name(self) -> &'static str {
        match self {
            Self::Config => manifest::CONFIG_ARTIFACT,
            Self::Skills => manifest::SKILLS_ARTIFACT,
            Self::Manifest => "manifest.json",
        }
    }

    fn content_type(self) -> &'static str {
        match self {
            Self::Config => "application/json",
            Self::Skills => "application/zip",
            Self::Manifest => "application/octet-stream",
        }
    }
}

/// Owns execution-side secrets, deliberately without Debug or serialization.
/// Construction grants no network permission; Gateway must authorize execution.
pub struct S3UploadTarget {
    config: S3Config,
    prefix: String,
    client: Client,
}

impl S3UploadTarget {
    /// Compatibility version must come from the shared snapshot owner.
    pub fn new(config: S3Config, data_compat_version: u32) -> Result<Self, S3UploadError> {
        let config = config.normalize().map_err(upload_error)?;
        if !config.enabled {
            return Err(upload_error("上传需要启用 S3 配置"));
        }
        if data_compat_version == 0 {
            return Err(upload_error("缺少共享快照兼容版本"));
        }
        // Configuration permits a single percent-encoding layer. Decode before
        // passing raw keys to the signer, which performs the final URI encoding.
        let root = config.remote_root.split('/')
            .map(super::webdav_config::decode_path_segment)
            .collect::<Result<Vec<_>, _>>().map_err(upload_error)?.join("/");
        let profile = super::webdav_config::decode_path_segment(&config.profile)
            .map_err(upload_error)?;
        let prefix = format!("{root}/{profile}/{}/v{}/data-v{data_compat_version}",
            manifest::FORMAT, manifest::PROTOCOL_VERSION);
        // Validate addressing and signing credentials before accepting payloads.
        sign_request(&config, Method::PUT,
            &format!("{prefix}/manifest.json"), &[], HeaderMap::new(), Vec::new(), Utc::now())
            .map_err(upload_error)?;
        let client = Client::builder().redirect(Policy::none())
            .timeout(Duration::from_secs(30)).build()
            .map_err(|_| upload_error("无法创建 S3 客户端"))?;
        Ok(Self { config, prefix, client })
    }

    /// Sends these exact bytes once. Caller owns snapshot filtering and publishes
    /// manifest last. Timeout/network failures may leave remote bytes written.
    pub async fn upload(&self, artifact: UploadArtifact, bytes: Vec<u8>) -> Result<(), S3UploadError> {
        let limit = match artifact {
            UploadArtifact::Manifest => manifest::MAX_MANIFEST_BYTES as u64,
            _ => super::artifact::MAX_ARTIFACT_SIZE,
        };
        if bytes.len() as u64 > limit {
            return Err(upload_error("上传制品超过大小限制"));
        }
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(artifact.content_type()));
        let request = sign_request(&self.config, Method::PUT,
            &format!("{}/{}", self.prefix, artifact.name()), &[], headers, bytes, Utc::now())
            .map_err(upload_error)?;
        // Execute the signed request directly: never rebuild or mutate its body,
        // URL or signed headers, and never follow a credential-bearing redirect.
        let response = self.client.execute(request).await.map_err(|error| {
            upload_error(if error.is_timeout() { "请求超时" } else { "网络请求失败" })
        })?;
        let status = response.status();
        if !status.is_success() {
            let message = match status {
                StatusCode::UNAUTHORIZED => "认证失败：请检查 S3 凭据",
                StatusCode::FORBIDDEN => "访问被禁止：请检查 S3 凭据和写入权限",
                StatusCode::NOT_FOUND => "S3 bucket 不存在或不可访问",
                s if s.is_redirection() => "服务器返回重定向：请配置最终 S3 地址和区域",
                _ => "服务器拒绝 S3 上传请求",
            };
            return Err(S3UploadError { operation: "PUT", status: Some(status), message });
        }
        Ok(())
    }
}
