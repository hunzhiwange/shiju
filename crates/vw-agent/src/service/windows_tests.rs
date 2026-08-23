//! 验证 Windows 计划任务包装脚本及服务命令的失败恢复。

use super::windows::{
    install_windows, install_windows_with_schtasks, restart_windows, restart_windows_with_schtasks,
    start_windows, start_windows_with_schtasks, status_windows, status_windows_with_schtasks,
    stop_windows, uninstall_windows, windows_task_name,
};

fn temp_config() -> (tempfile::TempDir, crate::app::agent::config::Config) {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = crate::app::agent::config::Config::default();
    config.config_path = dir.path().join("vibewindow.json");
    (dir, config)
}

#[cfg(unix)]
fn fake_schtasks(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temporary schtasks directory");
    let path = dir.path().join("schtasks");
    std::fs::write(&path, script).expect("fake schtasks script");
    let mut permissions = std::fs::metadata(&path).expect("fake schtasks metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("fake schtasks executable");
    (dir, path)
}

#[test]
fn windows_task_name_is_stable() {
    assert_eq!(windows_task_name(), "VibeWindow Daemon");
}

#[cfg(not(target_os = "windows"))]
#[test]
// 即使计划任务注册失败，测试也确认包装脚本生成顺序和错误传播。
fn install_windows_writes_wrapper_before_schtasks_failure() {
    let (_dir, config) = temp_config();
    let result = install_windows(&config);

    let logs_dir = config.config_path.parent().unwrap().join("logs");
    let wrapper = logs_dir.join("vibewindow-daemon.cmd");
    let stdout_log = logs_dir.join("daemon.stdout.log");
    let stderr_log = logs_dir.join("daemon.stderr.log");
    let content = std::fs::read_to_string(&wrapper).expect("wrapper should be written");
    let exe = std::env::current_exe().expect("current exe");

    assert!(result.is_err(), "non-Windows hosts should not have schtasks available");
    assert_eq!(
        content,
        format!(
            "@echo off\r\n\"{}\" daemon >>\"{}\" 2>>\"{}\"",
            exe.display(),
            stdout_log.display(),
            stderr_log.display()
        )
    );
}

#[test]
fn install_windows_errors_when_logs_directory_parent_is_a_file() {
    let (dir, mut config) = temp_config();
    let blocking_file = dir.path().join("not-a-directory");
    std::fs::write(&blocking_file, "fixture").expect("blocking file fixture");
    config.config_path = blocking_file.join("vibewindow.json");

    let result = install_windows(&config);

    assert!(result.is_err(), "a file cannot contain the logs directory");
    assert!(!blocking_file.join("logs").exists());
}

#[test]
fn install_windows_errors_when_wrapper_path_is_a_directory() {
    let (_dir, config) = temp_config();
    let wrapper = config.config_path.parent().unwrap().join("logs").join("vibewindow-daemon.cmd");
    std::fs::create_dir_all(&wrapper).expect("wrapper directory fixture");

    let result = install_windows(&config);

    assert!(result.is_err(), "the wrapper cannot replace a directory");
    assert!(wrapper.is_dir());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn install_windows_reuses_existing_logs_directory_before_scheduler_failure() {
    let (_dir, config) = temp_config();
    let logs_dir = config.config_path.parent().unwrap().join("logs");
    std::fs::create_dir_all(&logs_dir).expect("existing logs directory fixture");

    let result = install_windows(&config);

    assert!(result.is_err(), "non-Windows hosts should not have schtasks available");
    assert!(logs_dir.join("vibewindow-daemon.cmd").is_file());
}

#[cfg(unix)]
#[test]
fn windows_service_commands_accept_successful_schtasks_responses() {
    let (_task_dir, schtasks) = fake_schtasks(
        r#"#!/bin/sh
if [ "$1" = "/Query" ]; then
  printf 'TaskName: VibeWindow Daemon\nStatus: Running\n'
fi
exit 0
"#,
    );
    let (_config_dir, config) = temp_config();

    install_windows_with_schtasks(&config, &schtasks).expect("install through fake schtasks");
    start_windows_with_schtasks(&schtasks).expect("start through fake schtasks");
    restart_windows_with_schtasks(&schtasks).expect("restart through fake schtasks");
    status_windows_with_schtasks(&schtasks).expect("status through fake schtasks");

    assert!(
        config.config_path.parent().unwrap().join("logs").join("vibewindow-daemon.cmd").is_file()
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_windows_reports_spawn_failure_when_schtasks_is_missing() {
    let err = start_windows().expect_err("schtasks should be unavailable off Windows");
    assert!(err.to_string().contains("Failed to spawn command"));
}

#[cfg(not(target_os = "windows"))]
#[test]
// 停止和查询在任务不存在时保持幂等，便于安全重复清理。
fn stop_and_status_windows_are_idempotent_when_schtasks_is_missing() {
    stop_windows().expect("stop ignores command failure");
    status_windows().expect("status maps command failure to not installed output");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn restart_windows_reports_start_failure_when_schtasks_is_missing() {
    let err = restart_windows().expect_err("restart should propagate the start failure");
    assert!(err.to_string().contains("Failed to spawn command"));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn uninstall_windows_removes_existing_wrapper_and_ignores_scheduler_errors() {
    let (_dir, config) = temp_config();
    let wrapper = config.config_path.parent().unwrap().join("logs").join("vibewindow-daemon.cmd");
    std::fs::create_dir_all(wrapper.parent().unwrap()).expect("logs dir");
    std::fs::write(&wrapper, "@echo off\r\n").expect("wrapper fixture");

    uninstall_windows(&config).expect("uninstall should ignore scheduler delete errors");

    assert!(!wrapper.exists());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn uninstall_windows_accepts_an_absent_wrapper() {
    let (_dir, config) = temp_config();
    let wrapper = config.config_path.parent().unwrap().join("logs").join("vibewindow-daemon.cmd");

    uninstall_windows(&config).expect("an absent wrapper is already uninstalled");

    assert!(!wrapper.exists());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn uninstall_windows_ignores_wrapper_cleanup_failure() {
    let (_dir, config) = temp_config();
    let wrapper = config.config_path.parent().unwrap().join("logs").join("vibewindow-daemon.cmd");
    std::fs::create_dir_all(&wrapper).expect("wrapper directory fixture");

    uninstall_windows(&config).expect("wrapper cleanup failures are best effort");

    assert!(wrapper.is_dir());
}
