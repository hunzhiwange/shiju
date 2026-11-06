use super::{provider::prepare_provider_launch, request::LaunchMaterial};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
};

pub(super) fn profile(protocol: ProviderProtocol) -> AppProviderWrite {
    AppProviderWrite {
        name: "fixture".into(),
        protocol,
        api_key: "fixture-key-only".into(),
        base_url: "https://provider.example.test/v1".into(),
        model: "fixture-model".into(),
    }
}

#[test]
fn cc_apps_provider_launch_isolation_private_process_success_and_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("claude");
    std::fs::write(&script, "#!/bin/sh\ntest \"$ANTHROPIC_API_KEY\" = fixture-key-only || exit 11\ntest \"$ANTHROPIC_BASE_URL\" = https://provider.example.test/v1 || exit 12\ntest -f \"$CLAUDE_CONFIG_DIR/settings.json\" || exit 13\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let before = std::env::var_os("ANTHROPIC_API_KEY");
    let launch = prepare_provider_launch(
        AppId::Claude,
        &profile(ProviderProtocol::Anthropic),
        LaunchMaterial {
            program: script,
            cwd: root.path().into(),
            args: vec!["hello".into()],
            skip_permissions: false,
        },
        root.path(),
    )
    .unwrap();
    launch.run().unwrap();
    let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!std::path::Path::new(&private).exists());
    assert_eq!(std::env::var_os("ANTHROPIC_API_KEY"), before);
}

