//! CLI installation and installed-version detection.
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io::Write, path::{Path, PathBuf}, process::Command};
#[cfg(not(target_arch = "wasm32"))]
pub fn install_cli_tool() -> Result<String, String> {
    let home = resolve_home_dir()?;
    let install_dir = config_runtime_paths::home_config_dir(&home).join("bin");
    fs::create_dir_all(&install_dir).map_err(|e| format!("创建安装目录失败: {e}"))?;

    let target_name = if cfg!(windows) { "shiju.exe" } else { "shiju" };
    let target_bin = install_dir.join(target_name);
    let replacing_existing = target_bin.exists();

    let source_bin_res = resolve_agent_binary_path();
    if let Ok(source_bin) = source_bin_res {
        install_cli_binary_from_path(&source_bin, &target_bin)?;
    } else if let Ok(url) = std::env::var("VIBEWINDOW_CLI_URL") {
        install_cli_binary_from_url(&url, &target_bin)?;
    } else {
        return Err(format!(
            "未在应用包中发现 CLI，也未设置 VIBEWINDOW_CLI_URL 可供下载。\n请下载独立 CLI 包并手动安装：\n1) 解压后将可执行文件复制到 {}\n2) 重命名为 shiju 或 shiju.exe\n3) 确保 PATH 包含 {} 后重启终端",
            config_runtime_paths::tilde_config_path("bin"),
            config_runtime_paths::tilde_config_path("bin")
        ));
    }

    let updated_profiles = ensure_path_profiles(&home, &install_dir)?;
    let mut message =
        format!("已安装到 {}\n可执行文件: {}", install_dir.display(), target_bin.display());
    if replacing_existing {
        message.push_str(
            "\n已安全替换旧 CLI；如果后台 service 正在运行，请重启 service 以加载新版本。",
        );
    }
    if updated_profiles.is_empty() {
        message.push_str("\nPATH 配置已存在，无需重复写入。");
    } else {
        message.push_str("\n已更新 PATH 配置文件：");
        for profile in updated_profiles {
            message.push_str(&format!("\n- {profile}"));
        }
    }
    Ok(message)
}

