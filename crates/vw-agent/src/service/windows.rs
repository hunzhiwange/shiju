use crate::app::agent::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::common::{run_capture, run_checked};

/// Windows 任务计划程序中的任务名称。
const WINDOWS_TASK_NAME: &str = "ShiJu Daemon";

pub(super) fn windows_task_name() -> &'static str {
    WINDOWS_TASK_NAME
}

fn schtasks_command(program: &Path) -> Command {
    let command = Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    };
    command
}

pub(super) fn install_windows(config: &Config) -> Result<()> {
    install_windows_with_schtasks(config, Path::new("schtasks"))
}

pub(super) fn install_windows_with_schtasks(config: &Config, schtasks: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let config_dir = config.config_path.parent().unwrap_or_else(|| Path::new("."));
    let logs_dir = config_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;

    let wrapper = logs_dir.join("shiju-daemon.cmd");
    let stdout_log = logs_dir.join("daemon.stdout.log");
    let stderr_log = logs_dir.join("daemon.stderr.log");
    let wrapper_content = format!(
        "@echo off\r\nchcp 65001 >nul\r\n\"{}\" --config-dir \"{}\" daemon >>\"{}\" 2>>\"{}\"",
        exe.display(),
        config_dir.display(),
        stdout_log.display(),
        stderr_log.display()
    );
    fs::write(&wrapper, &wrapper_content)?;

    // wscript 本身没有控制台；等待隐藏的 daemon 退出，让计划任务保持运行状态。
    let launcher = logs_dir.join("shiju-daemon.js");
    let command = format!("cmd.exe /d /c \"\"{}\"\"", wrapper.display());
    let script = format!(
        "var shell = new ActiveXObject(\"WScript.Shell\");\r\nWScript.Quit(shell.Run({}, 0, true));\r\n",
        serde_json::to_string(&command)?
    );
    // Windows Script Host 使用带 BOM 的 UTF-16，保留中文用户目录。
    let script_bytes: Vec<u8> = std::iter::once(0xfeff_u16)
        .chain(script.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    fs::write(&launcher, script_bytes)?;

    let task_name = windows_task_name();
    let _ = schtasks_command(schtasks).args(["/Delete", "/TN", task_name, "/F"]).output();
    run_checked(schtasks_command(schtasks).args([
        "/Create",
        "/TN",
        task_name,
        "/SC",
        "ONLOGON",
        "/TR",
        &format!("wscript.exe //B //NoLogo //E:JScript \"{}\"", launcher.display()),
        "/RL",
        "LIMITED",
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
    run_checked(schtasks_command(schtasks).args(["/Run", "/TN", windows_task_name()]))?;
    println!("✅ Service started");
    Ok(())
}

pub(super) fn stop_windows() -> Result<()> {
    stop_windows_with_schtasks(Path::new("schtasks"))
}

pub(super) fn stop_windows_with_schtasks(schtasks: &Path) -> Result<()> {
    let _ = run_checked(schtasks_command(schtasks).args(["/End", "/TN", windows_task_name()]));
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
    let out =
        run_capture(schtasks_command(schtasks).args(["/Query", "/TN", task_name, "/FO", "LIST"]));
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
    let _ =
        run_checked(schtasks_command(schtasks).args(["/Delete", "/TN", windows_task_name(), "/F"]));
    let wrapper = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs")
        .join("shiju-daemon.cmd");
    if wrapper.exists() {
        fs::remove_file(&wrapper).ok();
    }
    let launcher = wrapper.with_extension("js");
    if launcher.exists() {
        fs::remove_file(&launcher).ok();
    }
    println!("✅ Service uninstalled");
    Ok(())
}
