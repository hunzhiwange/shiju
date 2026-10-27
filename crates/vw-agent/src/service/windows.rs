use crate::app::agent::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::common::{run_capture, run_checked};

/// Windows 任务计划程序中的任务名称。
const WINDOWS_TASK_NAME: &str = "VibeWindow Daemon";

pub(super) fn windows_task_name() -> &'static str {
    WINDOWS_TASK_NAME
}

pub(super) fn install_windows(config: &Config) -> Result<()> {
    install_windows_with_schtasks(config, Path::new("schtasks"))
}

pub(super) fn install_windows_with_schtasks(config: &Config, schtasks: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir =
        config.config_path.parent().map_or_else(|| PathBuf::from("."), PathBuf::from).join("logs");
    fs::create_dir_all(&logs_dir)?;

    let wrapper = logs_dir.join("vibewindow-daemon.cmd");
    let stdout_log = logs_dir.join("daemon.stdout.log");
    let stderr_log = logs_dir.join("daemon.stderr.log");
    let wrapper_content = format!(
        "@echo off\r\n\"{}\" daemon >>\"{}\" 2>>\"{}\"",
        exe.display(),
        stdout_log.display(),
        stderr_log.display()
    );
    fs::write(&wrapper, &wrapper_content)?;

    let task_name = windows_task_name();
    let _ = Command::new(schtasks).args(["/Delete", "/TN", task_name, "/F"]).output();
    run_checked(Command::new(schtasks).args([
        "/Create",
        "/TN",
        task_name,
        "/SC",
        "ONLOGON",
        "/TR",
        &format!("\"{}\"", wrapper.display()),
        "/RL",
        "HIGHEST",
        "/F",
    ]))?;

    println!("✅ Installed Windows scheduled task: {}", task_name);
    println!("   Wrapper: {}", wrapper.display());
    println!("   Logs: {}", logs_dir.display());
    println!("   Start with: shiju service start");
    Ok(())
}

pub(super) fn start_windows() -> Result<()> {
    start_windows_with_schtasks(Path::new("schtasks"))
}

pub(super) fn start_windows_with_schtasks(schtasks: &Path) -> Result<()> {
    run_checked(Command::new(schtasks).args(["/Run", "/TN", windows_task_name()]))?;
    println!("✅ Service started");
    Ok(())
}

pub(super) fn stop_windows() -> Result<()> {
    stop_windows_with_schtasks(Path::new("schtasks"))
}

pub(super) fn stop_windows_with_schtasks(schtasks: &Path) -> Result<()> {
    let _ = run_checked(Command::new(schtasks).args(["/End", "/TN", windows_task_name()]));
    println!("✅ Service stopped");
    Ok(())
}

pub(super) fn restart_windows() -> Result<()> {
    restart_windows_with_schtasks(Path::new("schtasks"))
}

pub(super) fn restart_windows_with_schtasks(schtasks: &Path) -> Result<()> {
    stop_windows_with_schtasks(schtasks)?;
    start_windows_with_schtasks(schtasks)?;
    println!("✅ Service restarted");
    Ok(())
}

pub(super) fn status_windows() -> Result<()> {
    status_windows_with_schtasks(Path::new("schtasks"))
}

pub(super) fn status_windows_with_schtasks(schtasks: &Path) -> Result<()> {
    let task_name = windows_task_name();
    let out = run_capture(Command::new(schtasks).args(["/Query", "/TN", task_name, "/FO", "LIST"]));
    match out {
        Ok(text) => {
            let running = text.contains("Running");
            println!("Service: {}", if running { "✅ running" } else { "❌ not running" });
            println!("Task: {}", task_name);
        }
        Err(_) => {
            println!("Service: ❌ not installed");
        }
    }
    Ok(())
}

pub(super) fn uninstall_windows(config: &Config) -> Result<()> {
    uninstall_windows_with_schtasks(config, Path::new("schtasks"))
}

pub(super) fn uninstall_windows_with_schtasks(config: &Config, schtasks: &Path) -> Result<()> {
    let _ = run_checked(Command::new(schtasks).args(["/Delete", "/TN", windows_task_name(), "/F"]));
    let wrapper = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs")
        .join("vibewindow-daemon.cmd");
    if wrapper.exists() {
        fs::remove_file(&wrapper).ok();
    }
    println!("✅ Service uninstalled");
    Ok(())
}