fn material(root: &std::path::Path, program: &str) -> LaunchMaterial {
    LaunchMaterial {
        program: root.join(program),
        cwd: root.into(),
        args: vec!["中文 prompt".into()],
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
#[test]
fn cc_apps_provider_launch_isolation_codex_gemini_agy_use_actual_programs() {
    for (app, name, protocol, body) in [
        (
            AppId::Codex,
            "codex",
            ProviderProtocol::OpenaiResponses,
            "test \"$SHIJU_PROVIDER_API_KEY\" = fixture-key-only || exit 11\ntest -f \"$CODEX_HOME/config.toml\" || exit 12\ngrep -q 'wire_api = \"responses\"' \"$CODEX_HOME/config.toml\" || exit 13\ntest -z \"$OPENAI_API_KEY\" || exit 14\nprintf '%s' \"$CODEX_HOME\" > private-path\n",
        ),
        (
            AppId::Gemini,
            "gemini",
            ProviderProtocol::Gemini,
            "test \"$GEMINI_API_KEY\" = fixture-key-only || exit 11\ntest -f \"$GEMINI_CLI_HOME/.gemini/settings.json\" || exit 12\ntest -z \"$GOOGLE_API_KEY$GOOGLE_APPLICATION_CREDENTIALS\" || exit 13\nprintf '%s' \"$GEMINI_CLI_HOME\" > private-path\n",
        ),
        (
            AppId::Agy,
            "agy",
            ProviderProtocol::Gemini,
            "test \"$GEMINI_API_KEY\" = fixture-key-only || exit 11\ntest \"$1\" = --gemini_dir || exit 12\ntest -f \"$2/settings.json\" || exit 13\nprintf '%s' \"$2\" > private-path\n",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        executable(
            &root.path().join(name),
            &format!("#!/bin/sh\n{body}printf '%s\\n' \"$@\" > argv\npwd > cwd\n"),
        );
        prepare_provider_launch(app, &profile(protocol), material(root.path(), name), root.path())
            .unwrap()
            .run()
            .unwrap();
        let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
        assert!(!std::path::Path::new(&private).exists());
        let argv = std::fs::read_to_string(root.path().join("argv")).unwrap();
        assert!(argv.contains("中文 prompt"));
        assert!(!argv.contains("fixture-key-only"));
        assert_eq!(
            std::fs::read_to_string(root.path().join("cwd")).unwrap().trim(),
            root.path().canonicalize().unwrap().to_str().unwrap()
        );
    }
}
#[test]
fn cc_apps_provider_launch_isolation_unimplemented_apps_and_wrong_protocol_refuse() {
    for app in [
        AppId::ClaudeDesktop,
        AppId::GrokBuild,
        AppId::OpenCode,
        AppId::OpenClaw,
        AppId::Hermes,
        AppId::Pi,
        AppId::Mcode,
        AppId::Kimi,
        AppId::Dsh,
        AppId::Omp,
        AppId::Codex,
    ] {
        let root = tempfile::tempdir().unwrap();
        assert!(
            prepare_provider_launch(
                app,
                &profile(ProviderProtocol::Anthropic),
                material(root.path(), "missing"),
                root.path()
            )
            .is_err()
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
#[test]
fn cc_apps_provider_launch_isolation_failure_drop_and_private_permissions() {
    let root = tempfile::tempdir().unwrap();
    let temp = root.path().join("private");
    std::fs::create_dir(&temp).unwrap();
    for body in ["#!/bin/sh\nexit 37\n", "not an executable format"] {
        executable(&root.path().join("claude"), body);
        let launch = prepare_provider_launch(
            AppId::Claude,
            &profile(ProviderProtocol::Anthropic),
            material(root.path(), "claude"),
            &temp,
        )
        .unwrap();
        let private = std::fs::read_dir(&temp).unwrap().next().unwrap().unwrap().path();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&private).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(
                std::fs::metadata(private.join("settings.json")).unwrap().permissions().mode()
                    & 0o777,
                0o600
            );
        }
        assert!(launch.run().is_err());
        assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
    }
    let launch = prepare_provider_launch(
        AppId::Claude,
        &profile(ProviderProtocol::Anthropic),
        material(root.path(), "claude"),
        &temp,
    )
    .unwrap();
    drop(launch);
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
    let mut invalid = profile(ProviderProtocol::Anthropic);
    invalid.base_url = "https://name:fixture-key-only@example.test".into();
    let error =
        prepare_provider_launch(AppId::Claude, &invalid, material(root.path(), "claude"), &temp)
            .err()
            .unwrap();
    assert!(!error.contains("fixture-key-only"));
    assert!(!format!("{invalid:?}").contains("fixture-key-only"));
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}
#[test]
fn cc_apps_provider_launch_isolation_inherited_credentials_are_removed() {
    const CHILD: &str = "SHIJU31_ENV_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact", "apps::launch::provider_tests::cc_apps_provider_launch_isolation_inherited_credentials_are_removed"]).env(CHILD, "1").env("ANTHROPIC_AUTH_TOKEN", "ambient-fixture").env("OPENAI_API_KEY", "ambient-fixture").env("GOOGLE_API_KEY", "ambient-fixture").env("SHIJU_PROVIDER_API_KEY", "ambient-fixture").status().unwrap();
        assert!(status.success());
        return;
    }
    let root = tempfile::tempdir().unwrap();
    executable(
        &root.path().join("claude"),
        "#!/bin/sh\ntest -z \"$ANTHROPIC_AUTH_TOKEN$OPENAI_API_KEY$GOOGLE_API_KEY$SHIJU_PROVIDER_API_KEY\" || exit 41\ntest \"$ANTHROPIC_API_KEY\" = fixture-key-only || exit 42\n",
    );
    prepare_provider_launch(
        AppId::Claude,
        &profile(ProviderProtocol::Anthropic),
        material(root.path(), "claude"),
        root.path(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(std::env::var("OPENAI_API_KEY").unwrap(), "ambient-fixture");
}
#[test]
fn cc_apps_provider_launch_isolation_project_conflict_and_permission_flags() {
    let root = tempfile::tempdir().unwrap();
    executable(&root.path().join("codex"), "#!/bin/sh\nprintf '%s\\n' \"$@\" > argv\n");
    let mut m = material(root.path(), "codex");
    m.skip_permissions = true;
    prepare_provider_launch(
        AppId::Codex,
        &profile(ProviderProtocol::OpenaiResponses),
        m,
        root.path(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert!(
        std::fs::read_to_string(root.path().join("argv"))
            .unwrap()
            .contains("--dangerously-bypass-approvals-and-sandbox")
    );
    std::fs::create_dir(root.path().join(".codex")).unwrap();
    std::fs::write(root.path().join(".codex/config.toml"), "model_provider='other'").unwrap();
    assert!(
        prepare_provider_launch(
            AppId::Codex,
            &profile(ProviderProtocol::OpenaiResponses),
            material(root.path(), "codex"),
            root.path()
        )
        .is_err()
    );
    let mut m = material(root.path(), "codex");
    m.skip_permissions = true;
    assert!(
        prepare_provider_launch(AppId::Gemini, &profile(ProviderProtocol::Gemini), m, root.path())
            .is_err()
    );
}

#[test]
fn cc_apps_provider_launch_isolation_windows_batch_rules_refuse_without_spawn() {
    let root = tempfile::tempdir().unwrap();
    for name in ["codex.cmd", "claude.BAT"] {
        executable(&root.path().join(name), "#!/bin/sh\ntouch should-not-exist\n");
        assert!(
            prepare_provider_launch(
                AppId::Claude,
                &profile(ProviderProtocol::Anthropic),
                material(root.path(), name),
                root.path()
            )
            .is_err()
        );
        assert!(!root.path().join("should-not-exist").exists());
    }
}

#[test]
fn cc_apps_provider_launch_isolation_invalid_inputs_have_no_side_effects() {
    let root = tempfile::tempdir().unwrap();
    executable(&root.path().join("claude"), "#!/bin/sh\ntouch unexpected-launch\n");
    let private = root.path().join("private");
    std::fs::create_dir(&private).unwrap();
    for (field, value) in [
        ("api_key", "fixture-key-only\nextra"),
        ("base_url", "http://remote.example.test"),
        ("base_url", "https://example.test/?key=fixture-key-only"),
        ("base_url", "not-a-url"),
        ("model", "--dangerously-skip-permissions"),
        ("model", "line\nbreak"),
        ("name", " "),
    ] {
        let mut invalid = profile(ProviderProtocol::Anthropic);
        match field {
            "api_key" => invalid.api_key = value.into(),
            "base_url" => invalid.base_url = value.into(),
            "model" => invalid.model = value.into(),
            _ => invalid.name = value.into(),
        }
        let error = prepare_provider_launch(
            AppId::Claude,
            &invalid,
            material(root.path(), "claude"),
            &private,
        )
        .err()
        .unwrap();
        assert!(!error.contains("fixture-key-only"));
        assert_eq!(std::fs::read_dir(&private).unwrap().count(), 0);
    }
    for arg in ["--settings", "line\nbreak"] {
        let mut input = material(root.path(), "claude");
        input.args = vec![arg.into()];
        assert!(
            prepare_provider_launch(
                AppId::Claude,
                &profile(ProviderProtocol::Anthropic),
                input,
                &private
            )
            .is_err()
        );
    }
    assert!(!root.path().join("unexpected-launch").exists());
    assert_eq!(std::fs::read_dir(&private).unwrap().count(), 0);
}
