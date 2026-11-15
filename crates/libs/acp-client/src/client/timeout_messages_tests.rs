//! 验证代理启动和建会话超时配置的回退规则与诊断提示。
//!
//! 串行环境变量锁隔离进程级配置，避免超时用例相互污染。

use std::ffi::OsString;
use std::sync::{LazyLock, Mutex, MutexGuard};

use tokio::time::Duration;

use super::timeout_messages::{
    build_claude_acp_session_create_timeout_message, build_gemini_acp_startup_timeout_message,
    resolve_claude_acp_session_create_timeout, resolve_gemini_acp_startup_timeout,
};

static TIMEOUT_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn timeout_env_test_lock() -> MutexGuard<'static, ()> {
    TIMEOUT_ENV_TEST_LOCK.lock().expect("timeout env test lock should acquire")
}

struct EnvGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

// 下列行为用例覆盖默认值、合法覆盖、非法回退和认证提示条件。
#[test]
fn gemini_startup_timeout_uses_default_when_env_missing() {
    let _lock = timeout_env_test_lock();
    let _timeout = EnvGuard::unset("VWACP_GEMINI_ACP_STARTUP_TIMEOUT_MS");

    assert_eq!(resolve_gemini_acp_startup_timeout(), Duration::from_millis(15_000));
}

#[test]
fn claude_session_create_timeout_uses_trimmed_positive_env_value() {
    let _lock = timeout_env_test_lock();
    let _timeout = EnvGuard::set("VWACP_CLAUDE_ACP_SESSION_CREATE_TIMEOUT_MS", " 2500 ");

    assert_eq!(resolve_claude_acp_session_create_timeout(), Duration::from_millis(2_500));
}

#[test]
fn timeout_env_values_fall_back_when_invalid_or_zero() {
    let _lock = timeout_env_test_lock();
    let _gemini_timeout = EnvGuard::set("VWACP_GEMINI_ACP_STARTUP_TIMEOUT_MS", "not-a-number");
    let _claude_timeout = EnvGuard::set("VWACP_CLAUDE_ACP_SESSION_CREATE_TIMEOUT_MS", "0");

    assert_eq!(resolve_gemini_acp_startup_timeout(), Duration::from_millis(15_000));
    assert_eq!(resolve_claude_acp_session_create_timeout(), Duration::from_millis(60_000));
}

#[test]
fn gemini_timeout_message_includes_auth_hint_when_api_keys_missing() {
    let _lock = timeout_env_test_lock();
    let _gemini_key = EnvGuard::unset("GEMINI_API_KEY");
    let _google_key = EnvGuard::set("GOOGLE_API_KEY", " \t ");

    let message = build_gemini_acp_startup_timeout_message("gemini --experimental-acp");

    assert!(message.contains("Gemini CLI ACP startup timed out"));
    assert!(message.contains("No GEMINI_API_KEY or GOOGLE_API_KEY was set"));
    assert!(message.contains("Command: gemini --experimental-acp"));
}

#[test]
fn gemini_timeout_message_omits_auth_hint_when_api_key_present() {
    let _lock = timeout_env_test_lock();
    let _gemini_key = EnvGuard::set("GEMINI_API_KEY", "secret");
    let _google_key = EnvGuard::unset("GOOGLE_API_KEY");

    let message = build_gemini_acp_startup_timeout_message("gemini");

    assert!(!message.contains("No GEMINI_API_KEY or GOOGLE_API_KEY was set"));
    assert!(message.contains("API-key-based auth"));
    assert!(message.contains("Command: gemini"));
}

#[test]
fn claude_timeout_message_describes_session_create_stall_and_fallback() {
    let message = build_claude_acp_session_create_timeout_message();

    assert!(message.contains("Claude ACP session creation timed out"));
    assert!(message.contains("persistent-session stall"));
    assert!(message.contains("vwacp claude exec"));
}
