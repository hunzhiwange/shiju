//! Gateway-authorized WebDAV probes and fixed-artifact uploads. Never persists settings.

pub mod download;

use std::{fmt, time::Duration};

use reqwest::{Client, Method, StatusCode, Url};

use super::{
    webdav_config::{WebDavConfig, decode_path_segment},
    webdav_url::build_client,
};

/// Safe to display: contains no URL, credentials, response body or transport source.
#[derive(Debug)]
pub struct WebDavError {
    pub operation: &'static str,
    pub status: Option<StatusCode>,
    pub message: &'static str,
}

impl fmt::Display for WebDavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WebDAV {}: {}", self.operation, self.message)?;
        if let Some(status) = self.status {
            write!(f, " (HTTP {})", status.as_u16())?;
        }
        Ok(())
    }
}

impl std::error::Error for WebDavError {}

fn local_error(message: &'static str) -> WebDavError {
    WebDavError { operation: "connection", status: None, message }
}

/// Probe the base collection, then ensure remote_root/profile exists.
/// Caller must authorize host network access and resolve credentials beforehand.
/// Disabled drafts are allowed, but an explicit probe requires address and username.
/// Redirects are never followed; no settings, snapshots or archives are written.
pub async fn test_connection(config: WebDavConfig) -> Result<(), WebDavError> {
    let config = config.normalize().map_err(local_error)?;
    if config.base_url.is_empty() || config.username.is_empty() {
        return Err(local_error("连接测试需要 WebDAV 地址和用户名"));
    }
    let mut url = Url::parse(&config.base_url)
        .map_err(|_| local_error("WebDAV 地址无效"))?;
    // Validate the entire chain before the first network side effect.
    let segments = config.remote_root.split('/')
        .chain(std::iter::once(config.profile.as_str()))
        .map(decode_path_segment)
        .collect::<Result<Vec<_>, _>>()
        .map_err(local_error)?;
    let client = build_client().map_err(|_| local_error("无法创建 WebDAV 客户端"))?;
    let status = request(&client, "PROPFIND", &url, &config).await?;
    if !status.is_success() {
        return Err(status_error("PROPFIND", status, &url));
    }
    for segment in segments {
        url.path_segments_mut()
            .map_err(|_| local_error("WebDAV 地址无法追加目录"))?
            .pop_if_empty().push(&segment).push("");
        let status = request(&client, "MKCOL", &url, &config).await?;
        if status.is_success() {
            continue;
        }
        // Existing collections commonly return 405; some providers use 409.
        // Verify the original URL, never Location, and preserve permission errors.
        if matches!(status, StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT) {
            let probe = request(&client, "PROPFIND", &url, &config).await?;
            if probe.is_success() {
                continue;
            }
            if probe != StatusCode::NOT_FOUND {
                return Err(status_error("PROPFIND", probe, &url));
            }
        }
        return Err(status_error("MKCOL", status, &url));
    }
    Ok(())
}

async fn request(
    client: &Client,
    operation: &'static str,
    url: &Url,
    config: &WebDavConfig,
) -> Result<StatusCode, WebDavError> {
    let method = Method::from_bytes(operation.as_bytes())
        .map_err(|_| local_error("WebDAV 请求方法无效"))?;
    let mut request = client.request(method, url.clone())
        .basic_auth(&config.username, Some(&config.password))
        .timeout(Duration::from_secs(30));
    if operation == "PROPFIND" {
        request = request.header("Depth", "0");
    }
    request.send().await.map(|response| response.status()).map_err(|error| WebDavError {
        operation,
        status: None,
        message: if error.is_timeout() { "请求超时" } else { "网络请求失败" },
    })
}

fn status_error(operation: &'static str, status: StatusCode, url: &Url) -> WebDavError {
    let nutstore = url.host_str().is_some_and(|host| {
        ["jianguoyun.com", "nutstore.net", "nutstore.com"].iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    });
    let message = match status {
        StatusCode::UNAUTHORIZED if nutstore => "认证失败：坚果云请使用第三方应用密码，并确认目录位于 /dav/ 下",
        StatusCode::UNAUTHORIZED => "认证失败：请检查用户名和密码",
        StatusCode::FORBIDDEN if nutstore => "访问被禁止：请检查坚果云应用密码和 /dav/ 目录读写权限",
        StatusCode::FORBIDDEN => "访问被禁止：请检查目录读写权限",
        StatusCode::CONFLICT if operation == "MKCOL" && nutstore => "父目录不存在：请先在坚果云网页端创建顶层文件夹",
        StatusCode::CONFLICT if operation == "MKCOL" => "父目录不存在：请确认上级目录存在",
        StatusCode::METHOD_NOT_ALLOWED if operation == "MKCOL" => "无法创建目录，且未确认已有目录可访问",
        StatusCode::NOT_FOUND if nutstore => "目录不存在：请确认地址位于坚果云 /dav/ 可写目录下",
        StatusCode::NOT_FOUND => "远端目录不存在",
        s if s.is_redirection() => "服务器返回重定向：请配置最终 WebDAV 地址",
        _ => "服务器拒绝 WebDAV 请求",
    };
    WebDavError { operation, status: Some(status), message }
}

