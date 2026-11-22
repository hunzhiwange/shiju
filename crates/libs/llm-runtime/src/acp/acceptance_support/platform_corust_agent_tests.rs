use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway, the fixed Corust archive and RustCodeGraph"]
fn acp_platform_corust_agent_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_CORUST_GATEWAY_BIN")
        .expect("set SHIJU_ACP_CORUST_GATEWAY_BIN to the current Linux shiju gateway binary");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("corust-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "corust-agent", "--gateway-bin"])
        .arg(Path::new(&gateway).canonicalize().unwrap())
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Corust platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "0.6.0");
    assert_eq!(artifact["entry"], "corust-agent-acp");
    assert_eq!(
        artifact["native_sha256"],
        "3392fe6c6609a84824323f755b720f31f2d158ab6c74dbe360843334e02f1547"
    );

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        let mounted = matches!(phase, "mounted" | "rejected");
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert_eq!(turn["mcp_calls"], u8::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            },
        );
        // The provider probe only advances after receiving each tool result by its
        // original model ID on POST /v1/chat/completions.
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}

#[tokio::test]
#[ignore = "requires a disposable Linux container with glibc 2.39 and access to the pinned archive"]
async fn acp_platform_corust_agent_native_contract() {
    use acp_agent_registry::{install_builtin_agent, resolve_agent_spec};
    use acp_client::AcpClient;
    use acp_types::{AcpAgentConfig, PermissionMode};
    use std::{env, fs, time::Duration};

    const CHILD_ROOT: &str = "SHIJU_TEST_CORUST_NATIVE_ROOT";
    const AGENT: &str = "corust-agent";
    let Some(root) = env::var_os(CHILD_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        for directory in ["home", "config", "workspace"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let module = module_path!().split_once("::").unwrap().1;
        let test = format!("{module}::acp_platform_corust_agent_native_contract");
        let status = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", &test])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", root.path().join("home"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env(CHILD_ROOT, root.path())
            .current_dir(root.path().join("workspace"))
            .status()
            .unwrap();
        assert!(status.success(), "isolated Corust native contract failed");
        return;
    };

    let root = Path::new(&root);
    let runtime = root.join("config/acp-runtime");
    if cfg!(target_arch = "aarch64") {
        let error = install_builtin_agent(AGENT).await.unwrap_err();
        assert_eq!(error, "No fixed ACP executable for this Linux architecture");
        assert!(!runtime.exists());
        return;
    }

    install_builtin_agent(AGENT).await.unwrap();
    let entry = runtime
        .join("corust-agent-0.6.0-linux-x64-native/corust-agent-acp")
        .canonicalize()
        .unwrap();
    let spec = resolve_agent_spec(AGENT).unwrap();
    assert_eq!(Path::new(&spec.command), entry.as_path());
    assert!(spec.args.is_empty() && spec.env.is_empty());

    let client = AcpClient::new(
        AGENT,
        AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env },
    )
    .with_full_access(true)
    .with_permission_mode(PermissionMode::ApproveAll);
    let created = tokio::time::timeout(
        Duration::from_secs(30),
        client.create_session(root.join("workspace")),
    )
    .await;
    client.close().await.unwrap();
    assert!(!created.unwrap().unwrap().session_id.is_empty());
}
