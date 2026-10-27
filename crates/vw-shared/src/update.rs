//! VibeWindow 自更新功能。
//!
//! 负责从发布源获取最新版本并执行本地替换安装。

use anyhow::{Context, Result, bail};
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

const DEFAULT_RELEASE_API: &str = "https://api.github.com/repos/hunzhiwange/shiju/releases/latest";
const APP_UPDATE_API_ENV: &str = "VIBEWINDOW_APP_UPDATE_API";

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct ReleaseManifest {
    version: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct ReleaseAsset {
    name: String,
    download_url: String,
    binary_name: Option<String>,
    target: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateProgress {
    pub asset_name: String,
    pub download_url: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: f64,
    pub average_bytes_per_second: f64,
}

#[derive(Debug, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct CustomRelease {
    version: String,
    #[serde(default)]
    assets: Vec<CustomAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct CustomAsset {
    #[serde(default)]
    name: Option<String>,
    url: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    binary_name: Option<String>,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 获取远端发布源声明的最新版本号。
pub async fn fetch_latest_version() -> Result<String> {
    Ok(fetch_release_manifest().await?.version)
}

/// 执行桌面端应用内自更新。
pub async fn desktop_self_update() -> Result<String> {
    desktop_self_update_with_progress(|_| {}).await
}

/// 执行桌面端应用内自更新，并在下载阶段报告进度。
pub async fn desktop_self_update_with_progress<F>(on_progress: F) -> Result<String>
where
    F: FnMut(UpdateProgress) + Send,
{
    #[cfg(windows)]
    {
        let _ = on_progress;
        bail!("Windows 暂未支持应用内自更新，请下载新版本后手动覆盖安装。");
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = on_progress;
        bail!("WebAssembly 暂未支持应用内自更新。");
    }

    #[cfg(all(not(windows), not(target_arch = "wasm32")))]
    {
        let mut on_progress = on_progress;
        let release = fetch_release_manifest().await?;
        if normalize_version(&release.version) == normalize_version(current_version()) {
            return Ok(format!("当前已是最新版本 {}", release.version));
        }

        let current_exe = get_current_exe()?;
        let asset = find_asset_for_platform(&release)?;
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let preferred_binary_names =
            candidate_binary_names(&current_exe, asset.binary_name.as_deref());
        let new_binary =
            download_binary_with_progress(asset, temp_dir.path(), &preferred_binary_names, |p| {
                on_progress(p);
            })
            .await?;
        replace_binary(&new_binary, &current_exe)?;
        Ok(format!("已更新到 {}，重启后即可使用新版本。", release.version))
    }
}

fn current_release_api() -> String {
    env::var(APP_UPDATE_API_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASE_API.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_target_triple() -> Result<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "arm") => "armv7-unknown-linux-gnueabihf",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => bail!("Unsupported platform: {}-{}", os, arch),
    };
    Ok(target.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_binary_name() -> String {
    if cfg!(windows) { "shiju.exe".to_string() } else { "shiju".to_string() }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_archive_name(target: &str) -> String {
    if target.contains("windows") {
        format!("vibewindow-{}.zip", target)
    } else {
        format!("vibewindow-{}.tar.gz", target)
    }
}

async fn fetch_release_manifest() -> Result<ReleaseManifest> {
    let client = reqwest::Client::builder()
        .user_agent(format!("vibewindow/{}", current_version()))
        .build()
        .context("Failed to create HTTP client")?;
    let endpoint = current_release_api();
    let response = client
        .get(&endpoint)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("Failed to fetch release information from {}", endpoint))?;

    if !response.status().is_success() {
        bail!("Update API returned status: {}", response.status());
    }

    let payload =
        response.json::<serde_json::Value>().await.context("Failed to parse update payload")?;
    parse_release_manifest(payload)
}

fn parse_release_manifest(payload: serde_json::Value) -> Result<ReleaseManifest> {
    if payload.get("tag_name").is_some() {
        let release: GithubRelease =
            serde_json::from_value(payload).context("Failed to parse GitHub release payload")?;
        return Ok(ReleaseManifest {
            version: release.tag_name,
            assets: release
                .assets
                .into_iter()
                .map(|asset| ReleaseAsset {
                    name: asset.name,
                    download_url: asset.browser_download_url,
                    binary_name: None,
                    target: None,
                })
                .collect(),
        });
    }

    if payload.get("version").is_some() {
        let release: CustomRelease =
            serde_json::from_value(payload).context("Failed to parse custom update payload")?;
        return Ok(ReleaseManifest {
            version: release.version,
            assets: release
                .assets
                .into_iter()
                .map(|asset| {
                    let inferred_name = asset
                        .url
                        .rsplit('/')
                        .next()
                        .map(str::to_string)
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| "vibewindow-update.bin".to_string());
                    ReleaseAsset {
                        name: asset.name.unwrap_or(inferred_name),
                        download_url: asset.url,
                        binary_name: asset.binary_name,
                        target: asset.target,
                    }
                })
                .collect(),
        });
    }

    bail!("Unsupported update payload: expected `tag_name` or `version` field")
}

#[cfg(not(target_arch = "wasm32"))]
fn find_asset_for_platform(release: &ReleaseManifest) -> Result<&ReleaseAsset> {
    let target = get_target_triple()?;
    let archive_name = get_archive_name(&target);

    release
        .assets
        .iter()
        .find(|asset| asset.target.as_deref() == Some(target.as_str()))
        .or_else(|| release.assets.iter().find(|asset| asset.name == archive_name))
        .or_else(|| release.assets.iter().find(|asset| asset.name.contains(&target)))
        .or_else(|| {
            release.assets.iter().find(|asset| {
                asset.download_url.contains(&archive_name) || asset.download_url.contains(&target)
            })
        })
        .with_context(|| {
            format!("No release asset found for platform {} (looking for {})", target, archive_name)
        })
}

#[cfg(not(target_arch = "wasm32"))]
async fn download_binary(
    asset: &ReleaseAsset,
    temp_dir: &Path,
    preferred_binary_names: &[String],
) -> Result<PathBuf> {
    download_binary_with_progress(asset, temp_dir, preferred_binary_names, |_| {}).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn download_binary_with_progress<F>(
    asset: &ReleaseAsset,
    temp_dir: &Path,
    preferred_binary_names: &[String],
    mut on_progress: F,
) -> Result<PathBuf>
where
    F: FnMut(UpdateProgress),
{
    let extension = asset.name.rsplit('.').next().unwrap_or_default();
    if ["dmg", "rpm", "msi", "pkg", "deb"].iter().any(|kind| extension.eq_ignore_ascii_case(kind)) {
        bail!(
            "Installer packages require the desktop installer handoff; refusing binary replacement"
        );
    }

    let client = reqwest::Client::builder()
        .user_agent(format!("vibewindow/{}", current_version()))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&asset.download_url)
        .send()
        .await
        .with_context(|| format!("Failed to download release asset {}", asset.download_url))?;

    if !response.status().is_success() {
        bail!("Download failed with status: {}", response.status());
    }

    let archive_path = temp_dir.join(&asset.name);
    let total_bytes = response.content_length();
    let mut archive_file =
        fs::File::create(&archive_path).context("Failed to create archive temp file")?;
    let mut response = response;
    let start = Instant::now();
    let mut last_tick = start;
    let mut downloaded_bytes = 0_u64;
    on_progress(UpdateProgress {
        asset_name: asset.name.clone(),
        download_url: asset.download_url.clone(),
        downloaded_bytes,
        total_bytes,
        bytes_per_second: 0.0,
        average_bytes_per_second: 0.0,
    });

    while let Some(chunk) = response.chunk().await.context("Failed to read download content")? {
        archive_file.write_all(&chunk).context("Failed to write archive to temp file")?;
        downloaded_bytes = downloaded_bytes.saturating_add(chunk.len() as u64);
        let now = Instant::now();
        let elapsed = now.duration_since(start).as_secs_f64();
        let tick_elapsed = now.duration_since(last_tick).as_secs_f64();
        let bytes_per_second =
            if tick_elapsed > 0.0 { chunk.len() as f64 / tick_elapsed } else { 0.0 };
        let average_bytes_per_second =
            if elapsed > 0.0 { downloaded_bytes as f64 / elapsed } else { 0.0 };
        on_progress(UpdateProgress {
            asset_name: asset.name.clone(),
            download_url: asset.download_url.clone(),
            downloaded_bytes,
            total_bytes,
            bytes_per_second,
            average_bytes_per_second,
        });
        last_tick = now;
    }
    archive_file.flush().context("Failed to flush archive temp file")?;

    let binary_path = if asset.name.ends_with(".tar.gz") {
        extract_tar_gz(&archive_path, temp_dir)?;
        find_extracted_binary(temp_dir, preferred_binary_names)?
    } else if asset.name.ends_with(".zip") {
        extract_zip(&archive_path, temp_dir)?;
        find_extracted_binary(temp_dir, preferred_binary_names)?
    } else {
        let direct_binary_name = asset
            .binary_name
            .clone()
            .or_else(|| preferred_binary_names.first().cloned())
            .unwrap_or_else(get_binary_name);
        let direct_binary_path = temp_dir.join(direct_binary_name);
        if archive_path != direct_binary_path {
            fs::rename(&archive_path, &direct_binary_path)
                .context("Failed to stage downloaded binary")?;
        }
        direct_binary_path
    };

    ensure_executable(&binary_path)?;
    Ok(binary_path)
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(dest_dir)
        .output()
        .context("Failed to execute tar command")?;

    if !output.status.success() {
        bail!("tar extraction failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let output = Command::new("unzip")
        .arg("-o")
        .arg(archive_path)
        .arg("-d")
        .arg(dest_dir)
        .output()
        .context("Failed to execute unzip command")?;

    if !output.status.success() {
        bail!("unzip extraction failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn find_extracted_binary(root: &Path, preferred_binary_names: &[String]) -> Result<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut first_file = None;

    while let Some(dir) = stack.pop() {
        for entry in
            fs::read_dir(&dir).with_context(|| format!("Failed to read {}", dir.display()))?
        {
            let entry = entry.with_context(|| format!("Failed to inspect {}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if first_file.is_none() {
                first_file = Some(path.clone());
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if preferred_binary_names.iter().any(|name| name == file_name) {
                return Ok(path);
            }
        }
    }

    first_file.context("Binary not found in downloaded archive")
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_executable(_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(_path, fs::Permissions::from_mode(0o755)).with_context(|| {
            format!("Failed to set executable permissions for {}", _path.display())
        })?;
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn get_current_exe() -> Result<PathBuf> {
    env::current_exe().context("Failed to get current executable path")
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
fn candidate_binary_names(current_exe: &Path, explicit_binary_name: Option<&str>) -> Vec<String> {
    let mut names = Vec::new();
    push_unique(
        &mut names,
        explicit_binary_name.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string),
    );
    push_unique(
        &mut names,
        current_exe.file_name().and_then(|value| value.to_str()).map(str::to_string),
    );
    push_unique(&mut names, Some(get_binary_name()));
    push_unique(&mut names, Some("vw-webview".to_string()));
    names
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
fn push_unique(values: &mut Vec<String>, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(windows)]
fn replace_binary(_new_binary: &Path, _current_exe: &Path) -> Result<()> {
    bail!("Windows 暂未支持应用内自更新，请下载新版本后手动覆盖安装。");
}

#[cfg(unix)]
fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    let parent = current_exe.parent().context("Failed to resolve current executable directory")?;
    let file_name = current_exe
        .file_name()
        .and_then(|value| value.to_str())
        .context("Failed to resolve current executable name")?;
    let staged_path = parent.join(format!(".{file_name}.update"));
    let backup_path = parent.join(format!(".{file_name}.backup"));

    if staged_path.exists() {
        let _ = fs::remove_file(&staged_path);
    }
    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    fs::copy(new_binary, &staged_path).with_context(|| {
        format!(
            "Failed to copy updated binary {} -> {}",
            new_binary.display(),
            staged_path.display()
        )
    })?;
    ensure_executable(&staged_path)?;
    fs::rename(current_exe, &backup_path).with_context(|| {
        format!(
            "Failed to move current binary {} -> {}",
            current_exe.display(),
            backup_path.display()
        )
    })?;

    if let Err(error) = fs::rename(&staged_path, current_exe) {
        let _ = fs::rename(&backup_path, current_exe);
        let _ = fs::remove_file(&staged_path);
        return Err(error).with_context(|| {
            format!(
                "Failed to place updated binary {} -> {}",
                staged_path.display(),
                current_exe.display()
            )
        });
    }

    let _ = fs::remove_file(&backup_path);
    Ok(())
}

fn normalize_version(value: &str) -> &str {
    value.trim().trim_start_matches('v')
}

/// 检查当前版本是否存在可更新版本。
pub async fn check_for_update() -> Result<Option<String>> {
    let release = fetch_release_manifest().await?;
    let latest_version = normalize_version(&release.version);

    if latest_version == normalize_version(current_version()) {
        Ok(None)
    } else {
        Ok(Some(format!("{} (current: {})", release.version, current_version())))
    }
}

/// 为 CLI 场景执行检查或自更新流程。
pub async fn self_update(force: bool, check_only: bool) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (force, check_only);
        bail!("WebAssembly 暂未支持 CLI 自更新。");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("VibeWindow Self-Update");
        println!();

        let current_exe = get_current_exe()?;
        println!("Current binary: {}", current_exe.display());
        println!("Current version: v{}", current_version());
        println!();

        let release = fetch_release_manifest().await?;
        let latest_version = normalize_version(&release.version);

        println!("Latest version:  {}", release.version);

        println!();
        if latest_version == normalize_version(current_version()) && !force {
            println!("✅ Already up to date!");
            return Ok(());
        }

        if check_only {
            println!("Update available: {} -> {}", current_version(), latest_version);
            println!("Run `shiju update` to install the update.");
            return Ok(());
        }

        println!("Updating from v{} to {}...", current_version(), latest_version);

        let asset = find_asset_for_platform(&release)?;
        println!("Downloading: {}", asset.name);

        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let preferred_binary_names = vec![get_binary_name()];
        let new_binary = download_binary(asset, temp_dir.path(), &preferred_binary_names).await?;

        println!("Installing update...");
        replace_binary(&new_binary, &current_exe)?;

        println!();
        println!("✅ Successfully updated to {}!", release.version);
        println!();
        println!("Restart VibeWindow to use the new version.");

        Ok(())
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod desktop_release;
#[cfg(not(target_arch = "wasm32"))]
pub use desktop_release::{
    DesktopInstaller, DesktopRelease, InstallerKind, UpdateSource, download_desktop_installer,
    fetch_desktop_release,
};
