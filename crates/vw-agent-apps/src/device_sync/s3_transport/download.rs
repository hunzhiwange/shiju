//! Bounded S3 reads; Gateway authorizes network access and supplies credentials.
use std::{fmt, time::Duration};

use chrono::Utc;
use reqwest::{Client, Method, StatusCode, header::{self, HeaderMap, HeaderValue}, redirect::Policy};
use tokio::time::{Instant, timeout_at};


use crate::device_sync::{
    artifact::{ArtifactDeclaration, ArtifactVerifier, MAX_ARTIFACT_SIZE},
    manifest::MAX_MANIFEST_BYTES,
    s3_config::S3Config,
    s3_signing::sign_request,
    manifest,
    webdav_config::decode_path_segment,
};

/// Safe to display: excludes credentials, URL, response body and transport sources.
#[derive(Debug)]
pub struct S3DownloadError {
    pub operation: &'static str,
    pub status: Option<StatusCode>,
    pub message: &'static str,
}

impl fmt::Display for S3DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S3 {}: {}", self.operation, self.message)?;
        if let Some(status) = self.status { write!(f, " (HTTP {})", status.as_u16())?; }
        Ok(())
    }
}
impl std::error::Error for S3DownloadError {}


const IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// ETag is opaque and preserved for subsequent conditional requests.
pub struct RemoteMetadata {
    pub etag: String,
    pub content_length: Option<u64>,
}

/// Bytes remain staged in memory, bounded independently of HTTP headers.
/// Nothing is persisted or applied by this transport.
pub struct DownloadedArtifact {
    pub metadata: RemoteMetadata,
    pub bytes: Vec<u8>,
}

/// Owns credentials deliberately without Debug. No redirects or automatic retries.
pub struct S3DownloadTarget {
    config: S3Config,
    prefix: String,
    client: Client,
}

impl S3DownloadTarget {
    pub fn new(config: S3Config, data_compat_version: u32) -> Result<Self, S3DownloadError> {
        let config = config.normalize().map_err(|message| error("GET", message))?;
        if !config.enabled {
            return Err(error("GET", "下载需要启用 S3 配置"));
        }
        if data_compat_version == 0 {
            return Err(error("GET", "缺少共享快照兼容版本"));
        }
        // Decode once; the signer owns final object-key encoding, as for uploads.
        let root = config.remote_root.split('/').map(decode_path_segment)
            .collect::<Result<Vec<_>, _>>().map_err(|message| error("GET", message))?.join("/");
        let profile = decode_path_segment(&config.profile).map_err(|message| error("GET", message))?;
        let prefix = format!("{root}/{profile}/{}/v{}/data-v{data_compat_version}",
            manifest::FORMAT, manifest::PROTOCOL_VERSION);
        sign_request(&config, Method::GET, &format!("{prefix}/manifest.json"),
            &[], HeaderMap::new(), Vec::new(), Utc::now())
            .map_err(|message| error("GET", message))?;
        // No total client timeout: trusted size supplies the transfer budget below.
        let client = Client::builder().redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .no_gzip().no_brotli().no_deflate().no_zstd()
            .build().map_err(|_| error("GET", "无法创建 S3 客户端"))?;
        Ok(Self { config, prefix, client })
    }

    /// Encoded manifest path only, using the same addressing as the actual GET.
    /// Excludes the origin, credentials and query parameters.
    pub fn manifest_path(&self) -> Result<String, S3DownloadError> {
        super::super::s3_signing::build_object_url(
            &self.config,
            &format!("{}/manifest.json", self.prefix),
        )
        .map(|url| url.path().to_owned())
        .map_err(|message| error("query", message))
    }

    /// Only 404 returns None. Authentication failures and missing ETag are errors.
    pub async fn head(&self, artifact: &str) -> Result<Option<RemoteMetadata>, S3DownloadError> {
        let response = self.send(Method::HEAD, artifact, Instant::now() + IDLE_TIMEOUT).await?;
        response.map(|response| metadata(&response, "HEAD")).transpose()
    }

    /// Manifest is bounded separately; parsing/compatibility validation belongs to its owner.
    pub async fn download_manifest(&self) -> Result<Option<DownloadedArtifact>, S3DownloadError> {
        self.read("manifest.json", MAX_MANIFEST_BYTES as u64, None).await
    }

    /// A trusted manifest declaration supplies both the byte budget and digest.
    /// Successful EOF, exact size and SHA256 are required before bytes are returned.
    pub async fn download(&self, declaration: ArtifactDeclaration)
        -> Result<Option<DownloadedArtifact>, S3DownloadError>
    {
        let verifier = ArtifactVerifier::new(declaration.clone(), None)
            .map_err(|message| error("GET", message))?;
        self.read(&declaration.name, declaration.size, Some(verifier)).await
    }

