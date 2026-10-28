//! OpenRC 服务安装与运行目录准备。
//!
//! 本模块负责以 root 安装系统级 OpenRC 服务，创建受限的 `vibewindow` 系统用户，
//! 迁移现有运行状态，并确保配置、工作区与日志目录由服务用户拥有且可写。这里的
//! 权限检查直接影响守护进程的最小权限运行边界。

use crate::app::agent::config::Config;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::common::run_checked;

#[cfg(unix)]
/// 判断当前进程是否以 root 身份运行。
pub(super) fn is_root() -> bool {
    current_uid() == Some(0)
}

#[cfg(not(unix))]
/// 非 Unix 平台不支持 OpenRC root 检测，始终返回 `false`。
pub(super) fn is_root() -> bool {
    false
}

#[cfg(unix)]
/// 读取当前 Unix 用户 ID。
///
/// 返回 `None` 表示 `id -u` 不可用、执行失败或输出无法解析。
pub(super) fn current_uid() -> Option<u32> {
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().ok()
}

/// 校验现有 `vibewindow` 系统用户是否满足最小权限要求。
///
/// 如果用户存在但 UID、shell 或 home 不符合预期，会返回错误或警告。这里不自动修复
/// 异常用户，避免静默接管可能属于人工维护的账号。
fn check_vibewindow_user() -> Result<()> {
    let output = Command::new("getent").args(["passwd", "vibewindow"]).output();
    let is_alpine = Path::new("/etc/alpine-release").exists();
    let (del_cmd, add_cmd) = if is_alpine {
        (
            "deluser vibewindow && delgroup vibewindow",
            "addgroup -S vibewindow && adduser -S -s /sbin/nologin -H -D -G vibewindow vibewindow",
        )
    } else {
        ("userdel vibewindow", "useradd -r -s /sbin/nologin vibewindow")
    };

    match output {
        Ok(output) if output.status.success() => {
            let passwd_entry = String::from_utf8_lossy(&output.stdout);
            validate_vibewindow_user_entry(&passwd_entry, del_cmd, add_cmd)
        }
        _ => Ok(()),
    }
}

/// 校验一条 passwd 记录中的 OpenRC 服务用户安全属性。
fn validate_vibewindow_user_entry(passwd_entry: &str, del_cmd: &str, add_cmd: &str) -> Result<()> {
    let parts: Vec<&str> = passwd_entry.split(':').collect();
    if parts.len() >= 7 {
        let uid = parts[2];
        let gid = parts[3];
        let home = parts[5];
        let shell = parts[6];

        if uid.parse::<u32>().unwrap_or(999) >= 1000 {
            bail!(
                "User 'vibewindow' exists but has unexpected UID {} (expected system UID < 1000).\n\
                 Recreate with: sudo {} && sudo {}",
                uid,
                del_cmd,
                add_cmd
            );
        }

        if !shell.contains("nologin") && !shell.contains("false") {
            bail!(
                "User 'vibewindow' exists but has unexpected shell '{}'.\n\
                 Expected nologin/false for security. Fix with: sudo {} && sudo {}",
                shell,
                del_cmd,
                add_cmd
            );
        }

        if home != "/var/lib/vibewindow" && home != "/nonexistent" {
            eprintln!(
                "⚠️  Warning: vibewindow user has home directory '{}' (expected /var/lib/vibewindow or /nonexistent)",
                home
            );
        }

        let _ = gid;
    }
    Ok(())
}

/// 确保 `vibewindow` 系统用户存在。
///
/// Alpine 与常见 Linux 发行版使用不同的用户管理命令。本函数按平台创建系统用户，
/// 并在用户已存在时复用 `check_vibewindow_user` 做安全校验。
fn ensure_vibewindow_user() -> Result<()> {
    let output = Command::new("getent").args(["passwd", "vibewindow"]).output();
    if let Ok(output) = output {
        if output.status.success() {
            return check_vibewindow_user();
        }
    }

    let is_alpine = Path::new("/etc/alpine-release").exists();
    if is_alpine {
        let group_output = Command::new("getent").args(["group", "vibewindow"]).output();
        let group_exists = group_output.map(|o| o.status.success()).unwrap_or(false);

        if !group_exists {
            let output = Command::new("addgroup")
                .args(["-S", "vibewindow"])
                .output()
                .context("Failed to create vibewindow group")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to create vibewindow group: {}", stderr.trim());
            }
            println!("✅ Created system group: vibewindow");
        }

        let output = Command::new("adduser")
            .args(["-S", "-s", "/sbin/nologin", "-H", "-D", "-G", "vibewindow", "vibewindow"])
            .output()
            .context("Failed to create vibewindow user")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to create vibewindow user: {}", stderr.trim());
        }
    } else {
        let output = Command::new("useradd")
            .args(["-r", "-s", "/sbin/nologin", "vibewindow"])
            .output()
            .context("Failed to create vibewindow user")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to create vibewindow user: {}", stderr.trim());
        }
    }

    println!("✅ Created system user: vibewindow");
    Ok(())
}

