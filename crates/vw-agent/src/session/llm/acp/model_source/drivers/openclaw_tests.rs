use super::*;
use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

fn profile() -> AcpLaunchProfile {
    profile_with_command("/not-installed/openclaw")
}

fn profile_with_command(command: &str) -> AcpLaunchProfile {
    AcpLaunchProfile::new(
        &AcpAgentConfig { command: command.into(), args: vec!["acp".into()], ..Default::default() },
        RelayIdentity {
            tenant: crate::session::ui_store::current_tenant_scope(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap()
}

#[test]
fn acp_model_source_openclaw_config_routes_only_owned_gateway_and_model() {
    let mut launch = profile();
    // Preparing material must not require or start an installed Gateway process.
    configure(&mut launch, "http://127.0.0.1:32123").unwrap();
    let config: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&launch.config().env["OPENCLAW_CONFIG_PATH"]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        config["models"]["providers"]["session-provider"]["baseUrl"],
        "http://127.0.0.1:32123/v1"
    );
    assert_eq!(config["agents"]["defaults"]["model"]["primary"], "session-provider/selected-model");
    assert_eq!(config["agents"]["defaults"]["model"]["fallbacks"], serde_json::json!([]));
    assert_eq!(config["models"]["providers"].as_object().unwrap().len(), 1);
    let token = launch.binding.bearer_token();
    assert!(!serde_json::to_string(&config).unwrap().contains(token));
    assert!(launch.config().args.iter().all(|arg| !arg.contains(token)));
    assert!(launch.config().args.contains(&"--token-file".into()));
    assert_ne!(
        launch.config().env["OPENCLAW_STATE_DIR"],
        std::env::var("HOME").unwrap_or_default()
    );
}

#[test]
fn acp_model_source_openclaw_rejects_remote_gateway_and_conflicting_configuration() {
    for endpoint in
        ["https://remote.example", "http://127.0.0.1:32123/path", "http://user@127.0.0.1:32123"]
    {
        assert!(configure(&mut profile(), endpoint).is_err());
    }
    let mut launch = profile();
    launch
        .overlay_environment(std::collections::HashMap::from([(
            "OPENCLAW_CONFIG_PATH".into(),
            "/conflicting/config".into(),
        )]))
        .unwrap();
    assert!(configure(&mut launch, "http://127.0.0.1:32123").is_err());
    let mut launch = profile();
    launch.append_arguments(&["--url".into(), "ws://remote.example".into()]).unwrap();
    assert!(configure(&mut launch, "http://127.0.0.1:32123").is_err());
}

#[test]
fn acp_model_source_openclaw_reissue_preserves_history_without_persisting_credentials() {
    let root = tempfile::tempdir().unwrap();
    let mut first = profile();
    first.state_root = Some(root.path().to_owned());
    configure(&mut first, "http://127.0.0.1:32123").unwrap();
    let read = |launch: &AcpLaunchProfile| -> serde_json::Value {
        serde_json::from_slice(
            &std::fs::read(&launch.config().env["OPENCLAW_CONFIG_PATH"]).unwrap(),
        )
        .unwrap()
    };
    let first_config = read(&first);
    let old_token = first.binding.bearer_token().to_owned();
    let old_binding = first.binding.clone();
    let runtime = first.directory().to_owned();
    drop(first);
    assert!(!runtime.exists());
    assert!(old_binding.cancelled.is_cancelled());
    let mut second = profile();
    second.state_root = Some(root.path().to_owned());
    second.model_alias = "switched-model".into();
    configure(&mut second, "http://127.0.0.1:32124").unwrap();
    let second_config = read(&second);
    assert_eq!(first_config["session"]["store"], second_config["session"]["store"]);
    assert_ne!(old_token, second.binding.bearer_token());
    assert_eq!(
        second_config["agents"]["defaults"]["model"]["primary"],
        "session-provider/switched-model"
    );
    assert!(!second_config.to_string().contains(&old_token));
}

#[cfg(unix)]
#[test]
fn openclaw_gateway_resolves_cli_and_interpreter_with_desktop_path() {
    const CHILD: &str = "VW_ACP_TEST_OPENCLAW_PATH_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let absolute = std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
                .join("profile-bin/openclaw");
            for command in ["openclaw", absolute.to_str().unwrap()] {
                let mut launch = profile_with_command(command);
                configure(&mut launch, "http://127.0.0.1:32123").unwrap();
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    launch.gateway.as_mut().unwrap().start(),
                )
                .await
                .expect("Gateway readiness timed out")
                .expect("Gateway should resolve both its CLI and interpreter");
            }
        });
        return;
    }

    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("profile-bin");
    std::fs::create_dir(&bin).unwrap();
    let python = vw_shared::shell::resolve_executable("python3").expect("Python fixture runtime");
    for (name, source) in [
        ("openclaw", "#!/usr/bin/env vw-openclaw-runtime-fixture\n".to_owned()),
        (
            "vw-openclaw-runtime-fixture",
            format!(
                "#!{}\nimport socket,sys,time\nassert sys.argv[2:4] == ['gateway','run']\ns=socket.socket()\ns.bind(('127.0.0.1',int(sys.argv[sys.argv.index('--port')+1])))\ns.listen()\ntime.sleep(60)\n",
                python.display()
            ),
        ),
    ] {
        let path = bin.join(name);
        std::fs::write(&path, source).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        home.path().join(".profile"),
        format!("export PATH=\"{}:$PATH\"\n", bin.display()),
    )
    .unwrap();
    // Keep the desktop PATH and shell cache isolated from other tests.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "session::llm::acp::model_source::drivers::openclaw::tests::openclaw_gateway_resolves_cli_and_interpreter_with_desktop_path",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated OpenClaw regression failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
