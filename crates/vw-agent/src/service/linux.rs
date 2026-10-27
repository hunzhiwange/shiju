//! Linux 服务管理实现。
//!
//! 本模块根据 init system 分发 systemd user service 与 OpenRC 的安装、启动、
//! 停止、重启、状态查询和卸载流程。具体 OpenRC 安装细节放在 `openrc` 模块中。

use crate::app::agent::config::Config;
#[cfg(target_os = "linux")]
use anyhow::bail;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::InitSystem;
use super::common::{build_systemd_env_vars, run_capture, run_checked};
use super::openrc::install_linux_openrc;

#[cfg(target_os = "linux")]
/// 自动探测当前 Linux init system。
///
/// 返回 `Systemd` 或 `Openrc`；如果无法识别受支持的 init system，则返回错误并提示
/// 调用方显式传入 `--service-init`。该函数只检查本机标志路径和命令可用性。
pub(super) fn detect_init_system() -> Result<InitSystem> {
    if Path::new("/run/systemd/system").exists() {
        return Ok(InitSystem::Systemd);
    }

    if Path::new("/run/openrc").exists() {
        if Path::new("/sbin/openrc-run").exists() || which::which("rc-service").is_ok() {
            return Ok(InitSystem::Openrc);
        }
    }

    bail!(
        "Could not detect init system. Supported: systemd, OpenRC. \
         Use --service-init to specify manually."
    );
}

/// 安装 Linux 服务。
///
/// `config` 是当前代理配置，`init_system` 必须已经解析为具体 init system。安装失败
/// 会返回底层文件系统或命令执行错误。
pub(super) fn install_linux(config: &Config, init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => install_linux_systemd(config),
        InitSystem::Openrc => install_linux_openrc(config),
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
}

/// 启动 Linux 服务。
///
/// `init_system` 指定使用 systemd user service 或 OpenRC。命令执行失败会返回错误。
pub(super) fn start_linux(init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => {
            run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
            run_checked(Command::new("systemctl").args(["--user", "start", "vibewindow.service"]))?;
        }
        InitSystem::Openrc => {
            run_checked(Command::new("rc-service").args(["vibewindow", "start"]))?;
        }
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
    println!("✅ Service started");
    Ok(())
}

/// 停止 Linux 服务。
///
/// 停止命令失败会被忽略，以便卸载或重复停止场景保持幂等；函数仍会报告完成状态。
pub(super) fn stop_linux(init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => {
            let _ = run_checked(Command::new("systemctl").args([
                "--user",
                "stop",
                "vibewindow.service",
            ]));
        }
        InitSystem::Openrc => {
            let _ = run_checked(Command::new("rc-service").args(["vibewindow", "stop"]));
        }
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
    println!("✅ Service stopped");
    Ok(())
}

/// 重启 Linux 服务。
///
/// systemd 会先 reload user daemon，再重启服务；OpenRC 直接调用 `rc-service restart`。
pub(super) fn restart_linux(init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => {
            run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
            run_checked(Command::new("systemctl").args([
                "--user",
                "restart",
                "vibewindow.service",
            ]))?;
        }
        InitSystem::Openrc => {
            run_checked(Command::new("rc-service").args(["vibewindow", "restart"]))?;
        }
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
    println!("✅ Service restarted");
    Ok(())
}

/// 打印 Linux 服务状态。
///
/// `config` 用于定位 systemd unit 文件，`init_system` 决定查询命令。状态命令失败时
/// 会显示 `unknown`，避免状态查询因为服务未安装而中断。
pub(super) fn status_linux(config: &Config, init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => {
            let out = run_capture(Command::new("systemctl").args([
                "--user",
                "is-active",
                "vibewindow.service",
            ]))
            .unwrap_or_else(|_| "unknown".into());
            println!("Service state: {}", out.trim());
            println!("Unit: {}", linux_service_file(config)?.display());
        }
        InitSystem::Openrc => {
            let out = run_capture(Command::new("rc-service").args(["vibewindow", "status"]))
                .unwrap_or_else(|_| "unknown".into());
            println!("Service state: {}", out.trim());
            println!("Unit: /etc/init.d/vibewindow");
        }
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
    Ok(())
}

/// 卸载 Linux 服务。
///
/// `config` 用于定位 systemd unit 文件，`init_system` 决定删除 user service 或
/// OpenRC init 脚本。部分刷新/移除 runlevel 操作失败会降级为警告。
pub(super) fn uninstall_linux(config: &Config, init_system: InitSystem) -> Result<()> {
    match init_system {
        InitSystem::Systemd => {
            let file = linux_service_file(config)?;
            if file.exists() {
                fs::remove_file(&file)
                    .with_context(|| format!("Failed to remove {}", file.display()))?;
            }
            let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
            println!("✅ Service uninstalled ({})", file.display());
        }
        InitSystem::Openrc => {
            let init_script = Path::new("/etc/init.d/vibewindow");
            if init_script.exists() {
                if let Err(err) =
                    run_checked(Command::new("rc-update").args(["del", "vibewindow", "default"]))
                {
                    eprintln!(
                        "⚠️  Warning: Could not remove vibewindow from OpenRC default runlevel: {err}"
                    );
                }
                fs::remove_file(init_script)
                    .with_context(|| format!("Failed to remove {}", init_script.display()))?;
            }
            println!("✅ Service uninstalled (/etc/init.d/vibewindow)");
        }
        InitSystem::Auto => unreachable!("Auto should be resolved before this point"),
    }
    Ok(())
}

/// 安装 systemd user service。
///
/// 返回错误表示无法解析当前可执行文件、无法写入 unit 文件或无法创建目录。systemd
/// reload/enable 失败会被忽略，以便在非完整 systemd user 环境中仍能写入 unit 文件。
fn install_linux_systemd(config: &Config) -> Result<()> {
    let file = linux_service_file(config)?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let env_lines = build_systemd_env_vars();
    let config_dir_args = systemd_config_dir_args(config);
    let unit = format!(
        "[Unit]\nDescription=VibeWindow daemon\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={exe}{config_dir_args} daemon\nRestart=always\nRestartSec=3\n{env_lines}\n[Install]\nWantedBy=default.target\n",
        exe = exe.display(),
        env_lines = env_lines,
    );

    fs::write(&file, unit)?;
    let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
    let _ = run_checked(Command::new("systemctl").args(["--user", "enable", "vibewindow.service"]));
    println!("✅ Installed systemd user service: {}", file.display());
    println!("   Start with: shiju service start");
    Ok(())
}

pub(super) fn systemd_config_dir_args(config: &Config) -> String {
    config.config_path.parent().map_or_else(String::new, |path| {
        format!(" --config-dir {}", systemd_quote_arg(&path.display().to_string()))
    })
}

pub(super) fn systemd_quote_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// 返回 systemd user service 文件路径。
///
/// 参数 `config` 当前未参与路径计算，保留在签名中以匹配服务管理调用形态。无法获取
/// 用户 home 目录时返回错误。
pub(super) fn linux_service_file(config: &Config) -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let _ = config;
    Ok(home.join(".config").join("systemd").join("user").join("vibewindow.service"))
}