#[cfg(unix)]
/// 将单个路径的所有者改为 `vibewindow:vibewindow`。
///
/// 返回错误表示 `chown` 无法执行或系统拒绝修改所有者。
fn chown_to_vibewindow(path: &Path) -> Result<()> {
    let output = Command::new("chown")
        .args(["vibewindow:vibewindow", &path.to_string_lossy()])
        .output()
        .context("Failed to run chown")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to change ownership of {} to vibewindow:vibewindow: {}",
            path.display(),
            stderr.trim(),
        );
    }
    Ok(())
}

#[cfg(not(unix))]
/// 非 Unix 平台的占位实现。
fn chown_to_vibewindow(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
/// 递归将路径所有者改为 `vibewindow:vibewindow`。
///
/// 仅用于安装时准备配置目录，确保服务用户能读取既有配置和状态文件。
fn chown_recursive_to_vibewindow(path: &Path) -> Result<()> {
    let output = Command::new("chown")
        .args(["-R", "vibewindow:vibewindow", &path.to_string_lossy()])
        .output()
        .context("Failed to run recursive chown")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to recursively change ownership of {} to vibewindow:vibewindow: {}",
            path.display(),
            stderr.trim(),
        );
    }

    Ok(())
}

#[cfg(not(unix))]
/// 非 Unix 平台的占位实现。
fn chown_recursive_to_vibewindow(_path: &Path) -> Result<()> {
    Ok(())
}

/// 递归复制目录内容，但不覆盖目标已有文件。
///
/// `source` 是来源目录，`target` 是目标目录。返回错误表示目录读取、目标创建或文件
/// 复制失败。跳过已有文件可以保留安装目录中已经由管理员调整过的配置。
fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("Failed to create directory {}", target.display()))?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("Failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("Failed to inspect {}", source_path.display()))?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            if target_path.exists() {
                continue;
            }
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "Failed to copy file {} -> {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// 解析触发 sudo 的原始用户配置目录。
///
/// 优先使用 `SUDO_USER` 的 home 目录，回退到当前 `HOME`。返回 `None` 表示无法可靠
/// 定位用户配置目录。
fn resolve_invoking_user_config_dir() -> Option<PathBuf> {
    let sudo_user = std::env::var("SUDO_USER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "root");

    if let Some(user) = sudo_user {
        if let Ok(output) = Command::new("getent").args(["passwd", &user]).output() {
            if output.status.success() {
                let entry = String::from_utf8_lossy(&output.stdout);
                let fields: Vec<&str> = entry.trim().split(':').collect();
                if fields.len() >= 6 {
                    return Some(vw_config_types::paths::home_config_dir(PathBuf::from(fields[5])));
                }
            }
        }
    }

    std::env::var("HOME").ok().map(PathBuf::from).map(vw_config_types::paths::home_config_dir)
}

/// 在首次安装 OpenRC 时迁移已有运行状态。
///
/// 如果目标配置已存在则直接复用；如果原用户目录存在 `vibewindow.json`，则复制整套
/// 配置目录到系统配置目录。复制不会覆盖目标已有文件。
fn migrate_openrc_runtime_state_if_needed(config_dir: &Path) -> Result<()> {
    migrate_openrc_runtime_state_with(config_dir, resolve_invoking_user_config_dir)
}

/// 使用惰性解析的来源目录迁移 OpenRC 运行状态。
fn migrate_openrc_runtime_state_with<F>(config_dir: &Path, resolve_source_dir: F) -> Result<()>
where
    F: FnOnce() -> Option<PathBuf>,
{
    let target_config = config_dir.join("vibewindow.json");
    if target_config.exists() {
        println!("✅ Reusing existing OpenRC config at {}", target_config.display());
        return Ok(());
    }

    let Some(source_dir) = resolve_source_dir() else {
        return Ok(());
    };

    let source_config = source_dir.join("vibewindow.json");
    if !source_config.exists() {
        return Ok(());
    }

    copy_dir_recursive(&source_dir, config_dir)?;
    println!("✅ Migrated runtime state from {} to {}", source_dir.display(), config_dir.display());
    Ok(())
}

#[cfg(unix)]
/// 将字符串转成 POSIX shell 单引号安全形式。
///
/// 返回值可安全嵌入 `sh -c` 命令，用于包含空格或单引号的路径。
pub(crate) fn shell_single_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\"'\"'"))
}

