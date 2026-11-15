//! 归档提取、产物定位与原子替换。

use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use std::{env, fs, path::{Path, PathBuf}, process::Command};

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
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
pub(super) fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
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
pub(super) fn find_extracted_binary(root: &Path, preferred_binary_names: &[String]) -> Result<PathBuf> {
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
pub(super) fn ensure_executable(_path: &Path) -> Result<()> {
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
pub(super) fn get_current_exe() -> Result<PathBuf> {
    env::current_exe().context("Failed to get current executable path")
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
pub(super) fn candidate_binary_names(current_exe: &Path, explicit_binary_name: Option<&str>) -> Vec<String> {
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
pub(super) fn push_unique(values: &mut Vec<String>, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(windows)]
pub(super) fn replace_binary(_new_binary: &Path, _current_exe: &Path) -> Result<()> {
    bail!("Windows 暂未支持应用内自更新，请下载新版本后手动覆盖安装。");
}

#[cfg(unix)]
pub(super) fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
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

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn get_binary_name() -> String {
    if cfg!(windows) { "shiju.exe".to_string() } else { "shiju".to_string() }
}
