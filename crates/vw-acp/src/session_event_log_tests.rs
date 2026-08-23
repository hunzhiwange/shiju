//! 验证会话事件日志的安全文件名、轮转默认值与主目录路径。
//!
//! 临时主目录和最小会话记录 fixture 用于隔离事件与偏好状态。

use super::*;
use crate::queue_paths::{TEST_HOME_ENV_LOCK, default_home_dir};
use std::ffi::OsString;
use std::path::Path;

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn clear(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

// 下列行为用例覆盖规范化、状态同步、路径安全和陈旧数据恢复。
#[test]
fn safe_session_id_percent_encodes_path_separators_and_spaces() {
    assert_eq!(safe_session_id("abc-_.!~*'()"), "abc-_.!~*'()");
    assert_eq!(safe_session_id("a b/c"), "a%20b%2Fc");
    assert_eq!(safe_session_id("雪"), "%E9%9B%AA");
}

#[test]
fn safe_session_id_handles_empty_and_reserved_ascii() {
    assert_eq!(safe_session_id(""), "");
    assert_eq!(safe_session_id("a+b?c#d%e\n"), "a%2Bb%3Fc%23d%25e%0A");
    assert_eq!(safe_session_id("../escape\\path"), "..%2Fescape%5Cpath");
}

#[test]
fn session_event_paths_use_safe_session_id_under_base_dir() {
    let home = Path::new("/tmp/home");
    let base = vw_config_types::paths::home_config_dir(home).join("acp").join("sessions");

    assert_eq!(session_base_dir(home), base);
    assert_eq!(session_event_active_path("a b", home), base.join("a%20b.stream.ndjson"));
    assert_eq!(session_event_segment_path("a b", 2, home), base.join("a%20b.stream.2.ndjson"));
    assert_eq!(session_event_lock_path("a b", home), base.join("a%20b.stream.lock"));
}

#[test]
fn session_event_log_uses_default_rotation_limits() {
    let log = session_event_log("s1", "/tmp/home");

    assert_eq!(
        log.active_path,
        vw_config_types::paths::home_config_dir("/tmp/home")
            .join("acp")
            .join("sessions")
            .join("s1.stream.ndjson")
            .to_string_lossy()
    );
    assert_eq!(log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.max_segments, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.max_segment_bytes, DEFAULT_EVENT_SEGMENT_MAX_BYTES);
    assert_eq!(log.last_write_at, None);
    assert_eq!(log.last_write_error, None);
}

#[cfg(not(windows))]
#[tokio::test]
async fn default_session_event_paths_use_home_environment() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let _home = EnvVarGuard::set("HOME", "/tmp/vw-acp-event-home");
    let home = Path::new("/tmp/vw-acp-event-home");

    assert_eq!(default_home_dir(), Some(home.to_path_buf()));

    assert_eq!(default_session_base_dir(), Some(session_base_dir(home)));
    assert_eq!(
        default_session_event_active_path("a b"),
        Some(session_event_active_path("a b", home))
    );
    assert_eq!(
        default_session_event_segment_path("a b", 3),
        Some(session_event_segment_path("a b", 3, home))
    );
    assert_eq!(default_session_event_lock_path("a b"), Some(session_event_lock_path("a b", home)));
    assert_eq!(default_session_event_log("a b"), Some(session_event_log("a b", home)));
}

#[cfg(not(windows))]
#[tokio::test]
async fn default_session_event_paths_return_none_without_home_environment() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let _home = EnvVarGuard::clear("HOME");

    assert_eq!(default_home_dir(), None);
    assert_eq!(default_session_base_dir(), None);
    assert_eq!(default_session_event_active_path("s1"), None);
    assert_eq!(default_session_event_segment_path("s1", 1), None);
    assert_eq!(default_session_event_lock_path("s1"), None);
    assert_eq!(default_session_event_log("s1"), None);
}

#[cfg(windows)]
#[tokio::test]
async fn windows_default_session_event_paths_use_userprofile_environment() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let _userprofile = EnvVarGuard::set("USERPROFILE", r"C:\Users\vw-acp-event");
    let _homedrive = EnvVarGuard::clear("HOMEDRIVE");
    let _homepath = EnvVarGuard::clear("HOMEPATH");
    let home = Path::new(r"C:\Users\vw-acp-event");

    assert_eq!(default_home_dir(), Some(home.to_path_buf()));

    assert_eq!(default_session_base_dir(), Some(session_base_dir(home)));
    assert_eq!(
        default_session_event_active_path("a b"),
        Some(session_event_active_path("a b", home))
    );
    assert_eq!(
        default_session_event_segment_path("a b", 3),
        Some(session_event_segment_path("a b", 3, home))
    );
    assert_eq!(default_session_event_lock_path("a b"), Some(session_event_lock_path("a b", home)));
    assert_eq!(default_session_event_log("a b"), Some(session_event_log("a b", home)));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_default_session_event_paths_return_none_without_home_environment() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let _userprofile = EnvVarGuard::clear("USERPROFILE");
    let _homedrive = EnvVarGuard::clear("HOMEDRIVE");
    let _homepath = EnvVarGuard::clear("HOMEPATH");

    assert_eq!(default_home_dir(), None);
    assert_eq!(default_session_base_dir(), None);
    assert_eq!(default_session_event_active_path("s1"), None);
    assert_eq!(default_session_event_segment_path("s1", 1), None);
    assert_eq!(default_session_event_lock_path("s1"), None);
    assert_eq!(default_session_event_log("s1"), None);
}