#[cfg(unix)]
/// 构造以 `vibewindow` 用户身份执行的目录可写性探测命令。
///
/// `path` 是待检查路径，`has_runuser` 指示系统是否可用 `runuser`。返回程序名与参数
/// 列表；调用方负责执行并检查退出状态。
pub(crate) fn build_openrc_writability_probe_command(
    path: &Path,
    has_runuser: bool,
) -> (String, Vec<String>) {
    let probe = format!("test -w {}", shell_single_quote(&path.to_string_lossy()));
    if has_runuser {
        (
            "runuser".to_string(),
            vec![
                "-u".to_string(),
                "vibewindow".to_string(),
                "--".to_string(),
                "sh".to_string(),
                "-c".to_string(),
                probe,
            ],
        )
    } else {
        (
            "su".to_string(),
            vec![
                "-s".to_string(),
                "/bin/sh".to_string(),
                "-c".to_string(),
                probe,
                "vibewindow".to_string(),
            ],
        )
    }
}

#[cfg(unix)]
/// 验证 OpenRC 运行用户是否能写入指定路径。
///
/// 该检查在安装阶段执行，避免服务安装成功但启动后因目录权限不足而失败。失败时会
/// 返回包含修复建议的错误。
fn ensure_openrc_runtime_path_writable(path: &Path) -> Result<()> {
    let has_runuser = which::which("runuser").is_ok();
    let (program, args) = build_openrc_writability_probe_command(path, has_runuser);
    let output =
        Command::new(&program).args(args.iter().map(String::as_str)).output().with_context(
            || format!("Failed to verify OpenRC runtime write access for {}", path.display()),
        )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let details =
            if stderr.trim().is_empty() { "write-access probe failed" } else { stderr.trim() };
        bail!(
            "OpenRC runtime user 'vibewindow' cannot write {} ({details}). \
             Re-run `sudo shiju service install` and ensure ownership is vibewindow:vibewindow.",
            path.display(),
        );
    }

    Ok(())
}

#[cfg(unix)]
/// 验证 OpenRC 服务所需运行目录均可写。
fn ensure_openrc_runtime_dirs_writable(
    config_dir: &Path,
    workspace_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    for path in [config_dir, workspace_dir, log_dir] {
        ensure_openrc_runtime_path_writable(path)?;
    }
    Ok(())
}

#[cfg(not(unix))]
/// 非 Unix 平台的占位实现。
fn ensure_openrc_runtime_dirs_writable(
    _config_dir: &Path,
    _workspace_dir: &Path,
    _log_dir: &Path,
) -> Result<()> {
    Ok(())
}

/// 当服务二进制位于用户 home 中时给出部署警告。
///
/// 系统级 OpenRC 服务通常应使用稳定的全局路径，避免用户目录权限、清理或移动导致
/// 服务无法启动。
fn warn_if_binary_in_home(exe_path: &Path) {
    if binary_path_appears_in_home(exe_path) {
        eprintln!(
            "⚠️  Warning: Binary path '{}' appears to be in a user home directory.\n\
             For system-wide OpenRC service, consider installing to /usr/local/bin:\n\
             sudo cp '{}' /usr/local/bin/vibewindow",
            exe_path.display(),
            exe_path.display()
        );
    }
}

/// 判断服务二进制路径是否位于不稳定的用户目录中。
fn binary_path_appears_in_home(exe_path: &Path) -> bool {
    let path_str = exe_path.to_string_lossy();
    path_str.contains("/home/") || path_str.contains(".cargo/bin")
}

#[cfg(test)]
pub(super) mod test_support {
    use anyhow::Result;
    use std::path::{Path, PathBuf};

    pub fn binary_path_appears_in_home(exe_path: &Path) -> bool {
        super::binary_path_appears_in_home(exe_path)
    }

    pub fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
        super::copy_dir_recursive(source, target)
    }

    pub fn migrate_openrc_runtime_state_with<F>(
        config_dir: &Path,
        resolve_source_dir: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Option<PathBuf>,
    {
        super::migrate_openrc_runtime_state_with(config_dir, resolve_source_dir)
    }

    pub fn validate_vibewindow_user_entry(
        passwd_entry: &str,
        del_cmd: &str,
        add_cmd: &str,
    ) -> Result<()> {
        super::validate_vibewindow_user_entry(passwd_entry, del_cmd, add_cmd)
    }
}

