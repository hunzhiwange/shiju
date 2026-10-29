use super::*;

#[test]
fn shell_environment_removes_explicit_secret_overrides() {
    let mut command = tokio::process::Command::new("unused-test-program");
    command.env("VW_SHELL_TEST_API_TOKEN", "must-not-leak");
    apply_allowed_shell_environment(&mut command, &SecurityPolicy::default());
    assert!(!command.as_std().get_envs().any(|(name, _)| name == "VW_SHELL_TEST_API_TOKEN"));
}

#[cfg(windows)]
#[test]
fn shell_environment_preserves_windows_runtime_locations_without_case_duplicates() {
    let security = SecurityPolicy {
        shell_env_passthrough: vec!["Path".into(), "SystemRoot".into(), "BAD-NAME".into()],
        ..SecurityPolicy::default()
    };
    let allowed = collect_allowed_shell_env_vars(&security);
    for required in ["SYSTEMROOT", "USERPROFILE", "COMSPEC", "PATHEXT", "PROGRAMFILES(X86)", "TEMP"]
    {
        assert_eq!(
            allowed.iter().filter(|name| name.eq_ignore_ascii_case(required)).count(),
            1,
            "{required}"
        );
    }
    assert_eq!(allowed.iter().filter(|name| name.eq_ignore_ascii_case("PATH")).count(), 1);
    assert!(!allowed.iter().any(|name| name == "BAD-NAME"));
    let mut command = tokio::process::Command::new("unused-test-program");
    apply_allowed_shell_environment(&mut command, &security);
    let configured = command.as_std().get_envs().collect::<std::collections::HashMap<_, _>>();
    for name in PLATFORM_SAFE_ENV_VARS {
        if let Some(value) = allowed_shell_env_value(name) {
            assert_eq!(
                configured.get(std::ffi::OsStr::new(name)).and_then(|value| *value),
                Some(std::ffi::OsStr::new(&value)),
                "{name}"
            );
        }
    }
}

#[cfg(not(windows))]
#[test]
fn shell_environment_does_not_add_windows_variables_on_unix() {
    let allowed = collect_allowed_shell_env_vars(&SecurityPolicy::default());
    assert!(
        !allowed.iter().any(
            |name| ["SYSTEMROOT", "USERPROFILE", "COMSPEC", "PATHEXT"].contains(&name.as_str())
        )
    );
}
