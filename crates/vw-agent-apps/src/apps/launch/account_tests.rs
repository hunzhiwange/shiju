use super::{account::prepare_account_launch, request::LaunchMaterial};
use crate::auth::credential_file::{AccountCredential, CredentialFileStore};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

#[test]
fn cc_apps_account_launch_isolation_scoped_storage_process_and_cleanup() {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let tenant = TenantScope::new("platform", "company").unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    store
        .save(
            &tenant,
            "host",
            AppId::Claude,
            "account",
            &AccountCredential::ClaudeToken("fixture-official-token".into()),
        )
        .unwrap();
    drop(store);
    let store = CredentialFileStore::new(root.path()).unwrap();
    assert!(
        store
            .read(&TenantScope::new("platform", "other").unwrap(), "host", AppId::Claude, "account")
            .is_err()
    );
    assert!(store.read(&tenant, "other-host", AppId::Claude, "account").is_err());
    assert!(store.read(&tenant, "host", AppId::Codex, "account").is_err());
    assert!(store.read(&tenant, "host", AppId::Claude, "missing").is_err());
    let script = root.path().join("claude");
    std::fs::write(&script, "#!/bin/sh\ntest \"$CLAUDE_CODE_OAUTH_TOKEN\" = fixture-official-token || exit 1\ntest -z \"$ANTHROPIC_API_KEY\" || exit 2\ntest -d \"$CLAUDE_CONFIG_DIR\" || exit 3\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    prepare_account_launch(
        AppId::Claude,
        store.read(&tenant, "host", AppId::Claude, "account").unwrap(),
        LaunchMaterial {
            program: script,
            cwd: root.path().into(),
            args: vec![],
            skip_permissions: false,
        },
        root.path(),
    )
    .unwrap()
    .run()
    .unwrap();
    let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!std::path::Path::new(&private).exists());
}

