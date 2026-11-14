//! Bounded WebDAV reads; Gateway authorizes network access and supplies credentials.
use std::time::Duration;

use reqwest::{Client, Method, StatusCode, Url, header};
use tokio::time::{Instant, timeout_at};

use super::{WebDavError, status_error};
use crate::device_sync::{
    artifact::{ArtifactDeclaration, ArtifactVerifier, MAX_ARTIFACT_SIZE},
    manifest::MAX_MANIFEST_BYTES,
    webdav_config::WebDavConfig,
    webdav_url::{build_artifact_url, build_client},
};

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
pub struct WebDavDownloadTarget {
    config: WebDavConfig,
    version: u32,
    client: Client,
}

impl WebDavDownloadTarget {
    pub fn new(config: WebDavConfig, data_compat_version: u32) -> Result<Self, WebDavError> {
        let config = config.normalize().map_err(|message| error("GET", message))?;
        if !config.enabled || config.base_url.is_empty() || config.username.is_empty() {
            return Err(error("GET", "下载需要启用的 WebDAV 地址和用户名"));
        }
        build_artifact_url(&config.base_url, &config.remote_root, &config.profile,
            data_compat_version, "manifest.json").map_err(|message| error("GET", message))?;
        let client = build_client().map_err(|_| error("GET", "无法创建 WebDAV 客户端"))?;
        Ok(Self { config, version: data_compat_version, client })
    }

    /// Only 404 returns None. Authentication failures and missing ETag are errors.
    pub async fn head(&self, artifact: &str) -> Result<Option<RemoteMetadata>, WebDavError> {
        let response = self.send(Method::HEAD, artifact, Instant::now() + IDLE_TIMEOUT).await?;
        response.map(|response| metadata(&response, "HEAD")).transpose()
    }

    /// Manifest is bounded separately; parsing/compatibility validation belongs to its owner.
    pub async fn download_manifest(&self) -> Result<Option<DownloadedArtifact>, WebDavError> {
        self.read("manifest.json", MAX_MANIFEST_BYTES as u64, None).await
    }

    /// A trusted manifest declaration supplies both the byte budget and digest.
    /// Successful EOF, exact size and SHA256 are required before bytes are returned.
    pub async fn download(&self, declaration: ArtifactDeclaration)
        -> Result<Option<DownloadedArtifact>, WebDavError>
    {
        let verifier = ArtifactVerifier::new(declaration.clone(), None)
            .map_err(|message| error("GET", message))?;
        self.read(&declaration.name, declaration.size, Some(verifier)).await
    }

    async fn send(&self, method: Method, artifact: &str, deadline: Instant)
        -> Result<Option<reqwest::Response>, WebDavError>
    {
        let operation = if method == Method::HEAD { "HEAD" } else { "GET" };
        let url: Url = build_artifact_url(&self.config.base_url, &self.config.remote_root,
            &self.config.profile, self.version, artifact)
            .map_err(|message| error(operation, message))?;
        let request = self.client.request(method, url.clone())
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header(header::ACCEPT_ENCODING, "identity");
        let response = timeout_at(deadline.min(Instant::now() + IDLE_TIMEOUT), request.send())
            .await.map_err(|_| error(operation, "请求超时"))?
            .map_err(|_| error(operation, "网络请求失败"))?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None); }
        // Partial content must never masquerade as a complete artifact.
        if response.status() != StatusCode::OK {
            return Err(status_error(operation, response.status(), &url));
        }
        Ok(Some(response))
    }

    async fn read(&self, artifact: &str, limit: u64, mut verifier: Option<ArtifactVerifier>)
        -> Result<Option<DownloadedArtifact>, WebDavError>
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

fn metadata(response: &reqwest::Response, operation: &'static str) -> Result<RemoteMetadata, WebDavError> {
    let etag = response.headers().get(header::ETAG).and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| error(operation, "远端响应缺少有效 ETag"))?.to_owned();
    let content_length = response.headers().get(header::CONTENT_LENGTH).map(|value| {
        value.to_str().ok().and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| error(operation, "远端 Content-Length 无效"))
    }).transpose()?;
    Ok(RemoteMetadata { etag, content_length })
}

fn error(operation: &'static str, message: &'static str) -> WebDavError {
    WebDavError { operation, status: None, message }
}
