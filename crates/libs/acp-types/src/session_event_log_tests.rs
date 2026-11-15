//! 验证会话事件日志的安全文件名、轮转默认值与主目录路径。
//!
//! 临时主目录和最小会话记录 fixture 用于隔离事件与偏好状态。

use super::*;
use acp_paths::queue_paths::default_home_dir;

static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
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

// 下列行为用例覆盖类型默认值及平台主目录回退。
#[test]
fn session_event_log_uses_default_rotation_limits() {
    let log = session_event_log("s1", "/tmp/home");

    assert_eq!(
        log.active_path,
        config_runtime_paths::home_config_dir("/tmp/home")
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
