use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use super::environment_checks::{
    append_cli_tool_items, check_cli_tools, check_command_available, check_environment,
};
use super::{DiagItem, Severity};
use crate::tools::cli_discovery::{CliCategory, DiscoveredCli};

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

#[test]
fn cli_tools_check_reports_at_least_one_diagnostic() {
    let mut items = Vec::<DiagItem>::new();

    check_cli_tools(&mut items);

    assert!(!items.is_empty());
}

#[test]
fn empty_cli_discovery_reports_warning() {
    let mut items = Vec::<DiagItem>::new();

    append_cli_tool_items(&[], "cli-tools", &mut items);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].severity, Severity::Warn);
    assert_eq!(items[0].message, "No CLI tools found in PATH");
}

#[test]
fn discovered_cli_tools_report_version_and_summary() {
    let discovered = vec![
        DiscoveredCli {
            name: "demo".to_string(),
            path: PathBuf::from("/tmp/demo"),
            version: Some(format!("{}-tail", "v".repeat(80))),
            category: CliCategory::Build,
        },
        DiscoveredCli {
            name: "plain".to_string(),
            path: PathBuf::from("/tmp/plain"),
            version: None,
            category: CliCategory::Language,
        },
    ];
    let mut items = Vec::<DiagItem>::new();

    append_cli_tool_items(&discovered, "cli-tools", &mut items);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].severity, Severity::Ok);
    assert!(items[0].message.contains("demo (Build) — "));
    assert!(!items[0].message.contains("-tail"));
    assert_eq!(items[1].message, "plain (Language) — unknown version");
    assert_eq!(items[2].message, "2 CLI tools discovered");
}

#[test]
fn discovered_cli_tool_preserves_short_version_and_category() {
    let discovered = vec![DiscoveredCli {
        name: "git".to_string(),
        path: PathBuf::from("/tmp/git"),
        version: Some("git version 2.45.1".to_string()),
        category: CliCategory::VersionControl,
    }];
    let mut items = Vec::<DiagItem>::new();

    append_cli_tool_items(&discovered, "custom-cli", &mut items);

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].category, "custom-cli");
    assert_eq!(items[0].message, "git (Version Control) — git version 2.45.1");
    assert_eq!(items[1].category, "custom-cli");
    assert_eq!(items[1].message, "1 CLI tools discovered");
}

#[test]
fn explicit_empty_cli_version_is_not_reported_as_unknown() {
    let discovered = vec![DiscoveredCli {
        name: "empty-version".to_string(),
        path: PathBuf::from("/tmp/empty-version"),
        version: Some(String::new()),
        category: CliCategory::Build,
    }];
    let mut items = Vec::<DiagItem>::new();

    append_cli_tool_items(&discovered, "cli-tools", &mut items);

    assert_eq!(items[0].message, "empty-version (Build) — ");
    assert!(!items[0].message.contains("unknown version"));
}

#[test]
fn cli_tool_items_append_after_existing_diagnostics() {
    let discovered = vec![DiscoveredCli {
        name: "cargo".to_string(),
        path: PathBuf::from("/tmp/cargo"),
        version: None,
        category: CliCategory::PackageManager,
    }];
    let mut items = vec![DiagItem::error("existing", "keep me")];

    append_cli_tool_items(&discovered, "cli-tools", &mut items);

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].severity, Severity::Error);
    assert_eq!(items[0].category, "existing");
    assert_eq!(items[0].message, "keep me");
    assert_eq!(items[1].message, "cargo (Package Manager) — unknown version");
    assert_eq!(items[2].message, "1 CLI tools discovered");
}
