use super::*;
use serde::Deserialize;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(unix)]
fn write_executable(dir: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fake executable");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("chmod fake executable");
}

#[cfg(unix)]
fn prepend_path(dir: &Path) -> std::ffi::OsString {
    let old_path = std::env::var_os("PATH");
    let mut paths = vec![dir.to_path_buf()];
    if let Some(old_path) = old_path {
        paths.extend(std::env::split_paths(&old_path));
    }
    std::env::join_paths(paths).expect("join PATH")
}

fn one_shot_json_server(status: &str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let addr = listener.local_addr().expect("server addr");
    let status = status.to_string();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    });
    format!("http://{addr}")
}

#[test]
fn info_round_trips_through_json() {
    let info = Info { version: "1.2.3".to_string(), latest: "1.2.4".to_string() };

    let encoded = serde_json::to_value(&info).expect("serialize info");
    assert_eq!(encoded, json!({ "version": "1.2.3", "latest": "1.2.4" }));

    let decoded: Info = serde_json::from_value(encoded).expect("deserialize info");
    assert_eq!(decoded.version, "1.2.3");
    assert_eq!(decoded.latest, "1.2.4");
}

#[test]
fn event_definitions_are_stable() {
    assert_eq!(event::UPDATED.r#type, "installation.updated");
    assert_eq!(event::UPDATE_AVAILABLE.r#type, "installation.update-available");
}

#[test]
fn run_shell_applies_environment_overrides() {
    let _guard = env_lock();
    let output = run_shell(
        "printf '%s' \"$VW_INSTALLATION_TEST_VALUE\"",
        Some(&[("VW_INSTALLATION_TEST_VALUE", "ok")]),
    )
    .expect("shell output");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).expect("utf8 stdout"), "ok");
}

#[test]
fn run_cmd_surfaces_missing_binary_as_io_error() {
    let err = run_cmd(&["definitely-not-a-vibewindow-test-command"], None)
        .expect_err("missing command should fail");

    assert!(matches!(err, Error::Io(_)));
    assert!(!err.to_string().is_empty());
}

#[test]
fn run_cmd_applies_environment_overrides() {
    let output = run_cmd(
        &["sh", "-c", "printf '%s' \"$VW_INSTALLATION_CMD_ENV\""],
        Some(&[("VW_INSTALLATION_CMD_ENV", "from-run-cmd")]),
    )
    .expect("run command");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).expect("utf8 stdout"), "from-run-cmd");
}

#[cfg(unix)]
#[test]
fn method_detects_package_manager_from_fake_global_list() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(dir.path(), "npm", "#!/bin/sh\nprintf 'vibewindow-ai@1.2.3\\n'\n");
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    assert_eq!(method(), Method::Npm);

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn method_returns_unknown_when_no_package_manager_reports_installation() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    unsafe {
        std::env::set_var("PATH", dir.path());
    }

    assert_eq!(method(), Method::Unknown);

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn npm_registry_trims_trailing_slash_from_fake_npm() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "npm",
        "#!/bin/sh\nif [ \"$1\" = config ]; then printf 'https://registry.example.test/\\n'; fi\n",
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    assert_eq!(npm_registry().expect("registry"), "https://registry.example.test");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn npm_registry_uses_default_when_fake_npm_returns_blank() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(dir.path(), "npm", "#!/bin/sh\nprintf '\\n'\n");
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    assert_eq!(npm_registry().expect("default registry"), "https://registry.npmjs.org");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn info_uses_detected_npm_installation_and_local_registry() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_channel = std::env::var_os("VIBEWINDOW_CHANNEL");
    let old_version = std::env::var_os("VIBEWINDOW_VERSION");
    let old_registry = std::env::var_os("VW_INSTALLATION_TEST_REGISTRY");
    let registry = one_shot_json_server("200 OK", r#"{"version":"4.5.6"}"#);
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "npm",
        r#"#!/bin/sh
if [ "$1" = "config" ]; then
  printf '%s/\n' "$VW_INSTALLATION_TEST_REGISTRY"
else
  printf 'vibewindow-ai@4.0.0\n'
fi
"#,
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VIBEWINDOW_CHANNEL", "beta");
        std::env::set_var("VIBEWINDOW_VERSION", "4.0.0");
        std::env::set_var("VW_INSTALLATION_TEST_REGISTRY", registry);
    }

    let info = info().expect("installation info");
    assert_eq!(info.version, "4.0.0");
    assert_eq!(info.latest, "4.5.6");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_channel) = old_channel {
            std::env::set_var("VIBEWINDOW_CHANNEL", old_channel);
        } else {
            std::env::remove_var("VIBEWINDOW_CHANNEL");
        }
        if let Some(old_version) = old_version {
            std::env::set_var("VIBEWINDOW_VERSION", old_version);
        } else {
            std::env::remove_var("VIBEWINDOW_VERSION");
        }
        if let Some(old_registry) = old_registry {
            std::env::set_var("VW_INSTALLATION_TEST_REGISTRY", old_registry);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_REGISTRY");
        }
    }
}

