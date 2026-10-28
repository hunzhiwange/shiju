//! Stage the complete signed bundle on the destination volume and exchange it atomically.

use super::{UpdateRequest, command_output};
use anyhow::{Context, Result, bail};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn bundle(executable: &Path) -> Result<PathBuf> {
    let macos = executable.parent().context("应用目录缺失")?;
    let contents = macos.parent().context("应用 Contents 目录缺失")?;
    let app = contents.parent().context("应用包目录缺失")?;
    if macos.file_name().is_none_or(|name| name != "MacOS")
        || contents.file_name().is_none_or(|name| name != "Contents")
        || app.extension().is_none_or(|ext| ext != "app")
        || app.canonicalize()? != app
        || app.starts_with("/Volumes")
    {
        bail!("请先将 ShiJu 安装到本地应用目录，再执行自动更新");
    }
    Ok(app.to_owned())
}

fn plist_value(app: &Path, key: &str) -> Result<String> {
    command_output(
        Command::new("/usr/libexec/PlistBuddy")
            .args(["-c", &format!("Print :{key}")])
            .arg(app.join("Contents/Info.plist")),
    )
}

struct MountedImage(tempfile::TempDir);

impl Drop for MountedImage {
    fn drop(&mut self) {
        let _ =
            Command::new("/usr/bin/hdiutil").args(["detach", "-quiet"]).arg(self.0.path()).status();
    }
}

pub(super) fn prepare(dmg: &Path, executable: &Path, version: &str) -> Result<tempfile::TempDir> {
    let target = bundle(executable)?;
    let parent = target.parent().context("应用安装目录缺失")?;
    let stage = tempfile::Builder::new()
        .prefix(".shiju-update-")
        .tempdir_in(parent)
        .context("应用目录不可写，无法自动更新；请调整应用目录权限后重试")?;
    let mount = MountedImage(
        tempfile::Builder::new()
            .prefix("mount-")
            .tempdir_in(dmg.parent().context("安装包目录缺失")?)?,
    );
    command_output(
        Command::new("/usr/bin/hdiutil")
            .args(["attach", "-readonly", "-nobrowse", "-noautoopen", "-quiet", "-mountpoint"])
            .arg(mount.0.path())
            .arg(dmg),
    )?;
    let apps: Vec<_> = fs::read_dir(mount.0.path())?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "app"))
        .collect();
    if apps.len() != 1 {
        bail!("DMG 必须包含唯一的应用包");
    }
    let source = &apps[0];
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        bail!("DMG 应用包不能是符号链接");
    }
    let staged = stage.path().join("ShiJu.app");
    command_output(Command::new("/usr/bin/ditto").arg(source).arg(&staged))?;
    validate_bundle(&staged, &target, executable)?;
    validate_version(&staged, version)?;
    Ok(stage)
}

fn validate_version(app: &Path, version: &str) -> Result<()> {
    if plist_value(app, "CFBundleShortVersionString")? != version {
        bail!("应用包版本与已校验的发布版本不一致");
    }
    Ok(())
}

fn validate_bundle(staged: &Path, target: &Path, executable: &Path) -> Result<()> {
    if staged.canonicalize()? != staged || !staged.is_dir() {
        bail!("待更新应用包路径无效");
    }
    let identifier = plist_value(target, "CFBundleIdentifier")?;
    if identifier != "com.rsung.shiju" || plist_value(staged, "CFBundleIdentifier")? != identifier {
        bail!("更新包应用标识与当前 ShiJu 不匹配");
    }
    let name = plist_value(staged, "CFBundleExecutable")?;
    if Some(std::ffi::OsStr::new(&name)) != executable.file_name()
        || !staged.join("Contents/MacOS").join(&name).is_file()
    {
        bail!("更新包可执行文件与当前应用不匹配");
    }
    command_output(
        Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(staged),
    )?;
    Ok(())
}

fn paths(request: &UpdateRequest) -> Result<(PathBuf, PathBuf)> {
    let target = bundle(&request.executable)?;
    let stage = request.macos_stage.as_ref().context("更新暂存目录缺失")?;
    if stage.parent() != target.parent()
        || !stage
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".shiju-update-"))
        || stage.canonicalize()? != *stage
    {
        bail!("更新暂存目录必须在应用的同一目录与文件系统");
    }
    Ok((stage.join("ShiJu.app"), target))
}

pub(super) fn install(request: &UpdateRequest) -> Result<()> {
    let (staged, target) = paths(request)?;
    validate_bundle(&staged, &target, &request.executable)?;
    validate_version(&staged, request.ready.version())?;
    swap_bundles(&staged, &target)
}

pub(super) fn swap_bundles(staged: &Path, target: &Path) -> Result<()> {
    let source = CString::new(staged.as_os_str().as_bytes())?;
    let destination = CString::new(target.as_os_str().as_bytes())?;
    // Both directories must already exist. No delete/rename fallback: a failed swap
    // leaves the old app in place and preserves the staged app for diagnosis.
    if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_SWAP) } != 0 {
        return Err(std::io::Error::last_os_error())
            .context("完整应用包原子交换失败，旧版保持原位");
    }
    Ok(())
}

pub(super) fn rollback(request: &UpdateRequest) -> Result<()> {
    let (staged, target) = paths(request)?;
    swap_bundles(&staged, &target).context("恢复旧应用失败，备份已保留在更新暂存目录")
}

pub(super) fn cleanup(request: &UpdateRequest) -> Result<()> {
    let (staged, _) = paths(request)?;
    fs::remove_dir_all(staged.parent().context("更新目录缺失")?)?;
    Ok(())
}

pub(super) fn launch(executable: &Path) -> Result<()> {
    command_output(Command::new("/usr/bin/open").arg("-n").arg(bundle(executable)?))?;
    Ok(())
}
