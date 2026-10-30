//! Verified downloads for automatic updates or manual Linux installation.
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use vw_shared::update::{DesktopInstaller, DesktopRelease, InstallerKind, UpdateProgress};

/// Only the verified download path can produce this result. Fields stay private so
/// callers cannot substitute an arbitrary path or digest before opening it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreparedDesktopUpdate {
    path: PathBuf,
    directory: PathBuf,
    asset: DesktopInstaller,
    version: String,
}

/// Only the private helper protocol restores a ready download, and always rechecks it.
pub(crate) fn deserialize_handoff<'de, D>(
    deserializer: D,
) -> Result<PreparedDesktopUpdate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    struct Handoff {
        path: PathBuf,
        directory: PathBuf,
        asset: DesktopInstaller,
        version: String,
    }
    let handoff = <Handoff as serde::Deserialize>::deserialize(deserializer)?;
    let ready = PreparedDesktopUpdate {
        path: handoff.path,
        directory: handoff.directory,
        asset: handoff.asset,
        version: handoff.version,
    };
    verify_ready(&ready).map_err(serde::de::Error::custom)?;
    Ok(ready)
}

impl PreparedDesktopUpdate {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> InstallerKind {
        self.asset.kind
    }

    pub fn installs_automatically(&self) -> bool {
        matches!(self.kind(), InstallerKind::Dmg | InstallerKind::Msi)
    }

    pub fn instructions(&self) -> String {
        format!("安装包已就绪：{}。{}", self.path.display(), instructions(self.kind()))
    }
}

/// Downloads the selected snapshot without launching a process. Successful files
/// persist across application exit; failed attempts remove only their own files.
pub async fn prepare_desktop_update(
    release: &DesktopRelease,
    progress_tx: Sender<UpdateProgress>,
) -> Result<PreparedDesktopUpdate> {
    let target = desktop_target(std::env::consts::OS, std::env::consts::ARCH)?;
    let asset = release.installer_for_target(target)?;
    // Installer downloads use the ShiJu identity independently of legacy configuration paths.
    #[cfg(debug_assertions)]
    const APP_NAME: &str = "shiju-dev";
    #[cfg(not(debug_assertions))]
    const APP_NAME: &str = "shiju";
    let dirs = directories::ProjectDirs::from("com", "rsung", APP_NAME)
        .context("无法确定应用缓存目录")?;
    fs::create_dir_all(dirs.cache_dir())?;
    let attempt = create_attempt(dirs.cache_dir(), &release.version)?;
    let path = vw_shared::update::download_desktop_installer(
        release,
        target,
        attempt.path(),
        |progress| {
            let _ = progress_tx.send(progress);
        },
    )
    .await?;
    finish_download(attempt, path, asset, &release.version)
}

/// Revalidate the download, then reveal Linux downloads or start the automatic updater.
pub fn open_desktop_installer(ready: &PreparedDesktopUpdate) -> Result<String> {
    open_with(ready, std::env::consts::OS, |path| {
        if ready.installs_automatically() {
            crate::desktop_update_helper::start(ready)
        } else {
            open::that(path).context("无法打开安装包下载目录")
        }
    })
}

fn desktop_target(os: &str, arch: &str) -> Result<&'static str> {
    match (os, arch) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("windows", "aarch64") => Ok("aarch64-pc-windows-msvc"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        _ => bail!("不支持的桌面安装平台：{os}-{arch}"),
    }
}

fn extension(kind: InstallerKind) -> &'static str {
    match kind {
        InstallerKind::Dmg => "dmg",
        InstallerKind::Msi => "msi",
        InstallerKind::Rpm => "rpm",
    }
}

fn instructions(kind: InstallerKind) -> &'static str {
    match kind {
        InstallerKind::Dmg => {
            "点击自动更新后，应用将退出并自动替换完整应用包，无需拖拽；成功后确认启动新版。"
        }
        InstallerKind::Msi => {
            "点击自动更新后，应用将退出并通过 Windows Installer 更新；请允许系统授权，成功后确认启动新版。"
        }
        InstallerKind::Rpm => {
            "点击“打开下载位置”后，请自行退出应用并手动安装，安装完成后重新启动。此 RPM 适用于 RPM 系统；Ubuntu/Debian 需要 .deb 包，不能直接安装此 RPM，请从 ShiJu GitHub Release 下载对应版本和架构的 DEB 包。"
        }
    }
}

fn create_attempt(cache: &Path, version: &str) -> Result<tempfile::TempDir> {
    // Validate even this private boundary before using a version as a path component.
    let version = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        bail!("无效安装包版本");
    }
    let mut directory = cache.canonicalize()?;
    for component in ["updates".to_owned(), format!("v{version}")] {
        directory.push(component);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = fs::symlink_metadata(&directory)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || directory.canonicalize()? != directory
        {
            bail!("安装包缓存目录不能是符号链接：{}", directory.display());
        }
    }
    Ok(tempfile::Builder::new().prefix("attempt-").tempdir_in(directory)?)
}

fn finish_download(
    attempt: tempfile::TempDir,
    path: PathBuf,
    asset: &DesktopInstaller,
    version: &str,
) -> Result<PreparedDesktopUpdate> {
    let ready = PreparedDesktopUpdate {
        path,
        directory: attempt.path().to_owned(),
        asset: asset.clone(),
        version: version.trim_start_matches('v').to_owned(),
    };
    verify_ready(&ready)?;
    // Retain successful downloads even if the subsequent system opener fails.
    let _ = attempt.keep();
    Ok(ready)
}

pub(crate) fn verify_ready(ready: &PreparedDesktopUpdate) -> Result<()> {
    if !ready.path.is_absolute()
        || ready.path != ready.directory.join(&ready.asset.name)
        || ready.path.extension().and_then(|s| s.to_str()) != Some(extension(ready.kind()))
        || ready.path.canonicalize()? != ready.path
    {
        bail!("安装包路径未经验证或已被重定向");
    }
    let metadata = fs::symlink_metadata(&ready.path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() != ready.asset.size
    {
        bail!("安装包不是已验证的普通文件");
    }
    let mut file = fs::File::open(&ready.path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if hex::encode(digest.finalize()) != ready.asset.sha256 {
        bail!("安装包 SHA-256 校验失败");
    }
    Ok(())
}

fn open_with(
    ready: &PreparedDesktopUpdate,
    os: &str,
    opener: impl FnOnce(&Path) -> Result<()>,
) -> Result<String> {
    verify_ready(ready)?;
    if !matches!(
        (os, ready.kind()),
        ("macos", InstallerKind::Dmg)
            | ("windows", InstallerKind::Msi)
            | ("linux", InstallerKind::Rpm)
    ) {
        bail!("此安装包格式不适用于当前系统。{}", ready.instructions());
    }
    if !ready.installs_automatically() {
        // Open the directory, never the RPM itself: package handlers may request installation.
        opener(&ready.directory)
            .map_err(|error| anyhow::anyhow!("无法打开下载位置：{error:#}"))?;
        return Ok(format!("下载位置已打开，请手动升级。{}", ready.instructions()));
    }
    // The UI also uses Display on this error; retain the underlying launch failure there.
    opener(ready.path()).map_err(|error| anyhow::anyhow!("无法启动自动更新：{error:#}"))?;
    Ok(format!(
        "自动更新已准备就绪，正在退出应用；更新成功后将提示启动新版。{}",
        ready.instructions()
    ))
}

#[cfg(test)]
#[path = "installer_update_tests.rs"]
mod installer_update_tests;
