use super::*;

#[test]
fn omp_routes_only_through_the_reviewed_chat_transport() {
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        adapter: "openai-compatible".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        protocols: None,
    };
    assert_eq!(source_protocol("omp", &api).unwrap(), model_protocols::ModelProtocol::Chat,);
    assert_eq!(verified_release("omp"), Some("18.22.0"));
}

#[test]
fn omp_rejects_an_unreviewed_binary_before_exposing_credentials() {
    let config = agent_config::schema::AcpAgentConfig {
        command: "omp".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let identity = model_transport::RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: "session:omp".into(),
        provider: "provider".into(),
        model: "provider/model".into(),
        generation: 0,
    };
    let mut profile = AcpLaunchProfile::new(&config, identity).unwrap();
    let error = omp::configure(&mut profile, "http://127.0.0.1:12345/", "/workspace");
    assert!(error.is_err());
    assert!(!profile.config().env.contains_key("SHIJU_OMP_RELAY_TOKEN"));
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires a Linux Shiju gateway and the pinned OMP binary"]
fn acp_platform_omp_contract() {
    use serde_json::{Value, json};
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_OMP_GATEWAY_BIN")
        .expect("set SHIJU_ACP_OMP_GATEWAY_BIN to the Linux Shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("omp-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "omp", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "OMP gateway contract failed at {}: {evidence}", evidence["stage"]);
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "18.22.0");
    assert_eq!(artifact["entry"], "omp");
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");
    let platform = match std::env::consts::ARCH {
        "x86_64" => "linux-x64",
        "aarch64" => "linux-arm64",
        arch => panic!("unsupported OMP test architecture: {arch}"),
    };
    assert_eq!(artifact["platform"], platform);

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/selected-model");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let mounted = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
        assert_eq!(turn["mcp_calls"], if phase == "mounted" { 1 } else { 0 });
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
    }
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[tokio::test]
#[ignore = "requires a disposable GNU Linux container with access to the pinned OMP release"]
async fn acp_platform_omp_contract_cli_local() {
    use acp_agent_registry::{install_builtin_agent, resolve_agent_spec};
    use acp_client::AcpClient;
    use acp_types::AcpAgentConfig;
    use std::{env, fs, path::Path, process::Command, time::Duration};

    const CHILD_ROOT: &str = "SHIJU_TEST_OMP_CLI_LOCAL_ROOT";
    let Some(root) = env::var_os(CHILD_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        for directory in ["home", "config", "workspace"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let module = module_path!().split_once("::").unwrap().1;
        let name = format!("{module}::acp_platform_omp_contract_cli_local");
        let status = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", &name])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", root.path().join("home"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env(CHILD_ROOT, root.path())
            .current_dir(root.path().join("workspace"))
            .status()
            .unwrap();
        assert!(status.success(), "isolated OMP cli_local contract failed");
        return;
    };

    let root = Path::new(&root);
    install_builtin_agent("omp").await.unwrap();
    let entry = root
        .join(format!("config/acp-runtime/omp-18.22.0-linux-cli-{}/omp", env::consts::ARCH))
        .canonicalize()
        .unwrap();
    let spec = resolve_agent_spec("omp").unwrap();
    assert_eq!(Path::new(&spec.command), entry.as_path());
    assert_eq!(spec.args, ["acp"]);
    assert!(spec.env.is_empty());

    let client = || {
        AcpClient::new(
            "omp",
            AcpAgentConfig {
                command: spec.command.clone(),
                args: spec.args.clone(),
                env: spec.env.clone(),
            },
        )
        .with_full_access(true)
    };
    let workspace = root.join("workspace");
    let first = client();
    let session =
        tokio::time::timeout(Duration::from_secs(30), first.create_session(&workspace)).await;
    first.close().await.unwrap();
    assert!(!session.unwrap().unwrap().session_id.is_empty());

    // Keep the persisted command and args while replacing the installed file.
    // A new ACP client must recheck the entry before handing it to the OS.
    fs::write(&entry, b"unreviewed executable").unwrap();
    let second = client();
    let session =
        tokio::time::timeout(Duration::from_secs(10), second.create_session(&workspace)).await;
    second.close().await.unwrap();
    let error = session.unwrap().unwrap_err().to_string();
    assert!(error.contains("Invalid reviewed OMP Linux executable metadata"), "{error}");
}
