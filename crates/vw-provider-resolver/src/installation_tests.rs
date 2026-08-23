//! 安装信息测试验证 User-Agent 对发布通道、版本与客户端标识的组合规则。
//!
//! 用例同时固定本地开发环境的 `local`/`cli` 回退，并通过可恢复的环境守卫隔离显式值。

use std::env;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

use super::*;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    values: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn new(keys: &[&'static str]) -> Self {
        let lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let values = keys.iter().map(|key| (*key, env::var_os(key))).collect();
        Self { _lock: lock, values }
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { env::set_var(key, value) };
    }

    fn remove(&self, key: &str) {
        unsafe { env::remove_var(key) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            match value {
                Some(value) => unsafe { env::set_var(key, value) },
                None => unsafe { env::remove_var(key) },
            }
        }
    }
}

#[test]
fn user_agent_uses_local_defaults_when_installation_env_is_absent() {
    let env_guard =
        EnvGuard::new(&["VIBEWINDOW_CHANNEL", "VIBEWINDOW_VERSION", "VIBEWINDOW_CLIENT"]);
    env_guard.remove("VIBEWINDOW_CHANNEL");
    env_guard.remove("VIBEWINDOW_VERSION");
    env_guard.remove("VIBEWINDOW_CLIENT");

    assert_eq!(channel(), "local");
    assert_eq!(version(), "local");
    assert_eq!(user_agent(), "vibewindow/local/local/cli");
}

#[test]
fn user_agent_uses_explicit_channel_version_and_client() {
    let env_guard =
        EnvGuard::new(&["VIBEWINDOW_CHANNEL", "VIBEWINDOW_VERSION", "VIBEWINDOW_CLIENT"]);
    env_guard.set("VIBEWINDOW_CHANNEL", "beta");
    env_guard.set("VIBEWINDOW_VERSION", "1.2.3");
    env_guard.set("VIBEWINDOW_CLIENT", "desktop");

    assert_eq!(channel(), "beta");
    assert_eq!(version(), "1.2.3");
    assert_eq!(user_agent(), "vibewindow/beta/1.2.3/desktop");
}
