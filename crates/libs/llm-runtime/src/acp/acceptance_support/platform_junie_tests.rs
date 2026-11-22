use crate::acp::model_source::{drivers, profile::AcpLaunchProfile};
use agent_config::schema::AcpAgentConfig;
use serde_json::Value;
use vw_api_acp_model::acp_model_source::{
    AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
};

#[test]
fn acp_platform_junie_contract() {
    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/junie/linux-artifacts.json"
    )))
    .unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    let (archive_arch, machine) = match std::env::consts::ARCH {
        "x86_64" => ("amd64", 62),
        "aarch64" => ("aarch64", 183),
        arch => panic!("Unexpected Junie GNU target {arch}"),
    };
    assert_eq!(manifest["version"], "4100.5");
    assert_eq!(manifest["reported_version"].as_str(), drivers::verified_release("junie"));
    assert_eq!(manifest["entry"], "junie-app/bin/junie");
    assert_eq!(manifest["args"], serde_json::json!(["--acp=true"]));
    assert_eq!(target["elf_machine"], machine);
    assert_eq!(
        target["archive_url"],
        format!(
            "https://github.com/JetBrains/junie/releases/download/4100.5/junie-release-4100.5-linux-{archive_arch}.zip"
        )
    );

    assert_eq!(
        drivers::capabilities("junie")[0],
        AcpModelSourceCapability::Unsupported {
            source: AcpModelSource::SessionModel,
            reason: AcpModelSourceRejectionReason::UnsupportedCli,
        }
    );
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        url: "http://127.0.0.1:34567/v1".into(),
        adapter: "openai-compatible".into(),
        protocols: None,
    };
    assert_eq!(
        drivers::source_protocol("junie", &api).unwrap(),
        model_protocols::ModelProtocol::Chat
    );
    let workspace = tempfile::tempdir().unwrap();
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: workspace.path().join("junie").to_string_lossy().into_owned(),
            args: vec!["--acp=true".into()],
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "junie-platform".into(),
            session: "junie-platform".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    let error = drivers::junie::configure(
        &mut profile,
        "http://127.0.0.1:34567",
        workspace.path().to_str().unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "Junie managed launch requires Linux process and MCP isolation");
    assert!(!profile.config().env.contains_key("SHIJU_JUNIE_CONFIG"));
    assert!(!profile.directory().join("junie.json").exists());
    assert!(profile.session_model_id.is_none());
}

#[tokio::test]
#[ignore = "requires a disposable GNU Linux container and access to the fixed JetBrains archive"]
async fn acp_platform_junie_contract_native() {
    use acp_agent_registry::{install_builtin_agent, resolve_agent_spec};
    use acp_client::AcpClient;
    use acp_types::{AcpAgentConfig, PermissionMode};
    use std::{env, fs, path::Path, process::Command, time::Duration};

    const ROOT_ENV: &str = "SHIJU_TEST_JUNIE_NATIVE_ROOT";
    let Some(root) = env::var_os(ROOT_ENV) else {
        let root = tempfile::tempdir().unwrap();
        for directory in ["home", "config", "workspace"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let module = module_path!().split_once("::").unwrap().1;
        let status = Command::new(env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                &format!("{module}::acp_platform_junie_contract_native"),
            ])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", root.path().join("home"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env(ROOT_ENV, root.path())
            .current_dir(root.path().join("workspace"))
            .status()
            .unwrap();
        assert!(status.success(), "Junie native contract failed");
        return;
    };

    let root = Path::new(&root);
    install_builtin_agent("junie").await.unwrap();
    let entry = root
        .join("config/acp-runtime/junie-4100.5-linux-native-v1/junie-app/bin/junie")
        .canonicalize()
        .unwrap();
    let spec = resolve_agent_spec("junie").unwrap();
    assert_eq!(Path::new(&spec.command), entry.as_path());
    assert_eq!(spec.args, ["--acp=true"]);
    assert!(spec.env.is_empty());
    let client = AcpClient::new(
        "junie",
        AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env },
    )
    .with_permission_mode(PermissionMode::DenyAll);
    let created = tokio::time::timeout(
        Duration::from_secs(30),
        client.create_session(root.join("workspace")),
    )
    .await;
    client.close().await.unwrap();
    assert!(!created.unwrap().unwrap().session_id.is_empty());

    // Tampering with a non-entry file must also invalidate the fixed install.
    fs::write(entry.parent().unwrap().join("../lib/app/junie.cfg"), "changed").unwrap();
    assert!(
        acp_agent_registry::verify_fixed_linux_cli_launch(
            "junie",
            entry.to_str().unwrap(),
            &["--acp=true".into()]
        )
        .is_err()
    );
}
