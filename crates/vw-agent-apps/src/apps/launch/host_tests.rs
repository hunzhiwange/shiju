use super::{host, request::LaunchMaterial};
use std::process::Command;

#[test]
fn cc_apps_host_launch_uses_authorized_home_without_parent_secrets_or_hooks() {
    if std::env::var_os("SHIJU_HOST_LAUNCH_FIXTURE").is_none() {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "apps::launch::host_tests::cc_apps_host_launch_uses_authorized_home_without_parent_secrets_or_hooks", "--nocapture"])
            .env("SHIJU_HOST_LAUNCH_FIXTURE", "1")
            .env("HOME", "/fixture-untrusted-home")
            .env("XDG_CONFIG_HOME", "/fixture-untrusted-config")
            .env("OPENAI_API_KEY", "fixture-private-token")
            .env("PI_CODING_AGENT_DIR", "/fixture-untrusted-profile")
            .env("NODE_OPTIONS", "fixture-untrusted-hook")
            .status().unwrap();
        assert!(status.success());
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    let material = LaunchMaterial {
        program: "/bin/sh".into(),
        cwd: home.clone(),
        skip_permissions: false,
        args: vec![
            "-c".into(),
            r#"
test "$HOME" = "$PWD" || exit 10
test "$XDG_CONFIG_HOME" = "$HOME/.config" || exit 11
test "$XDG_CACHE_HOME" = "$HOME/.cache" || exit 12
test "$XDG_DATA_HOME" = "$HOME/.local/share" || exit 13
test "$XDG_STATE_HOME" = "$HOME/.local/state" || exit 14
test -z "$OPENAI_API_KEY$PI_CODING_AGENT_DIR$NODE_OPTIONS" || exit 15
printf '%s' "$HOME" > observed-home
"#
            .into(),
        ],
    };
    assert!(host::command(material, &home, &[]).unwrap().status().unwrap().success());
    assert_eq!(
        std::fs::read_to_string(home.join("observed-home")).unwrap(),
        home.to_str().unwrap()
    );
    assert_eq!(std::env::var("HOME").unwrap(), "/fixture-untrusted-home");
    assert_eq!(std::env::var("OPENAI_API_KEY").unwrap(), "fixture-private-token");
}