#[cfg(unix)]
#[test]
fn get_brew_formula_uses_core_when_tap_is_absent_and_defaults_when_unlisted() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "brew",
        r#"#!/bin/sh
if [ "$*" = "list --formula vibewindow" ]; then
  printf 'vibewindow\n'
fi
"#,
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    assert_eq!(get_brew_formula().expect("core formula"), "vibewindow");

    write_executable(dir.path(), "brew", "#!/bin/sh\nexit 1\n");
    assert_eq!(get_brew_formula().expect("fallback formula"), "vibewindow");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn latest_reports_missing_stable_version_for_tap_formula() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "brew",
        r#"#!/bin/sh
case "$*" in
  "list --formula anomalyco/tap/vibewindow") printf 'vibewindow\n' ;;
  "info --json=v2 anomalyco/tap/vibewindow") printf '{"formulae":[{"versions":{}}]}\n' ;;
  *) exit 1 ;;
esac
"#,
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    let err = latest(Some(Method::Brew)).expect_err("missing stable version");
    assert_eq!(
        err.to_string(),
        "Could not detect version for tap formula: anomalyco/tap/vibewindow"
    );

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn latest_reads_npm_dist_tag_from_configured_registry() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_channel = std::env::var_os("VIBEWINDOW_CHANNEL");
    let old_registry = std::env::var_os("VW_INSTALLATION_TEST_REGISTRY");
    let registry = one_shot_json_server("200 OK", r#"{"version":"7.8.9"}"#);
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "npm",
        "#!/bin/sh\nif [ \"$1\" = config ]; then printf '%s/\\n' \"$VW_INSTALLATION_TEST_REGISTRY\"; fi\n",
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VIBEWINDOW_CHANNEL", "beta");
        std::env::set_var("VW_INSTALLATION_TEST_REGISTRY", registry);
    }

    assert_eq!(latest(Some(Method::Npm)).expect("latest npm version"), "7.8.9");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_channel) = old_channel {
            std::env::set_var("VIBEWINDOW_CHANNEL", old_channel);
        } else {
            std::env::remove_var("VIBEWINDOW_CHANNEL");
        }
        if let Some(old_registry) = old_registry {
            std::env::set_var("VW_INSTALLATION_TEST_REGISTRY", old_registry);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_REGISTRY");
        }
    }
}

#[cfg(unix)]
#[test]
fn latest_reads_brew_tap_formula_json_from_fake_brew() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(
        dir.path(),
        "brew",
        r#"#!/bin/sh
case "$*" in
  "list --formula anomalyco/tap/vibewindow") printf 'vibewindow\n' ;;
  "info --json=v2 anomalyco/tap/vibewindow") printf '{"formulae":[{"versions":{"stable":"9.8.7"}}]}\n' ;;
  *) exit 1 ;;
esac
"#,
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    assert_eq!(latest(Some(Method::Brew)).expect("brew latest"), "9.8.7");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(unix)]
#[test]
fn upgrade_runs_package_manager_command_and_records_failure_stderr() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_args = std::env::var_os("VW_INSTALLATION_TEST_ARGS");
    let dir = tempfile::tempdir().expect("tempdir");
    let args_path = dir.path().join("npm.args");
    write_executable(
        dir.path(),
        "npm",
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$VW_INSTALLATION_TEST_ARGS\"\n",
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VW_INSTALLATION_TEST_ARGS", &args_path);
    }

    upgrade(Method::Npm, "1.2.3").expect("npm upgrade");
    assert_eq!(
        std::fs::read_to_string(&args_path).expect("args"),
        "install\n-g\nvibewindow-ai@1.2.3\n"
    );

    write_executable(dir.path(), "npm", "#!/bin/sh\nprintf 'package failed' >&2\nexit 9\n");
    let err = upgrade(Method::Npm, "2.0.0").expect_err("npm failure");
    assert!(matches!(err, Error::UpgradeFailed(_)));
    assert_eq!(err.to_string(), "package failed");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_args) = old_args {
            std::env::set_var("VW_INSTALLATION_TEST_ARGS", old_args);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_ARGS");
        }
    }
}