fn material(root: &std::path::Path, program: &str) -> LaunchMaterial {
    LaunchMaterial {
        program: root.join(program),
        cwd: root.into(),
        args: vec![],
        skip_permissions: false,
    }
}
fn executable(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}
fn codex_auth() -> AccountCredential {
    AccountCredential::CodexAuth(
        serde_json::json!({"auth_mode":"chatgpt", "last_refresh":"2026-01-01T00:00:00Z", "tokens":{"access_token":"fixture-access", "refresh_token":"fixture-refresh", "id_token":"eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJmaXh0dXJlLXVzZXIifQ.fixture", "account_id":"fixture-account"}}),
    )
}
#[test]
fn cc_apps_account_launch_isolation_codex_material_and_failure_cleanup() {
    let root = tempfile::tempdir().unwrap();
    executable(
        &root.path().join("codex"),
        "#!/bin/sh\ntest -f \"$CODEX_HOME/auth.json\" || exit 3\ntest -f \"$CODEX_HOME/config.toml\" || exit 4\ntest -z \"$OPENAI_API_KEY\" || exit 5\nprintf '%s' \"$CODEX_HOME\" > private-path\nexit 7\n",
    );
    assert!(
        prepare_account_launch(
            AppId::Codex,
            codex_auth(),
            material(root.path(), "codex"),
            root.path()
        )
        .unwrap()
        .run()
        .is_err()
    );
    let path = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!std::path::Path::new(&path).exists());
    // Discarding prepared material and a spawn failure both remove the private profile.
    let launch = prepare_account_launch(
        AppId::Codex,
        codex_auth(),
        material(root.path(), "codex"),
        root.path(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("shiju-account-"))
            .count(),
        1
    );
    drop(launch);
    let launch = prepare_account_launch(
        AppId::Codex,
        codex_auth(),
        material(root.path(), "codex"),
        root.path(),
    )
    .unwrap();
    std::fs::remove_file(root.path().join("codex")).unwrap();
    assert!(launch.run().is_err());
    assert!(
        !std::fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with("shiju-account-"))
    );
}
#[test]
fn cc_apps_account_launch_isolation_rejects_before_temporary_side_effects() {
    let root = tempfile::tempdir().unwrap();
    executable(&root.path().join("claude"), "#!/bin/sh\nexit 0\n");
    for app in [AppId::Agy, AppId::Gemini, AppId::Codex] {
        assert!(
            prepare_account_launch(
                app,
                AccountCredential::ClaudeToken("fixture".into()),
                material(root.path(), "claude"),
                root.path()
            )
            .is_err()
        );
    }
    for args in [vec!["--settings".into()], vec!["line\nbreak".into()], vec!["$(command)".into()]] {
        let mut input = material(root.path(), "claude");
        input.args = args;
        assert!(
            prepare_account_launch(
                AppId::Claude,
                AccountCredential::ClaudeToken("fixture".into()),
                input,
                root.path()
            )
            .is_err()
        );
    }
    let mut input = material(root.path(), "claude");
    input.skip_permissions = true;
    assert!(
        prepare_account_launch(
            AppId::Claude,
            AccountCredential::ClaudeToken("fixture".into()),
            input,
            root.path()
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[test]
fn cc_apps_account_launch_isolation_codex_project_override_refused() {
    let root = tempfile::tempdir().unwrap();
    executable(&root.path().join("codex"), "#!/bin/sh\nexit 0\n");
    std::fs::create_dir(root.path().join(".codex")).unwrap();
    std::fs::write(root.path().join(".codex/config.toml"), "model_provider='other'").unwrap();
    assert!(
        prepare_account_launch(
            AppId::Codex,
            codex_auth(),
            material(root.path(), "codex"),
            root.path()
        )
        .is_err()
    );
}
#[test]
fn cc_apps_account_launch_isolation_ambient_credentials_removed() {
    const CHILD: &str = "SHIJU_ACCOUNT_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "apps::launch::account_tests::cc_apps_account_launch_isolation_ambient_credentials_removed"]).env(CHILD, "1").env("ANTHROPIC_API_KEY", "ambient-fixture").env("OPENAI_API_KEY", "ambient-fixture").env("NODE_OPTIONS", "ambient-hook").status().unwrap().success());
        return;
    }
    let root = tempfile::tempdir().unwrap();
    executable(
        &root.path().join("claude"),
        "#!/bin/sh\ntest -z \"$ANTHROPIC_API_KEY$OPENAI_API_KEY$NODE_OPTIONS\" || exit 3\ntest \"$CLAUDE_CODE_OAUTH_TOKEN\" = fixture || exit 4\n",
    );
    prepare_account_launch(
        AppId::Claude,
        AccountCredential::ClaudeToken("fixture".into()),
        material(root.path(), "claude"),
        root.path(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(std::env::var("ANTHROPIC_API_KEY").unwrap(), "ambient-fixture");
}

#[test]
fn cc_apps_account_launch_isolation_codex_success_exact_auth_and_permissions() {
    let root = tempfile::tempdir().unwrap();
    executable(
        &root.path().join("codex"),
        "#!/bin/sh\ntest -z \"$OPENAI_API_KEY\" || exit 2\ncp \"$CODEX_HOME/auth.json\" observed-auth\nprintf '%s' \"$CODEX_HOME\" > private-path\n",
    );
    let launch = prepare_account_launch(
        AppId::Codex,
        codex_auth(),
        material(root.path(), "codex"),
        root.path(),
    )
    .unwrap();
    let private = std::fs::read_dir(root.path())
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().starts_with("shiju-account-"))
        .unwrap()
        .path();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(private.metadata().unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(
            private.join("auth.json").metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    launch.run().unwrap();
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("observed-auth")).unwrap()).unwrap();
    assert_eq!(actual["tokens"]["account_id"], "fixture-account");
    assert_eq!(actual["tokens"]["access_token"], "fixture-access");
    assert!(!private.exists());
}
#[test]
fn cc_apps_account_launch_isolation_storage_distinct_scopes_and_rejected_material() {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let store = CredentialFileStore::new(root.path()).unwrap();
    let a = TenantScope::new("platform", "a").unwrap();
    let b = TenantScope::new("platform", "b").unwrap();
    for (tenant, token) in [(&a, "fixture-a"), (&b, "fixture-b")] {
        store
            .save(
                tenant,
                "host",
                AppId::Claude,
                "same",
                &AccountCredential::ClaudeToken(token.into()),
            )
            .unwrap();
    }
    for (tenant, token) in [(&a, "fixture-a"), (&b, "fixture-b")] {
        match store.read(tenant, "host", AppId::Claude, "same").unwrap() {
            AccountCredential::ClaudeToken(actual) => assert_eq!(actual, token),
            _ => panic!("wrong credential service"),
        }
    }
    assert!(
        store
            .save(
                &a,
                "host",
                AppId::Claude,
                "../escape",
                &AccountCredential::ClaudeToken("fixture".into())
            )
            .is_err()
    );
    assert!(
        store
            .save(&a, "host", AppId::Agy, "same", &AccountCredential::ClaudeToken("fixture".into()))
            .is_err()
    );
    assert!(
        store
            .save(
                &a,
                "host",
                AppId::Codex,
                "same",
                &AccountCredential::CodexAuth(serde_json::json!({"OPENAI_API_KEY":"fixture-key"}))
            )
            .is_err()
    );
    assert!(
        store
            .save(
                &a,
                "host",
                AppId::Claude,
                "same",
                &AccountCredential::ClaudeToken("line\nbreak".into())
            )
            .is_err()
    );
}

#[test]
fn cc_apps_account_launch_isolation_rotated_codex_auth_reaches_owner_on_nonzero_exit() {
    let root = tempfile::tempdir().unwrap();
    executable(
        &root.path().join("codex"),
        "#!/bin/sh\ncat > \"$CODEX_HOME/auth.json\" <<'AUTH'\n{\"auth_mode\":\"chatgpt\",\"last_refresh\":\"2026-01-02T00:00:00Z\",\"tokens\":{\"access_token\":\"fixture-rotated-access\",\"refresh_token\":\"fixture-rotated-refresh\",\"id_token\":\"eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJmaXh0dXJlLXVzZXIifQ.fixture\",\"account_id\":\"fixture-account\"}}\nAUTH\nprintf '%s' \"$CODEX_HOME\" > private-path\nexit 7\n",
    );
    let mut rotated = None;
    let result = prepare_account_launch(
        AppId::Codex,
        codex_auth(),
        material(root.path(), "codex"),
        root.path(),
    )
    .unwrap()
    .run_with_refresh(|credential| {
        rotated = Some(credential);
        Ok(())
    });
    assert!(result.is_err());
    let Some(AccountCredential::CodexAuth(auth)) = rotated else {
        panic!("rotated official credentials must reach their owner before cleanup")
    };
    assert_eq!(auth["tokens"]["refresh_token"], "fixture-rotated-refresh");
    let path = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!std::path::Path::new(&path).exists());
}
