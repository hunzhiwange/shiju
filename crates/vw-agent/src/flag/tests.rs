// 覆盖底层环境标志解析的空值、边界值与非 UTF-8 输入。

use super::{env_string, number, truthy};
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
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
fn missing_flags_are_false_and_absent_numbers_are_none() {
    let _lock = env_lock();
    let _flag = EnvGuard::remove("VIBEWINDOW_TEST_FLAG_DOES_NOT_EXIST");
    let _number = EnvGuard::remove("VIBEWINDOW_TEST_NUMBER_DOES_NOT_EXIST");

    assert!(!truthy("VIBEWINDOW_TEST_FLAG_DOES_NOT_EXIST"));
    assert_eq!(number("VIBEWINDOW_TEST_NUMBER_DOES_NOT_EXIST"), None);
}

#[test]
fn env_string_preserves_empty_unicode_and_whitespace_values() {
    let _lock = env_lock();
    let _empty = EnvGuard::set("VIBEWINDOW_TEST_STRING_EMPTY", "");
    let _unicode = EnvGuard::set("VIBEWINDOW_TEST_STRING_UNICODE", "配置/目录");
    let _whitespace = EnvGuard::set("VIBEWINDOW_TEST_STRING_WHITESPACE", "  value  ");

    assert_eq!(env_string("VIBEWINDOW_TEST_STRING_EMPTY").as_deref(), Some(""));
    assert_eq!(env_string("VIBEWINDOW_TEST_STRING_UNICODE").as_deref(), Some("配置/目录"));
    assert_eq!(env_string("VIBEWINDOW_TEST_STRING_WHITESPACE").as_deref(), Some("  value  "));
}

#[test]
fn truthy_rejects_whitespace_and_numeric_near_misses() {
    let _lock = env_lock();
    let _padded_true = EnvGuard::set("VIBEWINDOW_TEST_TRUTHY_PADDED", " true ");
    let _leading_zero = EnvGuard::set("VIBEWINDOW_TEST_TRUTHY_LEADING_ZERO", "01");
    let _empty = EnvGuard::set("VIBEWINDOW_TEST_TRUTHY_EMPTY", "");

    assert!(!truthy("VIBEWINDOW_TEST_TRUTHY_PADDED"));
    assert!(!truthy("VIBEWINDOW_TEST_TRUTHY_LEADING_ZERO"));
    assert!(!truthy("VIBEWINDOW_TEST_TRUTHY_EMPTY"));
}

#[test]
fn number_accepts_i64_maximum() {
    let _lock = env_lock();
    let _maximum = EnvGuard::set("VIBEWINDOW_TEST_NUMBER_I64_MAX", i64::MAX.to_string());

    assert_eq!(number("VIBEWINDOW_TEST_NUMBER_I64_MAX"), Some(i64::MAX as u64));
}

#[test]
fn number_rejects_overflow_fraction_and_whitespace() {
    let _lock = env_lock();
    let _overflow =
        EnvGuard::set("VIBEWINDOW_TEST_NUMBER_OVERFLOW", (i64::MAX as u64 + 1).to_string());
    let _fraction = EnvGuard::set("VIBEWINDOW_TEST_NUMBER_FRACTION", "1.5");
    let _whitespace = EnvGuard::set("VIBEWINDOW_TEST_NUMBER_WHITESPACE", " 42 ");

    assert_eq!(number("VIBEWINDOW_TEST_NUMBER_OVERFLOW"), None);
    assert_eq!(number("VIBEWINDOW_TEST_NUMBER_FRACTION"), None);
    assert_eq!(number("VIBEWINDOW_TEST_NUMBER_WHITESPACE"), None);
}

#[cfg(unix)]
#[test]
fn env_string_lossily_converts_non_utf8_values() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = env_lock();
    let value = std::ffi::OsString::from_vec(b"left\xF0right".to_vec());
    let _non_utf8 = EnvGuard::set("VIBEWINDOW_TEST_STRING_NON_UTF8", value);

    assert_eq!(env_string("VIBEWINDOW_TEST_STRING_NON_UTF8").as_deref(), Some("left\u{FFFD}right"));
}
