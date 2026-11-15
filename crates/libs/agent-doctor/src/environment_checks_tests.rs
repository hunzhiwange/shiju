// 覆盖命令可用性与关键环境变量的诊断结果。

use std::sync::{Mutex, MutexGuard};

use super::environment_checks::{check_command_available, check_environment};
use super::{DiagItem, Severity};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
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

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().expect("environment lock should be available")
}

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn missing_command_is_reported_as_unavailable() {
    let command = "vibewindow-command-that-should-not-exist";
    let mut items = Vec::<DiagItem>::new();

    check_command_available(command, &["--version"], "env", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Warn);
}

#[test]
fn command_with_non_zero_exit_is_reported_as_warning() {
    let mut items = Vec::<DiagItem>::new();

    check_command_available("sh", &["-c", "exit 7"], "env", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Warn);
    assert!(items[0].message.contains("found but returned non-zero"));
}

#[test]
fn successful_command_records_first_stdout_line() {
    let mut items = Vec::<DiagItem>::new();

    check_command_available("sh", &["-c", "printf 'doctor-ok\\nsecond-line'"], "env", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Ok);
    assert!(items[0].message.contains("doctor-ok"));
    assert!(!items[0].message.contains("second-line"));
}

#[test]
fn successful_command_without_stdout_records_empty_preview() {
    let mut items = Vec::<DiagItem>::new();

    check_command_available("sh", &["-c", "true"], "env", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Ok);
    assert_eq!(items[0].message, "sh: ");
}

#[test]
fn environment_reports_shell_and_home_when_set() {
    let _lock = lock_env();
    let _shell = EnvGuard::set("SHELL", "/bin/sh");
    let _home = EnvGuard::set("HOME", "/tmp/vibewindow-home");
    let _userprofile = EnvGuard::unset("USERPROFILE");
    let mut items = Vec::<DiagItem>::new();

    check_environment(&mut items);

    assert!(
        items.iter().any(|item| item.severity == Severity::Ok && item.message == "shell: /bin/sh")
    );
    assert!(
        items
            .iter()
            .any(|item| item.severity == Severity::Ok && item.message == "home directory env set")
    );
}

#[test]
fn environment_reports_missing_shell_and_home() {
    let _lock = lock_env();
    let _shell = EnvGuard::unset("SHELL");
    let _home = EnvGuard::unset("HOME");
    let _userprofile = EnvGuard::unset("USERPROFILE");
    let mut items = Vec::<DiagItem>::new();

    check_environment(&mut items);

    assert!(
        items
            .iter()
            .any(|item| item.severity == Severity::Warn && item.message == "$SHELL not set")
    );
    assert!(items.iter().any(|item| item.severity == Severity::Error
        && item.message == "neither $HOME nor $USERPROFILE is set"));
}

#[test]
fn environment_accepts_userprofile_as_home_fallback() {
    let _lock = lock_env();
    let _shell = EnvGuard::set("SHELL", "/bin/sh");
    let _home = EnvGuard::unset("HOME");
    let _userprofile = EnvGuard::set("USERPROFILE", "C:\\Users\\vibewindow");
    let mut items = Vec::<DiagItem>::new();

    check_environment(&mut items);

    assert!(
        items
            .iter()
            .any(|item| item.severity == Severity::Ok && item.message == "home directory env set")
    );
}