#[cfg(target_arch = "wasm32")]
pub fn install_cli_tool() -> Result<String, String> {
    Err("当前平台不支持安装 CLI 工具".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_binary_from_path(source_bin: &Path, target_bin: &Path) -> Result<(), String> {
    let staged_bin = staged_cli_binary_path(target_bin);
    remove_stale_staged_cli_binary(&staged_bin)?;
    fs::copy(source_bin, &staged_bin).map_err(|e| {
        let _ = fs::remove_file(&staged_bin);
        format!(
            "复制 CLI 可执行文件失败: {} -> {}: {e}",
            source_bin.display(),
            staged_bin.display()
        )
    })?;
    install_staged_cli_binary(&staged_bin, target_bin)
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_binary_from_url(url: &str, target_bin: &Path) -> Result<(), String> {
    let staged_bin = staged_cli_binary_path(target_bin);
    remove_stale_staged_cli_binary(&staged_bin)?;
    if let Err(error) = download_file(url, &staged_bin) {
        let _ = fs::remove_file(&staged_bin);
        return Err(error);
    }
    install_staged_cli_binary(&staged_bin, target_bin)
}

#[cfg(not(target_arch = "wasm32"))]
fn install_staged_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    ensure_cli_binary_executable(staged_bin)?;
    replace_cli_binary(staged_bin, target_bin).map_err(|error| {
        let _ = fs::remove_file(staged_bin);
        error
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn staged_cli_binary_path(target_bin: &Path) -> PathBuf {
    let file_name = target_bin.file_name().and_then(|value| value.to_str()).unwrap_or("shiju");
    target_bin.with_file_name(format!(".{file_name}.installing"))
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_stale_staged_cli_binary(staged_bin: &Path) -> Result<(), String> {
    if staged_bin.exists() {
        fs::remove_file(staged_bin)
            .map_err(|e| format!("清理旧临时 CLI 文件失败 {}: {e}", staged_bin.display()))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_cli_binary_executable(_path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置执行权限失败: {e}"))?;
    }
    Ok(())
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn replace_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    fs::rename(staged_bin, target_bin).map_err(|e| {
        format!(
            "替换 CLI 可执行文件失败: {} -> {}: {e}",
            staged_bin.display(),
            target_bin.display()
        )
    })
}

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
fn replace_cli_binary(staged_bin: &Path, target_bin: &Path) -> Result<(), String> {
    let backup_bin = target_bin.with_extension("backup");
    if backup_bin.exists() {
        let _ = fs::remove_file(&backup_bin);
    }
    if target_bin.exists() {
        fs::rename(target_bin, &backup_bin).map_err(|e| {
            format!(
                "备份旧 CLI 可执行文件失败: {} -> {}: {e}",
                target_bin.display(),
                backup_bin.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(staged_bin, target_bin) {
        let _ = fs::rename(&backup_bin, target_bin);
        return Err(format!(
            "替换 CLI 可执行文件失败: {} -> {}: {error}",
            staged_bin.display(),
            target_bin.display()
        ));
    }
    let _ = fs::remove_file(&backup_bin);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn current_cli_tool_version_label() -> String {
    match read_installed_cli_version() {
        Ok(version) => version,
        Err(error) => error,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn current_cli_tool_version_label() -> String {
    "当前平台不支持".to_string()
}

pub fn versions_match(current: &str, latest: &str) -> bool {
    normalize_version(current) == normalize_version(latest)
}

fn normalize_version(value: &str) -> String {
    let line = value
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim())
        .unwrap_or_else(|| value.trim());
    line.split_whitespace().last().unwrap_or(line).trim().trim_start_matches('v').to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_installed_cli_version() -> Result<String, String> {
    let output = match Command::new("shiju").arg("--version").output() {
        Ok(output) => output,
        Err(_) => {
            let cli_path = resolve_cli_binary_path()?;
            Command::new(cli_path)
                .arg("--version")
                .output()
                .map_err(|e| format!("执行 shiju --version 失败: {e}"))?
        }
    };
    extract_version_output(output.stdout, output.stderr)
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_version_output(stdout: Vec<u8>, stderr: Vec<u8>) -> Result<String, String> {
    let combined = if !stdout.is_empty() {
        String::from_utf8_lossy(&stdout).into_owned()
    } else {
        String::from_utf8_lossy(&stderr).into_owned()
    };
    combined
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .ok_or_else(|| "CLI 未安装或未返回版本信息".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_cli_binary_path() -> Result<PathBuf, String> {
    let home = resolve_home_dir()?;
    let cli_path = config_runtime_paths::home_config_dir(home)
        .join("bin")
        .join(if cfg!(windows) { "shiju.exe" } else { "shiju" });
    if cli_path.exists() {
        Ok(cli_path)
    } else {
        Err("CLI 未安装，无法执行 shiju --version".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_home_dir() -> Result<PathBuf, String> {
    if let Some(user_dirs) = directories::UserDirs::new() {
        return Ok(user_dirs.home_dir().to_path_buf());
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    Err("无法定位用户主目录".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_agent_binary_path() -> Result<PathBuf, String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
    let exe_dir = current_exe.parent().ok_or_else(|| "无法定位当前可执行文件目录".to_string())?;
    let agent_names =
        if cfg!(windows) { ["shiju.exe", "vibe-agent.exe"] } else { ["shiju", "vibe-agent"] };
    let mut candidates = agent_names.iter().map(|name| exe_dir.join(name)).collect::<Vec<_>>();

    if let Ok(cwd) = std::env::current_dir() {
        for agent_name in agent_names {
            candidates.push(cwd.join("target").join("release").join(agent_name));
            candidates.push(cwd.join("target").join("debug").join(agent_name));
        }
    }

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("未找到内置 CLI 可执行文件 shiju（兼容名 vibe-agent）".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if cfg!(windows) {
        let status = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, dest.display()))
            .status()
            .map_err(|e| format!("启动下载命令失败: {e}"))?;
        if status.success() {
            return Ok(());
        }
        Err("下载失败，请手动下载并安装，或确保 PowerShell 可用".to_string())
    } else {
        let curl =
            Command::new("curl").arg("-fsSL").arg(url).arg("-o").arg(dest.as_os_str()).status();
        if let Ok(status) = curl
            && status.success()
        {
            return Ok(());
        }
        let wget = Command::new("wget").arg("-qO").arg(dest.as_os_str()).arg(url).status();
        if let Ok(status) = wget
            && status.success()
        {
            return Ok(());
        }
        Err("下载失败，请手动下载并安装，或确保已安装 curl/wget".to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_path_profiles(home: &Path, install_dir: &Path) -> Result<Vec<String>, String> {
    #[cfg(windows)]
    let profiles = vec![
        home.join("Documents").join("WindowsPowerShell").join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents").join("PowerShell").join("Microsoft.PowerShell_profile.ps1"),
    ];
    #[cfg(not(windows))]
    let profiles = vec![home.join(".zshrc"), home.join(".bashrc"), home.join(".profile")];

    let mut updated = Vec::new();
    for profile in profiles {
        if ensure_profile_contains_path(&profile, install_dir)? {
            updated.push(profile.display().to_string());
        }
    }
    Ok(updated)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_profile_contains_path(profile: &Path, install_dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    let export_line = format!(r#"$env:Path = "{};$env:Path""#, install_dir.display());
    #[cfg(not(windows))]
    let export_line = format!("export PATH={}:$PATH", install_dir.display());

    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置文件目录失败 {}: {e}", parent.display()))?;
    }

    let install_path_text = install_dir.to_string_lossy();
    let existing = fs::read_to_string(profile).unwrap_or_default();
    if existing.contains(export_line.as_str()) || existing.contains(install_path_text.as_ref()) {
        return Ok(false);
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile)
        .map_err(|e| format!("打开配置文件失败 {}: {e}", profile.display()))?;

    if !existing.ends_with('\n') && !existing.is_empty() {
        file.write_all(b"\n")
            .map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    }
    file.write_all(export_line.as_bytes())
        .map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    file.write_all(b"\n").map_err(|e| format!("写入配置文件失败 {}: {e}", profile.display()))?;
    Ok(true)
}

#[cfg(test)]
#[path = "cli_install_tests.rs"]
mod cli_install_tests;
