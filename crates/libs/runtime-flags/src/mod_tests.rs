// 覆盖公开环境标志辅助函数、动态默认值与静态标志读取。

use super::{env_string, number, truthy, vibewindow_client, vibewindow_config_dir};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let old = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, old }
    }

    fn remove(key: &'static str) -> Self {
        let old = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

// 这些用例会修改进程级环境变量，因此通过共享锁与守卫避免并发污染并恢复原值。
#[test]
fn env_string_reads_present_values_and_missing_as_none() {
    let _lock = env_lock();
    let _missing = EnvGuard::remove("VIBEWINDOW_UNIT_ENV_STRING_MISSING");
    let _present = EnvGuard::set("VIBEWINDOW_UNIT_ENV_STRING_PRESENT", "hello");

    assert_eq!(env_string("VIBEWINDOW_UNIT_ENV_STRING_PRESENT").as_deref(), Some("hello"));
    assert_eq!(env_string("VIBEWINDOW_UNIT_ENV_STRING_MISSING"), None);
}

#[test]
fn truthy_accepts_true_case_insensitively_and_one_only() {
    let _lock = env_lock();
    let _true_upper = EnvGuard::set("VIBEWINDOW_UNIT_TRUTHY_TRUE", "TrUe");
    let _one = EnvGuard::set("VIBEWINDOW_UNIT_TRUTHY_ONE", "1");
    let _false = EnvGuard::set("VIBEWINDOW_UNIT_TRUTHY_FALSE", "yes");
    let _missing = EnvGuard::remove("VIBEWINDOW_UNIT_TRUTHY_MISSING");

    assert!(truthy("VIBEWINDOW_UNIT_TRUTHY_TRUE"));
    assert!(truthy("VIBEWINDOW_UNIT_TRUTHY_ONE"));
    assert!(!truthy("VIBEWINDOW_UNIT_TRUTHY_FALSE"));
    assert!(!truthy("VIBEWINDOW_UNIT_TRUTHY_MISSING"));
}

#[test]
fn number_accepts_positive_i64_values_only() {
    let _lock = env_lock();
    let _positive = EnvGuard::set("VIBEWINDOW_UNIT_NUMBER_POSITIVE", "42");
    let _zero = EnvGuard::set("VIBEWINDOW_UNIT_NUMBER_ZERO", "0");
    let _negative = EnvGuard::set("VIBEWINDOW_UNIT_NUMBER_NEGATIVE", "-1");
    let _invalid = EnvGuard::set("VIBEWINDOW_UNIT_NUMBER_INVALID", "abc");
    let _missing = EnvGuard::remove("VIBEWINDOW_UNIT_NUMBER_MISSING");

    assert_eq!(number("VIBEWINDOW_UNIT_NUMBER_POSITIVE"), Some(42));
    assert_eq!(number("VIBEWINDOW_UNIT_NUMBER_ZERO"), None);
    assert_eq!(number("VIBEWINDOW_UNIT_NUMBER_NEGATIVE"), None);
    assert_eq!(number("VIBEWINDOW_UNIT_NUMBER_INVALID"), None);
    assert_eq!(number("VIBEWINDOW_UNIT_NUMBER_MISSING"), None);
}

#[test]
fn dynamic_public_helpers_reflect_environment_and_defaults() {
    let _lock = env_lock();
    let _project_config = EnvGuard::set("VIBEWINDOW_DISABLE_PROJECT_CONFIG", "1");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", "/tmp/vibewindow-config");
    let _client = EnvGuard::set("VIBEWINDOW_CLIENT", "desktop");

    assert!(super::vibewindow_disable_project_config());
    assert_eq!(vibewindow_config_dir().as_deref(), Some("/tmp/vibewindow-config"));
    assert_eq!(vibewindow_client(), "desktop");
}

#[test]
fn client_defaults_to_cli_when_unset() {
    let _lock = env_lock();
    let _client = EnvGuard::remove("VIBEWINDOW_CLIENT");

    assert_eq!(vibewindow_client(), "cli");
}

#[test]
fn static_bool_flags_are_readable() {
    let flags = [
        *super::VIBEWINDOW_AUTO_SHARE,
        *super::VIBEWINDOW_DISABLE_AUTOUPDATE,
        *super::VIBEWINDOW_DISABLE_PRUNE,
        *super::VIBEWINDOW_DISABLE_TERMINAL_TITLE,
        *super::VIBEWINDOW_DISABLE_DEFAULT_PLUGINS,
        *super::VIBEWINDOW_DISABLE_LSP_DOWNLOAD,
        *super::VIBEWINDOW_ENABLE_EXPERIMENTAL_MODELS,
        *super::VIBEWINDOW_DISABLE_AUTOCOMPACT,
        *super::VIBEWINDOW_DISABLE_CLAUDE_CODE,
        *super::VIBEWINDOW_DISABLE_CLAUDE_CODE_PROMPT,
        *super::VIBEWINDOW_DISABLE_CLAUDE_CODE_SKILLS,
        *super::VIBEWINDOW_DISABLE_EXTERNAL_SKILLS,
        *super::VIBEWINDOW_EXPERIMENTAL,
        *super::VIBEWINDOW_EXPERIMENTAL_FILEWATCHER,
        *super::VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER,
        *super::VIBEWINDOW_EXPERIMENTAL_ICON_DISCOVERY,
        *super::VIBEWINDOW_EXPERIMENTAL_DISABLE_COPY_ON_SELECT,
        *super::VIBEWINDOW_ENABLE_EXA,
        *super::VIBEWINDOW_EXPERIMENTAL_OXFMT,
        *super::VIBEWINDOW_EXPERIMENTAL_LSP_TY,
        *super::VIBEWINDOW_DISABLE_FILETIME_CHECK,
        *super::VIBEWINDOW_EXPERIMENTAL_MARKDOWN,
    ];

    assert_eq!(flags.len(), 23);
}

#[test]
fn static_string_flags_are_readable() {
    let flags = [
        &*super::VIBEWINDOW_GIT_BASH_PATH,
        &*super::VIBEWINDOW_CONFIG,
        &*super::VIBEWINDOW_CONFIG_CONTENT,
        &*super::VIBEWINDOW_PERMISSION,
        &*super::VIBEWINDOW_FAKE_VCS,
        &*super::VIBEWINDOW_SERVER_PASSWORD,
        &*super::VIBEWINDOW_SERVER_USERNAME,
        &*super::VIBEWINDOW_MODELS_PATH,
    ];

    assert_eq!(flags.len(), 8);
}

#[test]
fn static_number_flags_are_readable() {
    let flags = [
        &*super::VIBEWINDOW_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS,
        &*super::VIBEWINDOW_EXPERIMENTAL_OUTPUT_TOKEN_MAX,
    ];

    assert_eq!(flags.len(), 2);
}
