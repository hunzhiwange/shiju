//! 下载独立 Web 发布包，校验后在暂存目录中解压并切换安装目录。
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use vw_shared::update::{UpdateSource, desktop_release::SHIJU_MANIFEST};

const RELEASE_API: &str = "https://api.github.com/repos/hunzhiwange/shiju/releases/latest";
const DOWNLOAD_ROOT: &str = "https://github.com/hunzhiwange/shiju/releases/download/";
const SHIJU_ROOT: &str = "https://www.shiju.cloud/agents/shiju-agent";
const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub(super) fn root() -> Result<PathBuf> {
    let home = directories::BaseDirs::new().context("无法获取用户目录")?;
    Ok(vw_config_types::paths::home_config_dir(home.home_dir()).join("web"))
}

pub(super) fn installed() -> Result<Option<PathBuf>> {
    let path = root()?.join("current");
    if !path.exists() {
        return Ok(None);
    }
    validate_package(&path)?;
    Ok(Some(path))
}

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("ShiJu/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .https_only(true)
        .build()
        .context("创建 Web 下载客户端失败")
}

fn download(client: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<Vec<u8>> {
    let response = client.get(url).send()?.error_for_status()?;
    if response.content_length().is_some_and(|size| size > limit) {
        bail!("Web 下载文件过大");
    }
    let mut data = Vec::new();
    response.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        bail!("Web 下载文件过大");
    }
    Ok(data)
}

#[derive(serde::Deserialize)]
struct ShijuManifest {
    schema_version: u64,
    version: String,
    published_at: String,
    release_url: String,
}

fn release_urls(source: UpdateSource, payload: &[u8]) -> Result<(String, String, String)> {
    match source {
        UpdateSource::Github => {
            let release: Release = serde_json::from_slice(payload)?;
            let name = archive_name(&release.tag_name)?;
            if release.draft || release.prerelease {
                bail!("Web 安装仅支持正式发布版本");
            }
            let zip = asset_url(&release, &name)?.to_owned();
            let checksum = asset_url(&release, &format!("{name}.sha256"))?.to_owned();
            Ok((name, zip, checksum))
        }
        UpdateSource::Shiju => {
            // 与桌面更新共用 latest/manifest.json，Web 包使用相同版本目录与 ZIP 格式。
            let manifest: ShijuManifest = serde_json::from_slice(payload)?;
            let tag = format!("v{}", manifest.version);
            let name = archive_name(&tag)?;
            if manifest.schema_version != 1
                || manifest.published_at.trim().is_empty()
                || manifest.release_url
                    != format!("https://github.com/hunzhiwange/shiju/releases/tag/{tag}")
            {
                bail!("ShiJu Web 发布清单格式无效");
            }
            let zip = format!("{SHIJU_ROOT}/releases/{tag}/{name}");
            let checksum = format!("{zip}.sha256");
            Ok((name, zip, checksum))
        }
    }
}

pub(super) fn install(source: UpdateSource) -> Result<PathBuf> {
    let client = client()?;
    let endpoint = match source {
        UpdateSource::Shiju => SHIJU_MANIFEST,
        UpdateSource::Github => RELEASE_API,
    };
    let payload = download(&client, endpoint, 4 * 1024 * 1024)?;
    let (name, zip_url, checksum_url) = release_urls(source, &payload)?;
    let checksum = download(&client, &checksum_url, 1024)?;
    let archive = download(&client, &zip_url, MAX_ARCHIVE_BYTES)?;
    verify_checksum(&archive, &checksum, &name)?;

    let root = root()?;
    fs::create_dir_all(&root)?;
    let stage = tempfile::tempdir_in(&root)?;
    let package = extract(&archive, stage.path(), name.trim_end_matches(".zip"))?;
    validate_package(&package)?;
    let target = root.join("current");
    let backup = root.join("previous");
    if backup.exists() {
        fs::remove_dir_all(&backup).context("清理上次 Web 安装备份失败")?;
    }
    if target.exists() {
        fs::rename(&target, &backup).context("备份 Web 安装失败")?;
    }
    if let Err(error) = fs::rename(&package, &target) {
        if backup.exists() {
            fs::rename(&backup, &target).context("Web 安装失败，恢复旧版本失败")?;
        }
        return Err(error).context("切换 Web 安装失败");
    }
    Ok(target)
}

