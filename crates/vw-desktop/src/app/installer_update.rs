//! Persistent installer handoff; opening an installer does not complete an upgrade.
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use vw_shared::update::{DesktopInstaller, DesktopRelease, InstallerKind, UpdateProgress};

/// Only the verified download path can produce this result. Fields stay private so
/// callers cannot substitute an arbitrary path or digest before opening it.
#[derive(Debug, Clone)]
pub struct PreparedDesktopUpdate {
    path: PathBuf,
    directory: PathBuf,
    asset: DesktopInstaller,
}

impl PreparedDesktopUpdate {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> InstallerKind {
        self.asset.kind
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
    finish_download(attempt, path, asset)
}

/// The opaque ready result is required instead of a raw path/kind pair. Recheck
/// the file immediately before passing the path to the system's default opener.
/// Never elevates, installs silently, exits the app, or removes the installer.
pub fn open_desktop_installer(ready: &PreparedDesktopUpdate) -> Result<String> {
    open_with(ready, std::env::consts::OS, |path| open::that(path).context("系统安装器启动失败"))
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
        InstallerKind::Dmg => "请将应用拖入 Applications 并退出旧版，再启动新版以确认版本。",
        InstallerKind::Msi => "请完成系统 MSI 交互安装，再启动新版以确认版本。",
        InstallerKind::Rpm => {
            "此 RPM 仅适用于兼容 RPM 的 Linux 系统；请使用系统软件安装器打开上述文件，或在兼容系统中手动安装该 RPM，再启动新版以确认版本。"
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
) -> Result<PreparedDesktopUpdate> {
    let ready =
        PreparedDesktopUpdate { path, directory: attempt.path().to_owned(), asset: asset.clone() };
    verify_ready(&ready)?;
    // Retain successful downloads even if the subsequent system opener fails.
    let _ = attempt.keep();
    Ok(ready)
}

fn verify_ready(ready: &PreparedDesktopUpdate) -> Result<()> {
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
    opener(ready.path()).with_context(|| format!("无法打开安装器。{}", ready.instructions()))?;
    Ok(format!("已打开安装器，请完成系统安装。{}", ready.instructions()))
}

#[cfg(test)]
#[path = "installer_update_tests.rs"]
mod installer_update_tests;