    async fn send(&self, method: Method, artifact: &str, deadline: Instant)
        -> Result<Option<reqwest::Response>, S3DownloadError>
    {
        let operation = if method == Method::HEAD { "HEAD" } else { "GET" };
        if !matches!(artifact, manifest::CONFIG_ARTIFACT | manifest::SKILLS_ARTIFACT | "manifest.json") {
            return Err(error(operation, "不支持的同步制品"));
        }
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
        let request = sign_request(&self.config, method,
            &format!("{}/{}", self.prefix, artifact), &[], headers, Vec::new(), Utc::now())
            .map_err(|message| error(operation, message))?;
        // Send the signed request intact; never replay credentials to a redirect.
        let response = timeout_at(deadline.min(Instant::now() + IDLE_TIMEOUT), self.client.execute(request))
            .await.map_err(|_| error(operation, "请求超时"))?
            .map_err(|_| error(operation, "网络请求失败"))?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None); }
        // Partial content must never masquerade as a complete artifact.
        if response.status() != StatusCode::OK {
            return Err(S3DownloadError { operation, status: Some(response.status()), message: match response.status() {
                StatusCode::UNAUTHORIZED => "认证失败：请检查 S3 凭据",
                StatusCode::FORBIDDEN => "访问被禁止：请检查 S3 凭据和读取权限",
                status if status.is_redirection() => "服务器返回重定向：请配置最终 S3 地址和区域",
                _ => "服务器拒绝完整 S3 下载请求",
            } });
        }
        if response.headers().get(header::CONTENT_ENCODING)
            .is_some_and(|value| value.as_bytes() != b"identity") {
            return Err(error(operation, "远端返回不支持的内容编码"));
        }
        Ok(Some(response))
    }

    async fn read(&self, artifact: &str, limit: u64, mut verifier: Option<ArtifactVerifier>)
        -> Result<Option<DownloadedArtifact>, S3DownloadError>
    {
        if limit > MAX_ARTIFACT_SIZE { return Err(error("GET", "制品超过大小限制")); }
        // Budget uses trusted size (or manifest cap), never untrusted Content-Length.
        let deadline = Instant::now() + Duration::from_secs(300 + limit.div_ceil(16 * 1024));
        let Some(mut response) = self.send(Method::GET, artifact, deadline).await? else {
            return Ok(None);
        };
        let metadata = metadata(&response, "GET")?;
        if metadata.content_length.is_some_and(|length| length > limit
            || (verifier.is_some() && length != limit)) {
            return Err(error("GET", "Content-Length 与制品大小限制不符"));
        }
        let mut bytes = Vec::new();
        let mut idle_deadline = Instant::now() + IDLE_TIMEOUT;
        loop {
            let chunk = timeout_at(deadline.min(idle_deadline), response.chunk()).await
                .map_err(|_| error("GET", "下载总时限或空闲时限已到"))?
                .map_err(|_| error("GET", "下载传输中断"))?;
            let Some(chunk) = chunk else { break; };
            if chunk.is_empty() { continue; }
            idle_deadline = Instant::now() + IDLE_TIMEOUT;
            let size = (bytes.len() as u64).checked_add(chunk.len() as u64)
                .filter(|size| *size <= limit)
                .ok_or_else(|| error("GET", "下载流超过制品大小限制"))?;
            if let Some(verifier) = verifier.as_mut() {
                verifier.update(&chunk).map_err(|message| error("GET", message))?;
            }
            bytes.try_reserve(chunk.len()).map_err(|_| error("GET", "无法分配下载缓冲区"))?;
            bytes.extend_from_slice(&chunk);
            debug_assert_eq!(bytes.len() as u64, size);
        }
        if metadata.content_length.is_some_and(|length| length != bytes.len() as u64) {
            return Err(error("GET", "下载长度与 Content-Length 不符"));
        }
        if let Some(verifier) = verifier {
            verifier.finish().map_err(|message| error("GET", message))?;
        }
        Ok(Some(DownloadedArtifact { metadata, bytes }))
    }
}

fn metadata(response: &reqwest::Response, operation: &'static str) -> Result<RemoteMetadata, S3DownloadError> {
    let etag = response.headers().get(header::ETAG).and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| error(operation, "远端响应缺少有效 ETag"))?.to_owned();
    let content_length = response.headers().get(header::CONTENT_LENGTH).map(|value| {
        value.to_str().ok().and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| error(operation, "远端 Content-Length 无效"))
    }).transpose()?;
    Ok(RemoteMetadata { etag, content_length })
}

fn error(operation: &'static str, message: &'static str) -> S3DownloadError {
    S3DownloadError { operation, status: None, message }
}