fn archive_name(tag: &str) -> Result<String> {
    let version = tag.strip_prefix('v').context("Web 发布标签必须为 vX.Y.Z")?;
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        bail!("Web 发布标签必须为 vX.Y.Z");
    }
    Ok(format!("shiju-web-{version}.zip"))
}

fn asset_url<'a>(release: &'a Release, name: &str) -> Result<&'a str> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .with_context(|| format!("GitHub 最新版本尚未发布 {name}，请在发布后重试"))?;
    let expected = format!("{DOWNLOAD_ROOT}{}/{name}", release.tag_name);
    if asset.browser_download_url != expected {
        bail!("Web 发布包下载地址不符合 ShiJu GitHub 发布路径");
    }
    Ok(&asset.browser_download_url)
}

fn verify_checksum(archive: &[u8], checksum: &[u8], name: &str) -> Result<()> {
    let text = std::str::from_utf8(checksum)?;
    let parts: Vec<_> = text.split_whitespace().collect();
    if parts.len() != 2
        || parts[1] != name
        || parts[0].len() != 64
        || !parts[0].bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("Web 发布包 SHA-256 文件格式无效");
    }
    let actual = hex::encode(Sha256::digest(archive));
    if !actual.eq_ignore_ascii_case(parts[0]) {
        bail!("Web 发布包 SHA-256 校验失败");
    }
    Ok(())
}

fn extract(bytes: &[u8], destination: &Path, stem: &str) -> Result<PathBuf> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    if archive.len() > 4096 {
        bail!("Web 发布包文件数量过多");
    }
    let mut remaining = MAX_EXTRACTED_BYTES;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let path = entry.enclosed_name().context("Web 发布包包含不安全路径")?;
        // 拒绝软链接、Windows 路径及其它平台特殊文件。
        if entry.name().contains('\\')
            || path.components().any(|c| !matches!(c, Component::Normal(_)))
            || entry.unix_mode().is_some_and(|mode| {
                let kind = mode & 0o170000;
                kind != 0 && kind != 0o100000 && kind != 0o040000
            })
            || path.components().next().map(|c| c.as_os_str()) != Some(std::ffi::OsStr::new(stem))
            || path.to_string_lossy().contains(':')
        {
            bail!("Web 发布包包含不支持的文件或路径");
        }
        let target = destination.join(path);
        if entry.is_dir() {
            fs::create_dir_all(target)?;
            continue;
        }
        if entry.size() > remaining {
            bail!("Web 发布包解压大小超限");
        }
        fs::create_dir_all(target.parent().context("Web 文件缺少父目录")?)?;
        let mut output = fs::OpenOptions::new().write(true).create_new(true).open(target)?;
        let copied = std::io::copy(&mut entry.by_ref().take(remaining + 1), &mut output)?;
        if copied > remaining {
            bail!("Web 发布包解压大小超限");
        }
        remaining -= copied;
        output.flush()?;
    }
    Ok(destination.join(stem))
}

pub(super) fn validate_package(path: &Path) -> Result<()> {
    for name in ["index.html", "shi_ju.js", "shi_ju_bg.wasm", "serve.mjs", "package.json"] {
        let metadata = fs::symlink_metadata(path.join(name))
            .with_context(|| format!("Web 安装不完整，缺少 {name}，请重新安装"))?;
        if !metadata.is_file() || metadata.len() == 0 {
            bail!("Web 安装文件无效: {name}，请重新安装");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "installer_tests.rs"]
mod tests;
