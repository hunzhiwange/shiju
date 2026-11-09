//! Host-mediated downloads for the MCP Apps draft extension.
use base64::Engine;
use reqwest::{Url, header::LOCATION};
use serde_json::Value;
use std::{
    future::Future,
    io::{self, Write},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

const MAX_INLINE_BYTES: usize = 512 * 1024;
const MAX_REMOTE_BYTES: usize = 16 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
type DownloadResult<T> = Result<T, &'static str>;

pub(crate) struct DownloadPrompt {
    pub filename: String,
    pub origin: Option<String>,
}

pub(crate) async fn select_destination(prompt: DownloadPrompt) -> Option<PathBuf> {
    let title = match prompt.origin {
        Some(origin) => format!("从 {origin} 下载并保存文件"),
        None => "保存 MCP 应用文件".to_owned(),
    };
    rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_file_name(prompt.filename)
        .save_file()
        .await
        .map(|file| file.path().to_owned())
}

pub(super) struct Download {
    prompt: DownloadPrompt,
    contents: Contents,
}

enum Contents {
    Inline(Vec<u8>),
    Https(Url),
}

impl Download {
    pub fn parse(params: &Value, resource_domains: &[String]) -> DownloadResult<Self> {
        let Some([content]) = params["contents"].as_array().map(Vec::as_slice) else {
            return Err("Choose exactly one file per download request");
        };
        let (uri, contents, origin) = match content["type"].as_str() {
            Some("resource") => {
                let resource = &content["resource"];
                let uri = resource_uri(resource)?;
                let bytes = match (resource.get("text"), resource.get("blob")) {
                    (Some(Value::String(text)), None) if text.len() <= MAX_INLINE_BYTES => {
                        text.as_bytes().to_vec()
                    }
                    (None, Some(Value::String(blob)))
                        if blob.len() <= MAX_INLINE_BYTES.div_ceil(3) * 4 =>
                    {
                        let bytes = base64::engine::general_purpose::STANDARD
                            .decode(blob)
                            .map_err(|_| "Invalid base64 download data")?;
                        if bytes.len() > MAX_INLINE_BYTES {
                            return Err("Inline download exceeds size limit");
                        }
                        bytes
                    }
                    (Some(Value::String(_)), None) | (None, Some(Value::String(_))) => {
                        return Err("Inline download exceeds size limit");
                    }
                    _ => return Err("Download requires one text or blob field"),
                };
                (uri, Contents::Inline(bytes), None)
            }
            Some("resource_link") => {
                let uri = resource_uri(content)?;
                let url = https_url(uri)?;
                let origin = url.origin().ascii_serialization();
                if !resource_domains.contains(&origin) {
                    return Err("Download origin is not allowed");
                }
                (uri, Contents::Https(url), Some(origin))
            }
            _ => return Err("Unsupported download content type"),
        };
        Ok(Self { prompt: DownloadPrompt { filename: suggested_name(uri), origin }, contents })
    }

    pub async fn save<F: Future<Output = Option<PathBuf>>>(
        self,
        choose_path: impl FnOnce(DownloadPrompt) -> F,
        active: impl Fn() -> bool,
    ) -> DownloadResult<()> {
        if !active() {
            return Err("Download session is no longer active");
        }
        let path = choose_path(self.prompt).await.ok_or("Download cancelled by user")?;
        if !active() {
            return Err("Download session is no longer active");
        }
        let destination = Destination::new(path)?;
        let bytes = match self.contents {
            Contents::Inline(bytes) => bytes,
            Contents::Https(url) => tokio::time::timeout(Duration::from_secs(30), fetch(url))
                .await
                .map_err(|_| "Download resource timed out")??,
        };
        destination.save(&bytes, active)
    }
}

fn resource_uri(value: &Value) -> DownloadResult<&str> {
    value["uri"]
        .as_str()
        .filter(|uri| !uri.is_empty() && uri.len() <= 8192)
        .ok_or("Invalid download resource URI")
}

fn suggested_name(uri: &str) -> String {
    let url = Url::parse(uri).ok();
    let path = url.as_ref().map_or_else(|| uri.split(['?', '#']).next().unwrap_or(""), Url::path);
    let segment = path.rsplit('/').next().unwrap_or("");
    let decoded = urlencoding::decode(segment).unwrap_or_default();
    let name: String = decoded
        .chars()
        .map(|ch| {
            if ch.is_control()
                || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                    | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let name = name.trim_matches([' ', '.']);
    if name.is_empty() || name.len() > 240 {
        return "download".to_owned();
    }
    // Device names remain reserved on Windows even with an extension.
    let stem = name.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || stem.strip_prefix("COM").or_else(|| stem.strip_prefix("LPT")).is_some_and(|suffix| {
            matches!(
                suffix,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    {
        return format!("_{name}");
    }
    name.to_owned()
}

struct Destination {
    path: PathBuf,
    replace: bool,
}

impl Destination {
    fn new(path: PathBuf) -> DownloadResult<Self> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err("Invalid download destination");
        }
        let replace = match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            _ => return Err("Download destination is unavailable"),
        };
        Ok(Self { path, replace })
    }

    fn save(self, bytes: &[u8], active: impl Fn() -> bool) -> DownloadResult<()> {
        const WRITE_ERROR: &str = "Could not save download";
        if !active() {
            return Err("Download session is no longer active");
        }
        let mut temp = tempfile::NamedTempFile::new_in(self.path.parent().ok_or(WRITE_ERROR)?)
            .map_err(|_| WRITE_ERROR)?;
        temp.write_all(bytes).map_err(|_| WRITE_ERROR)?;
        temp.as_file().sync_all().map_err(|_| WRITE_ERROR)?;
        if !active() {
            return Err("Download session is no longer active");
        }
        // Save panels confirm replacement. For an initially absent file, guard
        // against a destination appearing while the download is in progress.
        if self.replace { temp.persist(self.path) } else { temp.persist_noclobber(self.path) }
            .map_err(|_| WRITE_ERROR)?;
        Ok(())
    }
}

fn https_url(raw: &str) -> DownloadResult<Url> {
    const INVALID: &str = "Download requires a public HTTPS URL without credentials";
    let authority = raw
        .strip_prefix("https://")
        .and_then(|suffix| suffix.split(['/', '?', '#']).next())
        .ok_or(INVALID)?;
    if authority.is_empty() || authority.contains(['@', '%']) || invalid_url_characters(raw) {
        return Err(INVALID);
    }
    let mut url = Url::parse(raw).map_err(|_| INVALID)?;
    let host = url.domain().ok_or(INVALID)?;
    if !host.contains('.')
        || host.ends_with('.')
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host == "home.arpa"
        || host.ends_with(".home.arpa")
    {
        return Err(INVALID);
    }
    url.set_fragment(None);
    Ok(url)
}

fn invalid_url_characters(raw: &str) -> bool {
    raw.chars().any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
}

fn redirect_target(url: &Url, location: &str) -> DownloadResult<Url> {
    const INVALID: &str = "Invalid download redirect";
    if location.len() > 8192 || invalid_url_characters(location) {
        return Err(INVALID);
    }
    // Validate a raw authority before the URL parser can normalize away empty userinfo.
    if let Some(authority) = location.strip_prefix("//") {
        https_url(&format!("https://{authority}"))?;
    } else if Url::parse(location).is_ok() {
        https_url(location)?;
    }
    let next = url.join(location).map_err(|_| INVALID)?;
    let next = https_url(next.as_str())?;
    if next.origin() != url.origin() {
        return Err("Download redirect leaves the approved origin");
    }
    Ok(next)
}

fn validate_addresses(addresses: &[SocketAddr]) -> DownloadResult<()> {
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err("Download DNS returned a nonpublic address");
    }
    Ok(())
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [first, second, third, _] = ip.octets();
            !matches!(
                (first, second, third),
                (0 | 10 | 127 | 224..=255, _, _)
                    | (100, 64..=127, _)
                    | (169, 254, _)
                    | (172, 16..=31, _)
                    | (192, 168, _)
                    | (192, 0, 0 | 2)
                    | (192, 88, 99)
                    | (198, 18 | 19, _)
                    | (198, 51, 100)
                    | (203, 0, 113)
            )
        }
        IpAddr::V6(ip) => {
            let [first, second, ..] = ip.segments();
            // Allow global unicast, excluding special-purpose and transition prefixes.
            first & 0xe000 == 0x2000
                && !matches!((first, second), (0x2001, 0..=0x01ff | 0x0db8) | (0x2002 | 0x3fff, _))
        }
    }
}

