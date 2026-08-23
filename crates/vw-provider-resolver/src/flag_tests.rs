//! 环境变量开关测试覆盖字符串读取、严格真值规则、客户端默认值与惰性静态量稳定性。
//!
//! 测试以进程级互斥锁串行修改环境，并在守卫析构时恢复原值，防止并行用例相互影响。

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn new(keys: &[&'static str]) -> Self {
        let lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let saved = keys.iter().map(|key| (*key, std::env::var_os(key))).collect();
        Self { _lock: lock, saved }
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove(&self, key: &str) {
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

#[test]
fn env_string_returns_lossy_string_for_present_key() {
    let env = EnvGuard::new(&["VW_PROVIDER_RESOLVER_FLAG_STRING"]);
    env.set("VW_PROVIDER_RESOLVER_FLAG_STRING", "value");

    assert_eq!(super::env_string("VW_PROVIDER_RESOLVER_FLAG_STRING").as_deref(), Some("value"));
}

#[test]
fn env_string_returns_none_for_missing_key() {
    let env = EnvGuard::new(&["VW_PROVIDER_RESOLVER_FLAG_MISSING"]);
    env.remove("VW_PROVIDER_RESOLVER_FLAG_MISSING");

    assert_eq!(super::env_string("VW_PROVIDER_RESOLVER_FLAG_MISSING"), None);
}

#[test]
fn truthy_accepts_true_and_one_case_insensitively() {
    let env = EnvGuard::new(&["VW_PROVIDER_RESOLVER_FLAG_BOOL"]);

    env.set("VW_PROVIDER_RESOLVER_FLAG_BOOL", "TRUE");
    assert!(super::truthy("VW_PROVIDER_RESOLVER_FLAG_BOOL"));

    env.set("VW_PROVIDER_RESOLVER_FLAG_BOOL", "1");
    assert!(super::truthy("VW_PROVIDER_RESOLVER_FLAG_BOOL"));
}

#[test]
fn truthy_rejects_missing_and_non_truthy_values() {
    let env = EnvGuard::new(&["VW_PROVIDER_RESOLVER_FLAG_BOOL_FALSE"]);

    env.remove("VW_PROVIDER_RESOLVER_FLAG_BOOL_FALSE");
    assert!(!super::truthy("VW_PROVIDER_RESOLVER_FLAG_BOOL_FALSE"));

    env.set("VW_PROVIDER_RESOLVER_FLAG_BOOL_FALSE", "yes");
    assert!(!super::truthy("VW_PROVIDER_RESOLVER_FLAG_BOOL_FALSE"));
}

#[test]
fn vibewindow_client_defaults_to_cli_and_uses_env_value() {
    let env = EnvGuard::new(&["VIBEWINDOW_CLIENT"]);

    env.remove("VIBEWINDOW_CLIENT");
    assert_eq!(super::vibewindow_client(), "cli");

    env.set("VIBEWINDOW_CLIENT", "desktop");
    assert_eq!(super::vibewindow_client(), "desktop");
}

#[test]
fn public_lazy_flags_can_be_read_and_remain_stable() {
    let models_path = super::VIBEWINDOW_MODELS_PATH.clone();
    let disable_fetch = *super::VIBEWINDOW_DISABLE_MODELS_FETCH;
    let models_url = super::VIBEWINDOW_MODELS_URL.clone();

    assert_eq!(super::VIBEWINDOW_MODELS_PATH.clone(), models_path);
    assert_eq!(*super::VIBEWINDOW_DISABLE_MODELS_FETCH, disable_fetch);
    assert_eq!(super::VIBEWINDOW_MODELS_URL.clone(), models_url);
}