/// 生成 OpenRC init 脚本文本。
///
/// `exe_path` 是服务可执行文件路径，`config_dir` 是系统配置目录。返回值包含以
/// `vibewindow:vibewindow` 身份运行的 OpenRC 脚本内容。
pub(super) fn generate_openrc_script(exe_path: &Path, config_dir: &Path) -> String {
    format!(
        r#"#!/sbin/openrc-run

name="vibewindow"
description="VibeWindow daemon"

command="{}"
command_args="--config-dir {} daemon"
command_background="yes"
command_user="vibewindow:vibewindow"
pidfile="/run/${{RC_SVCNAME}}.pid"
umask 027
output_log="/var/log/vibewindow/access.log"
error_log="/var/log/vibewindow/error.log"

depend() {{
    need net
    after firewall
}}
"#,
        exe_path.display(),
        config_dir.display()
    )
}

/// 解析 OpenRC 服务应使用的可执行文件路径。
///
/// 使用当前执行 service install 的本地 CLI，避免选中另一份旧版可执行文件。
fn resolve_openrc_executable() -> Result<PathBuf> {
    std::env::current_exe().context("Failed to resolve current executable")
}

/// 安装 Linux OpenRC 服务。
///
/// `config` 当前只用于保持服务安装接口一致。函数必须以 root 运行；它会创建系统
/// 用户、准备配置/工作区/日志目录、写入 `/etc/init.d/vibewindow` 并加入 default
/// runlevel。任何关键文件系统或权限步骤失败都会返回错误。
pub(super) fn install_linux_openrc(config: &Config) -> Result<()> {
    if !is_root() {
        bail!(
            "OpenRC service installation requires root privileges.\n\
             Please run with sudo: sudo shiju service install"
        );
    }

    ensure_vibewindow_user()?;

    let exe = resolve_openrc_executable()?;
    warn_if_binary_in_home(&exe);

    let config_dir = Path::new("/etc/vibewindow");
    let workspace_dir = config_dir.join("workspace");
    let log_dir = Path::new("/var/log/vibewindow");

    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .with_context(|| format!("Failed to create {}", config_dir.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(config_dir, fs::Permissions::from_mode(0o755)).with_context(
                || format!("Failed to set permissions on {}", config_dir.display()),
            )?;
        }
        println!("✅ Created directory: {}", config_dir.display());
    }

    migrate_openrc_runtime_state_if_needed(config_dir)?;

    if !workspace_dir.exists() {
        fs::create_dir_all(&workspace_dir)
            .with_context(|| format!("Failed to create {}", workspace_dir.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&workspace_dir, fs::Permissions::from_mode(0o750)).with_context(
                || format!("Failed to set permissions on {}", workspace_dir.display()),
            )?;
        }
        chown_to_vibewindow(&workspace_dir)?;
        println!(
            "✅ Created directory: {} (owned by vibewindow:vibewindow)",
            workspace_dir.display()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&workspace_dir, fs::Permissions::from_mode(0o750))
            .with_context(|| format!("Failed to set permissions on {}", workspace_dir.display()))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(config_dir, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Failed to set permissions on {}", config_dir.display()))?;

        let config_path = config_dir.join("vibewindow.json");
        if config_path.exists() {
            fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).with_context(
                || format!("Failed to set permissions on {}", config_path.display()),
            )?;
        }

        let secret_key_path = config_dir.join(".secret_key");
        if secret_key_path.exists() {
            fs::set_permissions(&secret_key_path, fs::Permissions::from_mode(0o600)).with_context(
                || format!("Failed to set permissions on {}", secret_key_path.display()),
            )?;
        }
    }

    // 配置目录中可能包含迁移而来的状态文件；递归修正所有者能保证守护进程以
    // 低权限用户启动后仍可读取自己的配置，同时避免继续依赖 root 权限。
    chown_recursive_to_vibewindow(config_dir)?;

    let created_log_dir = !log_dir.exists();
    if created_log_dir {
        fs::create_dir_all(log_dir)
            .with_context(|| format!("Failed to create {}", log_dir.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(log_dir, fs::Permissions::from_mode(0o750))
                .with_context(|| format!("Failed to set permissions on {}", log_dir.display()))?;
        }
    }

    chown_to_vibewindow(log_dir)?;
    ensure_openrc_runtime_dirs_writable(config_dir, &workspace_dir, log_dir)?;

    if created_log_dir {
        println!("✅ Created directory: {} (owned by vibewindow:vibewindow)", log_dir.display());
    }

    let init_script = generate_openrc_script(&exe, config_dir);
    let init_path = Path::new("/etc/init.d/vibewindow");
    fs::write(init_path, init_script)
        .with_context(|| format!("Failed to write {}", init_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(init_path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Failed to set permissions on {}", init_path.display()))?;
    }

    run_checked(Command::new("rc-update").args(["add", "vibewindow", "default"]))?;
    println!("✅ Installed OpenRC service: /etc/init.d/vibewindow");
    println!("   Config path: /etc/vibewindow/vibewindow.json");
    println!("   Start with: sudo shiju service start");
    let _ = config;
    Ok(())
}
