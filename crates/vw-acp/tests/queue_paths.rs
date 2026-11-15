//! 验证 ACP 队列锁文件与 IPC socket 路径的稳定生成规则。
//!
//! 队列路径同时承担跨进程互斥和本机 IPC 寻址职责，因此测试固定 hash 前缀、
//! home 目录归属和平台差异，避免 session id 或用户目录被直接拼进不安全路径。

use std::path::{Path, PathBuf};

use vw_acp::{
    default_home_dir, default_queue_base_dir, default_queue_lock_file_path,
    default_queue_socket_base_dir, default_queue_socket_path, queue_base_dir,
    queue_key_for_session, queue_lock_file_path, queue_socket_base_dir, queue_socket_path,
};

/// session id 对外只暴露固定长度的 SHA-256 前缀，避免原始 id 泄露到路径名。
#[test]
fn queue_key_for_session_matches_sha256_prefix() {
    assert_eq!(queue_key_for_session("session-123"), "b9c84322f82434cb46e239d2");
}

#[test]
fn queue_key_for_empty_session_matches_sha256_prefix() {
    assert_eq!(queue_key_for_session(""), "e3b0c44298fc1c149afbf4c8");
}

#[test]
fn queue_key_for_session_hides_path_like_input() {
    let key = queue_key_for_session("session-with/slashes and spaces");

    assert_eq!(key, "c708ae709a254169332b25d6");
    assert!(!key.contains('/'));
    assert!(!key.contains(' '));
}

/// 锁文件位于用户 home 下的 VibeWindow ACP 队列目录，便于随用户数据生命周期清理。
#[test]
fn queue_base_dir_and_lock_file_path_use_home_directory() {
    let home_dir = Path::new("/Users/tester");

    assert_eq!(
        queue_base_dir(home_dir),
        config_runtime_paths::home_config_dir(home_dir).join("acp").join("queues")
    );
    assert_eq!(
        queue_lock_file_path("session-123", home_dir),
        config_runtime_paths::home_config_dir(home_dir)
            .join("acp")
            .join("queues")
            .join("b9c84322f82434cb46e239d2.lock")
    );
}

#[test]
fn queue_base_dir_accepts_relative_home_directory() {
    assert_eq!(
        queue_base_dir(Path::new("relative-home")),
        PathBuf::from("relative-home")
            .join(config_runtime_paths::HOME_CONFIG_DIR_NAME)
            .join("acp")
            .join("queues")
    );
}

/// Unix socket 路径放在系统临时目录的 home-hash 子目录中，以规避 socket 路径长度限制。
#[cfg(not(windows))]
#[test]
fn unix_queue_socket_paths_are_hashed_under_system_temp_dir() {
    let home_dir = Path::new("/Users/tester");
    let socket_base_dir = std::env::temp_dir().join("vwacp-9b643592ef");

    assert_eq!(queue_socket_base_dir(home_dir), Some(socket_base_dir.clone()));
    assert_eq!(
        queue_socket_path("session-123", home_dir),
        socket_base_dir.join("b9c84322f82434cb46e239d2.sock")
    );
}

#[cfg(not(windows))]
#[test]
fn unix_default_queue_paths_are_derived_from_home() {
    let session_id = "session-123";
    let expected_home = std::env::var_os("HOME").map(PathBuf::from);

    assert_eq!(default_home_dir(), expected_home);
    assert_eq!(default_queue_base_dir(), expected_home.as_ref().map(queue_base_dir));
    assert_eq!(
        default_queue_lock_file_path(session_id),
        expected_home.as_ref().map(|home_dir| queue_lock_file_path(session_id, home_dir))
    );
    assert_eq!(
        default_queue_socket_base_dir(),
        expected_home.as_ref().and_then(queue_socket_base_dir)
    );
    assert_eq!(
        default_queue_socket_path(session_id),
        expected_home.as_ref().map(|home_dir| queue_socket_path(session_id, home_dir))
    );
}

/// Windows 使用命名管道，不需要单独的 socket 基础目录。
#[cfg(windows)]
#[test]
fn windows_queue_socket_path_uses_named_pipe() {
    assert_eq!(
        queue_socket_path("session-123", Path::new("C:\\Users\\tester")),
        PathBuf::from(r"\\.\pipe\vwacp-b9c84322f82434cb46e239d2")
    );
    assert_eq!(queue_socket_base_dir(Path::new("C:\\Users\\tester")), None);
}

#[cfg(windows)]
#[test]
fn windows_default_queue_paths_are_derived_from_platform_home() {
    let session_id = "session-123";
    let expected_home = std::env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| {
        let home_drive = std::env::var_os("HOMEDRIVE")?;
        let home_path = std::env::var_os("HOMEPATH")?;
        let mut joined = PathBuf::from(home_drive);
        joined.push(home_path);
        Some(joined)
    });

    assert_eq!(default_home_dir(), expected_home);
    assert_eq!(default_queue_base_dir(), expected_home.as_ref().map(queue_base_dir));
    assert_eq!(
        default_queue_lock_file_path(session_id),
        expected_home.as_ref().map(|home_dir| queue_lock_file_path(session_id, home_dir))
    );
    assert_eq!(default_queue_socket_base_dir(), None);
    assert_eq!(
        default_queue_socket_path(session_id),
        Some(queue_socket_path(session_id, PathBuf::new()))
    );
}
