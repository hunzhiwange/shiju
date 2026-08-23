//! 验证认证环境变量别名、凭据优先级以及令牌名称规范化。

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{LazyLock, Mutex, MutexGuard};

use super::auth_env::{basename_token, build_agent_environment, read_env_credential, to_env_token};

static AUTH_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn auth_env_test_lock() -> MutexGuard<'static, ()> {
    AUTH_ENV_TEST_LOCK.lock().expect("auth env test lock should acquire")
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

#[test]
fn build_agent_environment_injects_valid_aliases_and_filters_empty_credentials() {
    let mut credentials = HashMap::new();
    credentials.insert("openai-api-key".to_string(), "secret".to_string());
    credentials.insert("empty".to_string(), "  ".to_string());
    credentials.insert("bad=key".to_string(), "equals-secret".to_string());
    credentials.insert("nul\0key".to_string(), "nul-secret".to_string());
    credentials.insert("***".to_string(), "symbols-secret".to_string());

    let env = build_agent_environment(&credentials);

    assert_eq!(env.get("openai-api-key").map(String::as_str), Some("secret"));
    assert_eq!(env.get("OPENAI_API_KEY").map(String::as_str), Some("secret"));
    assert_eq!(env.get("VWACP_AUTH_OPENAI_API_KEY").map(String::as_str), Some("secret"));
    assert!(!env.contains_key("empty"));
    assert!(!env.contains_key("bad=key"));
    assert_eq!(env.get("BAD_KEY").map(String::as_str), Some("equals-secret"));
    assert_eq!(env.get("VWACP_AUTH_BAD_KEY").map(String::as_str), Some("equals-secret"));
    assert!(!env.contains_key("nul\0key"));
    assert_eq!(env.get("NUL_KEY").map(String::as_str), Some("nul-secret"));
    assert_eq!(env.get("VWACP_AUTH_NUL_KEY").map(String::as_str), Some("nul-secret"));
    assert_eq!(env.get("***").map(String::as_str), Some("symbols-secret"));
    assert!(!env.contains_key("VWACP_AUTH_"));
}

#[test]
fn read_env_credential_prefers_raw_method_id_over_aliases() {
    let _lock = auth_env_test_lock();
    let _raw = EnvGuard::set("auth-env-test-prefer-raw", "raw-secret");
    let _normalized = EnvGuard::set("AUTH_ENV_TEST_PREFER_RAW", "normalized-secret");
    let _prefixed = EnvGuard::set("VWACP_AUTH_AUTH_ENV_TEST_PREFER_RAW", "prefixed-secret");

    let credential = read_env_credential("auth-env-test-prefer-raw");

    assert_eq!(credential.as_deref(), Some("raw-secret"));
}

#[test]
fn read_env_credential_uses_normalized_alias_after_blank_raw_value() {
    let _lock = auth_env_test_lock();
    let _raw = EnvGuard::set("auth-env-test-normalized", " \t ");
    let _normalized = EnvGuard::set("AUTH_ENV_TEST_NORMALIZED", "normalized-secret");
    let _prefixed = EnvGuard::set("VWACP_AUTH_AUTH_ENV_TEST_NORMALIZED", "prefixed-secret");

    let credential = read_env_credential("auth-env-test-normalized");

    assert_eq!(credential.as_deref(), Some("normalized-secret"));
}

#[test]
fn read_env_credential_uses_prefixed_alias_when_other_keys_are_missing() {
    let _lock = auth_env_test_lock();
    let _raw = EnvGuard::unset("auth-env-test-prefixed");
    let _normalized = EnvGuard::unset("AUTH_ENV_TEST_PREFIXED");
    let _prefixed = EnvGuard::set("VWACP_AUTH_AUTH_ENV_TEST_PREFIXED", "prefixed-secret");

    let credential = read_env_credential("auth-env-test-prefixed");

    assert_eq!(credential.as_deref(), Some("prefixed-secret"));
}

#[test]
fn read_env_credential_returns_none_for_missing_or_blank_values() {
    let _lock = auth_env_test_lock();
    let _raw = EnvGuard::set("auth-env-test-blank", " ");
    let _normalized = EnvGuard::set("AUTH_ENV_TEST_BLANK", "\n");
    let _prefixed = EnvGuard::set("VWACP_AUTH_AUTH_ENV_TEST_BLANK", "\t");
    let _missing_raw = EnvGuard::unset("auth-env-test-missing");
    let _missing_normalized = EnvGuard::unset("AUTH_ENV_TEST_MISSING");
    let _missing_prefixed = EnvGuard::unset("VWACP_AUTH_AUTH_ENV_TEST_MISSING");

    assert_eq!(read_env_credential("auth-env-test-blank"), None);
    assert_eq!(read_env_credential("auth-env-test-missing"), None);
}

#[test]
fn to_env_token_normalizes_ascii_segments_and_rejects_empty_results() {
    assert_eq!(to_env_token(" openai-api key "), Some("OPENAI_API_KEY".to_string()));
    assert_eq!(to_env_token("model.1"), Some("MODEL_1".to_string()));
    assert_eq!(to_env_token("a密b"), Some("A_B".to_string()));
    assert_eq!(to_env_token(" -- "), None);
    assert_eq!(to_env_token("密钥"), None);
}

#[test]
fn basename_token_extracts_file_name_and_lowercases_it() {
    assert_eq!(basename_token("/usr/local/bin/CODEX-ACP"), "codex-acp");
    assert_eq!(basename_token("Agent"), "agent");
    assert_eq!(basename_token(""), "");
}
