//! 验证服务环境渲染、XML 转义与外部命令输出错误处理。

use super::common::{
    build_launchd_env_vars, build_systemd_env_vars, run_capture, run_checked, xml_escape,
};
use std::ffi::OsString;
use std::process::Command;
use std::sync::Mutex;

const SERVICE_ENV_NAMES: &[&str] = &[
    "GEMINI_API_KEY",
    "GEMINI_CLI_CLIENT_ID",
    "GEMINI_CLI_CLIENT_SECRET",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
];

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard(Vec<(&'static str, Option<OsString>)>);

impl EnvGuard {
    fn clear() -> Self {
        let saved = SERVICE_ENV_NAMES
            .iter()
            .map(|&name| (name, std::env::var_os(name)))
            .collect::<Vec<_>>();
        for &name in SERVICE_ENV_NAMES {
            // SAFETY: these tests serialize all mutations to service-related env vars.
            unsafe {
                std::env::remove_var(name);
            }
        }
        Self(saved)
    }

    fn set(name: &str, value: &str) {
        // SAFETY: callers hold ENV_LOCK, so concurrent test env mutation is avoided here.
        unsafe {
            std::env::set_var(name, value);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..) {
            // SAFETY: EnvGuard is only used while ENV_LOCK is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}

fn with_clean_service_env<T>(f: impl FnOnce() -> T) -> T {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _guard = EnvGuard::clear();
    f()
}

#[test]
fn xml_escape_replaces_reserved_xml_characters_and_keeps_plain_text() {
    assert_eq!(xml_escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    assert_eq!(xml_escape("plain_text-123"), "plain_text-123");
    assert_eq!(xml_escape(""), "");
}

#[test]
fn env_builders_return_empty_strings_without_service_environment() {
    with_clean_service_env(|| {
        assert_eq!(build_launchd_env_vars(), "");
        assert_eq!(build_systemd_env_vars(), "");
    });
}

#[test]
fn launchd_env_builder_xml_escapes_non_empty_values_and_skips_empty_values() {
    with_clean_service_env(|| {
        EnvGuard::set("OPENAI_API_KEY", "sk<&>\"'");
        EnvGuard::set("ANTHROPIC_API_KEY", "");

        let block = build_launchd_env_vars();

        assert!(block.starts_with("\n  <key>EnvironmentVariables</key>"));
        assert!(block.contains("<key>OPENAI_API_KEY</key>"));
        assert!(block.contains("<string>sk&lt;&amp;&gt;&quot;&apos;</string>"));
        assert!(!block.contains("ANTHROPIC_API_KEY"));
    });
}

#[test]
fn launchd_env_builder_preserves_service_env_order_for_multiple_values() {
    with_clean_service_env(|| {
        EnvGuard::set("GEMINI_API_KEY", "gemini");
        EnvGuard::set("OPENROUTER_API_KEY", "router");

        let block = build_launchd_env_vars();

        assert_eq!(
            block,
            "\n  <key>EnvironmentVariables</key>\n  <dict>\n    <key>GEMINI_API_KEY</key>\n    <string>gemini</string>\n    <key>OPENROUTER_API_KEY</key>\n    <string>router</string>\n  </dict>"
        );
    });
}

#[test]
fn systemd_env_builder_includes_non_empty_values_and_trailing_newline() {
    with_clean_service_env(|| {
        EnvGuard::set("OPENAI_API_KEY", "sk-test");
        EnvGuard::set("ANTHROPIC_API_KEY", "");

        let block = build_systemd_env_vars();

        assert_eq!(block, "Environment=\"OPENAI_API_KEY=sk-test\"\n");
    });
}

#[test]
fn systemd_env_builder_preserves_service_env_order_for_multiple_values() {
    with_clean_service_env(|| {
        EnvGuard::set("GEMINI_CLI_CLIENT_ID", "client-id");
        EnvGuard::set("GEMINI_CLI_CLIENT_SECRET", "client-secret");
        EnvGuard::set("OPENROUTER_API_KEY", "router");

        let block = build_systemd_env_vars();

        assert_eq!(
            block,
            "Environment=\"GEMINI_CLI_CLIENT_ID=client-id\"\nEnvironment=\"GEMINI_CLI_CLIENT_SECRET=client-secret\"\nEnvironment=\"OPENROUTER_API_KEY=router\"\n"
        );
    });
}

#[cfg(not(target_os = "windows"))]
#[test]
fn run_checked_accepts_success_and_reports_stderr_on_failure() {
    run_checked(Command::new("sh").args(["-lc", "exit 0"])).unwrap();

    let err = run_checked(Command::new("sh").args(["-lc", "echo nope >&2; exit 17"]))
        .expect_err("non-zero status should error");
    assert!(err.to_string().contains("Command failed: nope"));
}

#[cfg(target_os = "windows")]
#[test]
fn run_checked_accepts_success_and_reports_stderr_on_failure_windows() {
    run_checked(Command::new("cmd").args(["/C", "exit /b 0"])).unwrap();

    let err = run_checked(Command::new("cmd").args(["/C", "echo nope 1>&2 & exit /b 17"]))
        .expect_err("non-zero status should error");
    assert!(err.to_string().contains("Command failed: nope"));
}

#[test]
fn run_checked_reports_spawn_failures() {
    let mut command = Command::new("/definitely/not/a/vibewindow-command");
    let err = run_checked(&mut command).expect_err("missing command should fail to spawn");
    assert!(err.to_string().contains("Failed to spawn command"));
}

#[test]
fn run_capture_reports_spawn_failures() {
    let mut command = Command::new("/definitely/not/a/vibewindow-command");
    let err = run_capture(&mut command).expect_err("missing command should fail to spawn");
    assert!(err.to_string().contains("Failed to spawn command"));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn run_capture_prefers_stdout_and_falls_back_to_stderr() {
    let stdout = run_capture(Command::new("sh").args(["-lc", "echo out; echo err >&2"]))
        .expect("stdout capture should succeed");
    assert_eq!(stdout.trim(), "out");

    let stderr = run_capture(Command::new("sh").args(["-lc", "echo warn >&2"]))
        .expect("stderr fallback should succeed");
    assert_eq!(stderr.trim(), "warn");

    let empty = run_capture(Command::new("sh").args(["-lc", "true"]))
        .expect("empty command output should succeed");
    assert_eq!(empty, "");
}

#[cfg(target_os = "windows")]
#[test]
fn run_capture_prefers_stdout_and_falls_back_to_stderr_windows() {
    let stdout = run_capture(Command::new("cmd").args(["/C", "echo out"]))
        .expect("stdout capture should succeed");
    assert_eq!(stdout.trim(), "out");

    let stderr = run_capture(Command::new("cmd").args(["/C", "echo warn 1>&2"]))
        .expect("stderr fallback should succeed");
    assert_eq!(stderr.trim(), "warn");

    let empty = run_capture(Command::new("cmd").args(["/C", "exit /b 0"]))
        .expect("empty command output should succeed");
    assert_eq!(empty, "");
}
