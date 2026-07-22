//! # 服务管理模块
//!
//! 本模块提供跨平台的系统服务（守护进程）管理功能，支持将 VibeWindow 代理
//! 安装为系统服务，并管理其生命周期（安装、启动、停止、重启、卸载、状态查询）。
//!
//! ## 支持的平台
//!
//! - **macOS**: 通过 `launchd` (LaunchAgents) 实现
//! - **Linux**: 支持 `systemd` (用户级) 和 `OpenRC` (系统级) 两种 init 系统
//! - **Windows**: 通过任务计划程序 (Task Scheduler) 实现
//!
//! ## 使用示例
//!
//! ```no_run
//! use crate::app::agent::config::Config;
//! use crate::app::agent::service::{handle_command, InitSystem, ServiceCommands};
//!
//! let config = Config::default();
//! handle_command(&ServiceCommands::Install, &config, InitSystem::Auto)?;
//! handle_command(&ServiceCommands::Start, &config, InitSystem::Auto)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

mod common;
mod linux;
mod macos;
mod openrc;
mod windows;

use crate::app::agent::config::Config;
use std::time::Duration;

use anyhow::{Result, bail};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[cfg(test)]
use self::common::{run_capture, run_checked, xml_escape};
#[cfg(test)]
use self::linux::{linux_service_file, systemd_config_dir_args, systemd_quote_arg};
#[cfg(test)]
use self::openrc::generate_openrc_script;
#[cfg(all(test, unix))]
use self::openrc::{
    build_openrc_writability_probe_command, current_uid, is_root, shell_single_quote,
};
#[cfg(test)]
use self::windows::windows_task_name;

/// 支持的 init 系统类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InitSystem {
    /// 自动检测模式。
    #[default]
    Auto,
    /// systemd init 系统。
    Systemd,
    /// OpenRC init 系统。
    Openrc,
}

impl FromStr for InitSystem {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "systemd" => Ok(Self::Systemd),
            "openrc" => Ok(Self::Openrc),
            other => bail!("Unknown init system: '{}'. Supported: auto, systemd, openrc", other),
        }
    }
}

impl InitSystem {
    /// 将自动检测模式解析为具体的 init 系统。
    #[cfg(target_os = "linux")]
    pub fn resolve(self) -> Result<Self> {
        match self {
            Self::Auto => linux::detect_init_system(),
            concrete => Ok(concrete),
        }
    }

    /// 非 Linux 平台上的占位实现。
    #[cfg(not(target_os = "linux"))]
    pub fn resolve(self) -> Result<Self> {
        match self {
            Self::Auto => Ok(Self::Systemd),
            concrete => Ok(concrete),
        }
    }
}

/// 处理服务命令的统一入口。
pub fn handle_command(
    command: &ServiceCommands,
    config: &Config,
    init_system: InitSystem,
) -> Result<()> {
    wait_for_internal_command_delay();

    match command {
        ServiceCommands::Install => install(config, init_system),
        ServiceCommands::Start => start(config, init_system),
        ServiceCommands::Stop => stop(config, init_system),
        ServiceCommands::Restart => restart(config, init_system),
        ServiceCommands::Status => status(config, init_system),
        ServiceCommands::Uninstall => uninstall(config, init_system),
    }
}

fn wait_for_internal_command_delay() {
    const INTERNAL_DELAY_ENV: &str = "VIBEWINDOW_INTERNAL_SERVICE_COMMAND_DELAY_MS";

    let raw = std::env::var(INTERNAL_DELAY_ENV).ok();
    let Some(delay) = internal_command_delay(raw.as_deref()) else {
        return;
    };

    std::thread::sleep(delay);
}

fn internal_command_delay(raw: Option<&str>) -> Option<Duration> {
    let delay_ms = raw?.trim().parse::<u64>().ok()?;
    if delay_ms == 0 || delay_ms > 10_000 {
        return None;
    }

    Some(Duration::from_millis(delay_ms))
}

fn install(config: &Config, init_system: InitSystem) -> Result<()> {
    uninstall(config, init_system)?;

    if cfg!(target_os = "macos") {
        macos::install_macos(config)
    } else if cfg!(target_os = "linux") {
        linux::install_linux(config, init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        windows::install_windows(config)
    } else {
        bail!("Service management is supported on macOS and Linux only");
    }
}

fn start(config: &Config, init_system: InitSystem) -> Result<()> {
    if cfg!(target_os = "macos") {
        let _ = config;
        macos::start_macos()
    } else if cfg!(target_os = "linux") {
        linux::start_linux(init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        let _ = config;
        windows::start_windows()
    } else {
        let _ = config;
        bail!("Service management is supported on macOS and Linux only")
    }
}

fn stop(config: &Config, init_system: InitSystem) -> Result<()> {
    if cfg!(target_os = "macos") {
        let _ = config;
        macos::stop_macos()
    } else if cfg!(target_os = "linux") {
        linux::stop_linux(init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        let _ = config;
        windows::stop_windows()
    } else {
        let _ = config;
        bail!("Service management is supported on macOS and Linux only")
    }
}

fn restart(config: &Config, init_system: InitSystem) -> Result<()> {
    if cfg!(target_os = "macos") {
        let _ = config;
        let _ = init_system;
        macos::restart_macos()
    } else if cfg!(target_os = "linux") {
        let _ = config;
        linux::restart_linux(init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        let _ = config;
        let _ = init_system;
        windows::restart_windows()
    } else {
        let _ = config;
        let _ = init_system;
        bail!("Service management is supported on macOS and Linux only")
    }
}

fn status(config: &Config, init_system: InitSystem) -> Result<()> {
    if cfg!(target_os = "macos") {
        let _ = config;
        macos::status_macos()
    } else if cfg!(target_os = "linux") {
        linux::status_linux(config, init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        let _ = config;
        windows::status_windows()
    } else {
        bail!("Service management is supported on macOS and Linux only")
    }
}

fn uninstall(config: &Config, init_system: InitSystem) -> Result<()> {
    stop(config, init_system)?;

    if cfg!(target_os = "macos") {
        macos::uninstall_macos()
    } else if cfg!(target_os = "linux") {
        linux::uninstall_linux(config, init_system.resolve()?)
    } else if cfg!(target_os = "windows") {
        windows::uninstall_windows(config)
    } else {
        bail!("Service management is supported on macOS and Linux only")
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// 服务管理命令枚举。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Subcommand)]
pub enum ServiceCommands {
    /// 安装服务。
    Install,
    /// 启动服务。
    Start,
    /// 停止服务。
    Stop,
    /// 重启服务。
    Restart,
    /// 查询服务状态。
    Status,
    /// 卸载服务。
    Uninstall,
}
#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
#[cfg(test)]
#[path = "linux_tests.rs"]
mod linux_tests;
#[cfg(test)]
#[path = "macos_tests.rs"]
mod macos_tests;
#[cfg(test)]
#[path = "openrc_tests.rs"]
mod openrc_tests;
#[cfg(test)]
#[path = "windows_tests.rs"]
mod windows_tests;