#[cfg(unix)]
#[test]
fn upgrade_runs_package_manager_variants_that_use_direct_commands() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_log = std::env::var_os("VW_INSTALLATION_TEST_LOG");
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("commands.log");
    for name in ["pnpm", "bun", "scoop"] {
        write_executable(
            dir.path(),
            name,
            &format!(
                "#!/bin/sh\nprintf '{} %s\\n' \"$*\" >> \"$VW_INSTALLATION_TEST_LOG\"\n",
                name
            ),
        );
    }
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VW_INSTALLATION_TEST_LOG", &log_path);
    }

    upgrade(Method::Pnpm, "1.2.0").expect("pnpm upgrade");
    upgrade(Method::Bun, "1.3.0").expect("bun upgrade");
    upgrade(Method::Scoop, "1.4.0").expect("scoop upgrade");

    let log = std::fs::read_to_string(&log_path).expect("command log");
    assert!(log.contains("pnpm install -g vibewindow-ai@1.2.0"));
    assert!(log.contains("bun install -g vibewindow-ai@1.3.0"));
    assert!(log.contains("scoop install vibewindow@1.4.0"));

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_log) = old_log {
            std::env::set_var("VW_INSTALLATION_TEST_LOG", old_log);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_LOG");
        }
    }
}

#[cfg(unix)]
#[test]
fn upgrade_runs_brew_core_command() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_log = std::env::var_os("VW_INSTALLATION_TEST_LOG");
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("brew.log");
    write_executable(
        dir.path(),
        "brew",
        r#"#!/bin/sh
printf 'brew %s\n' "$*" >> "$VW_INSTALLATION_TEST_LOG"
case "$*" in
  "list --formula anomalyco/tap/vibewindow") exit 1 ;;
  "list --formula vibewindow") printf 'vibewindow\n' ;;
esac
"#,
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VW_INSTALLATION_TEST_LOG", &log_path);
    }

    upgrade(Method::Brew, "2.0.0").expect("core brew upgrade");

    let log = std::fs::read_to_string(&log_path).expect("brew log");
    assert!(log.contains("brew upgrade vibewindow"));

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_log) = old_log {
            std::env::set_var("VW_INSTALLATION_TEST_LOG", old_log);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_LOG");
        }
    }
}

#[cfg(unix)]
#[test]
fn upgrade_choco_failure_uses_elevation_message() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let dir = tempfile::tempdir().expect("tempdir");
    write_executable(dir.path(), "choco", "#!/bin/sh\nprintf 'raw stderr' >&2\nexit 5\n");
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
    }

    let err = upgrade(Method::Choco, "1.0.0").expect_err("choco failure");
    assert_eq!(err.to_string(), "not running from an elevated command shell");

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[test]
fn http_get_json_decodes_success_and_surfaces_status_errors() {
    #[derive(Debug, Deserialize)]
    struct Payload {
        value: String,
    }

    let ok_url = one_shot_json_server("200 OK", r#"{"value":"decoded"}"#);
    let payload: Payload = http_get_json(&ok_url, Some(&[("X-Test", "yes")])).expect("json");
    assert_eq!(payload.value, "decoded");

    let err_url = one_shot_json_server("500 Internal Server Error", r#"{"error":"nope"}"#);
    let err = http_get_json::<Payload>(&err_url, None).expect_err("status error");
    assert!(matches!(err, Error::Http(_)));
    assert!(!err.to_string().is_empty());
}

#[cfg(unix)]
#[test]
fn upgrade_curl_passes_install_script_and_version_to_shell() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_log = std::env::var_os("VW_INSTALLATION_TEST_LOG");
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("bash.log");
    write_executable(
        dir.path(),
        "bash",
        "#!/bin/sh\nprintf 'args=%s\\nversion=%s\\n' \"$*\" \"$VERSION\" > \"$VW_INSTALLATION_TEST_LOG\"\n",
    );
    unsafe {
        std::env::set_var("PATH", prepend_path(dir.path()));
        std::env::set_var("VW_INSTALLATION_TEST_LOG", &log_path);
    }

    upgrade(Method::Curl, "3.2.1").expect("curl upgrade");

    let log = std::fs::read_to_string(&log_path).expect("bash log");
    assert!(log.contains("curl -fsSL https://vibewindow.huododo.com/install | bash"));
    assert!(log.contains("version=3.2.1"));

    unsafe {
        if let Some(old_path) = old_path {
            std::env::set_var("PATH", old_path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(old_log) = old_log {
            std::env::set_var("VW_INSTALLATION_TEST_LOG", old_log);
        } else {
            std::env::remove_var("VW_INSTALLATION_TEST_LOG");
        }
    }
}
