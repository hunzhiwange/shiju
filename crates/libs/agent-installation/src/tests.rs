// 验证安装类型、升级错误及命令输出附加字段的稳定契约。

// 错误展示只暴露约定信息，命令输出中的保留键由最终结果统一覆盖。
use super::*;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

#[test]
fn method_as_str_is_stable() {
    let cases = [
        (Method::Curl, "curl"),
        (Method::Npm, "npm"),
        (Method::Yarn, "yarn"),
        (Method::Pnpm, "pnpm"),
        (Method::Bun, "bun"),
        (Method::Brew, "brew"),
        (Method::Scoop, "scoop"),
        (Method::Choco, "choco"),
        (Method::Unknown, "unknown"),
    ];

    for (method, name) in cases {
        assert_eq!(method.as_str(), name);
    }
}

#[test]
fn upgrade_failed_error_displays_stderr_only() {
    let err = UpgradeFailedError { stderr: "upgrade failed".into() };
    assert_eq!(err.to_string(), "upgrade failed");
}

#[test]
fn local_version_and_channel_defaults_are_stable() {
    unsafe {
        std::env::remove_var("VIBEWINDOW_VERSION");
        std::env::remove_var("VIBEWINDOW_CHANNEL");
    }

    assert_eq!(version(), "local");
    assert_eq!(channel(), "local");
    assert!(is_local());
    assert!(is_preview());
    assert!(user_agent().starts_with("vibewindow/local/local/"));
}

#[test]
fn upgrade_rejects_unsupported_install_methods_without_running_commands() {
    let unsupported = upgrade(Method::Yarn, "1.2.3").expect_err("yarn cannot upgrade");
    assert_eq!(unsupported.to_string(), "Unknown method: yarn");
    let unsupported = upgrade(Method::Unknown, "1.2.3").expect_err("unknown cannot upgrade");
    assert_eq!(unsupported.to_string(), "Unknown method: unknown");
}

#[test]
fn run_helpers_cover_empty_and_failing_commands() {
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(run_quiet(Vec::new()), "");

    let err = run_cmd(&[], None).expect_err("missing command should be rejected");
    assert_eq!(err.to_string(), "missing command");

    let shell = run_shell("printf stdout; printf stderr >&2; exit 7", None).unwrap();
    assert_eq!(String::from_utf8_lossy(&shell.stdout), "stdout");
    assert_eq!(String::from_utf8_lossy(&shell.stderr), "stderr");
    assert!(!shell.status.success());
}

#[test]
fn extra_maps_include_base_pairs_and_output_streams() {
    #[cfg(unix)]
    let status = std::process::ExitStatus::from_raw(5 << 8);
    #[cfg(windows)]
    let status = std::process::ExitStatus::from_raw(5);

    let output = Output { status, stdout: b"out".to_vec(), stderr: b"err".to_vec() };
    let map = extra_from_output(&output, [("method", Value::String("npm".into()))]);

    assert_eq!(map.get("method"), Some(&Value::String("npm".into())));
    assert_eq!(map.get("stdout"), Some(&Value::String("out".into())));
    assert_eq!(map.get("stderr"), Some(&Value::String("err".into())));
    assert_eq!(map.get("exit_code"), Some(&Value::Number(5.into())));
}

#[test]
fn error_display_delegates_to_inner_errors() {
    let json_err = serde_json::from_str::<serde_json::Value>("{bad").unwrap_err();
    let err = Error::from(json_err);
    assert!(!err.to_string().is_empty());

    let utf8_err = String::from_utf8(vec![0xff]).unwrap_err();
    let err = Error::from(utf8_err);
    assert!(err.to_string().contains("invalid utf-8"));
}

#[test]
fn error_from_io_preserves_kind_and_message() {
    let err = Error::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "installation access denied",
    ));

    assert_eq!(err.to_string(), "installation access denied");
    assert!(matches!(
        err,
        Error::Io(inner) if inner.kind() == std::io::ErrorKind::PermissionDenied
    ));
}

#[test]
fn error_display_covers_upgrade_and_unsupported_variants() {
    let upgrade = Error::UpgradeFailed(UpgradeFailedError { stderr: "upgrade stderr".into() });
    assert_eq!(upgrade.to_string(), "upgrade stderr");

    let unsupported = Error::Unsupported("automatic upgrade unavailable".into());
    assert_eq!(unsupported.to_string(), "automatic upgrade unavailable");
}

#[test]
fn extra_accepts_empty_input() {
    assert!(extra::<0>([]).is_empty());
}

#[test]
fn extra_uses_the_last_value_for_duplicate_keys() {
    let map = extra([
        ("channel", Value::String("preview".into())),
        ("channel", Value::String("latest".into())),
        ("enabled", Value::Bool(true)),
    ]);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("channel"), Some(&Value::String("latest".into())));
    assert_eq!(map.get("enabled"), Some(&Value::Bool(true)));
}

#[test]
fn extra_from_output_replaces_reserved_keys_and_decodes_lossily() {
    #[cfg(unix)]
    let status = std::process::ExitStatus::from_raw(0);
    #[cfg(windows)]
    let status = std::process::ExitStatus::from_raw(0);

    let output = Output { status, stdout: vec![b'o', 0xff], stderr: vec![0xfe] };
    let map = extra_from_output(
        &output,
        [
            ("stdout", Value::String("stale stdout".into())),
            ("stderr", Value::String("stale stderr".into())),
            ("exit_code", Value::Number(99.into())),
            ("method", Value::String("npm".into())),
        ],
    );

    assert_eq!(map.get("stdout"), Some(&Value::String("o�".into())));
    assert_eq!(map.get("stderr"), Some(&Value::String("�".into())));
    assert_eq!(map.get("exit_code"), Some(&Value::Number(0.into())));
    assert_eq!(map.get("method"), Some(&Value::String("npm".into())));
}

#[cfg(unix)]
#[test]
fn extra_from_output_uses_negative_one_when_exit_code_is_unavailable() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(9),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };

    let map = extra_from_output(&output, []);

    assert_eq!(map.get("exit_code"), Some(&Value::Number((-1).into())));
}

#[test]
fn run_quiet_returns_empty_for_an_invalid_program_name() {
    assert_eq!(run_quiet(vec!["invalid\0program"]), "");
}