async fn fetch(mut url: Url) -> DownloadResult<Vec<u8>> {
    for redirects in 0..=MAX_REDIRECTS {
        let host = url.domain().ok_or("Invalid download host")?;
        let port = url.port_or_known_default().ok_or("Invalid download port")?;
        let addresses: Vec<_> = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| "Download DNS lookup failed")?
            .collect();
        validate_addresses(&addresses)?;
        // Pin the verified addresses for this hop and use an independent,
        // credential-free client with no automatic redirect or proxy.
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .resolve_to_addrs(host, &addresses)
            .build()
            .map_err(|_| "Could not initialize HTTPS download")?;
        let mut response =
            client.get(url.clone()).send().await.map_err(|_| "HTTPS download failed")?;
        if response.status().is_redirection() {
            if redirects == MAX_REDIRECTS {
                return Err("Too many download redirects");
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or("Invalid download redirect")?;
            url = redirect_target(&url, location)?;
            continue;
        }
        if response.status() != reqwest::StatusCode::OK {
            return Err("Download server did not return a complete file");
        }
        if response.content_length().is_some_and(|len| len > MAX_REMOTE_BYTES as u64) {
            return Err("Download resource exceeds size limit");
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "HTTPS download failed")? {
            if bytes.len() + chunk.len() > MAX_REMOTE_BYTES {
                return Err("Download resource exceeds size limit");
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok(bytes);
    }
    Err("Too many download redirects")
}

#[cfg(test)]
#[path = "mcp_app_download_tests.rs"]
mod tests;