/// Closed upload names: callers cannot supply paths or arbitrary destinations.
#[derive(Clone, Copy)]
pub enum UploadArtifact {
    Config,
    Skills,
    Manifest,
}

impl UploadArtifact {
    fn name(self) -> &'static str {
        match self {
            Self::Config => super::manifest::CONFIG_ARTIFACT,
            Self::Skills => super::manifest::SKILLS_ARTIFACT,
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

/// A locally validated destination. Construction does not authorize network use.
/// Gateway must resolve credentials and authorize host access before upload.
/// No Debug implementation: this value owns execution-side credentials.
pub struct WebDavUploadTarget {
    config: WebDavConfig,
    data_compat_version: u32,
    client: Client,
}

impl WebDavUploadTarget {
    /// Compatibility must be supplied by the shared snapshot owner.
    pub fn new(config: WebDavConfig, data_compat_version: u32) -> Result<Self, WebDavError> {
        let config = config.normalize().map_err(upload_error)?;
        if !config.enabled || config.base_url.is_empty() || config.username.is_empty() {
            return Err(upload_error("上传需要启用的 WebDAV 地址和用户名"));
        }
        // Validate every destination component before accepting any bytes.
        super::webdav_url::build_artifact_url(
            &config.base_url, &config.remote_root, &config.profile,
            data_compat_version, "manifest.json",
        ).map_err(upload_error)?;
        let client = build_client().map_err(|_| upload_error("无法创建 WebDAV 客户端"))?;
        Ok(Self { config, data_compat_version, client })
    }

    /// Prepare the native version collections, retaining existing collections only
    /// after a successful probe. No artifact or manifest is written here.
    pub async fn prepare_collections(&self) -> Result<(), WebDavError> {
        test_connection(self.config.clone()).await?;
        let mut url = super::webdav_url::build_artifact_url(
            &self.config.base_url, &self.config.remote_root, &self.config.profile,
            self.data_compat_version, "manifest.json",
        ).map_err(upload_error)?;
        url.path_segments_mut().map_err(|_| upload_error("Invalid collection"))?.pop();
        let mut collections = Vec::new();
        for _ in 0..3 {
            collections.push(url.clone());
            url.path_segments_mut().map_err(|_| upload_error("Invalid collection"))?.pop();
        }
        for mut url in collections.into_iter().rev() {
            url.path_segments_mut().map_err(|_| upload_error("Invalid collection"))?.push("");
            let status = request(&self.client, "MKCOL", &url, &self.config).await?;
            if status.is_success() { continue; }
            if matches!(status, StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT)
                && request(&self.client, "PROPFIND", &url, &self.config).await?.is_success()
            { continue; }
            return Err(status_error("MKCOL", status, &url));
        }
        Ok(())
    }

    /// Upload exactly these staged bytes. The caller owns snapshot filtering,
    /// directory preparation and publication order (manifest last).
    /// A timeout may leave remote bytes written; no automatic retry is performed.
    pub async fn upload(&self, artifact: UploadArtifact, bytes: Vec<u8>) -> Result<(), WebDavError> {
        let limit = match artifact {
            UploadArtifact::Manifest => super::manifest::MAX_MANIFEST_BYTES as u64,
            _ => super::artifact::MAX_ARTIFACT_SIZE,
        };
        if bytes.len() as u64 > limit {
            return Err(upload_error("上传制品超过大小限制"));
        }
        let url = super::webdav_url::build_artifact_url(
            &self.config.base_url, &self.config.remote_root, &self.config.profile,
            self.data_compat_version, artifact.name(),
        ).map_err(upload_error)?;
        let response = self.client.put(url.clone())
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header(reqwest::header::CONTENT_TYPE, artifact.content_type())
            .timeout(Duration::from_secs(30))
            .body(bytes)
            .send().await.map_err(|error| WebDavError {
                operation: "PUT",
                status: None,
                message: if error.is_timeout() { "请求超时" } else { "网络请求失败" },
            })?;
        if !response.status().is_success() {
            return Err(status_error("PUT", response.status(), &url));
        }
        Ok(())
    }
}

fn upload_error(message: &'static str) -> WebDavError {
    WebDavError { operation: "PUT", status: None, message }
}
