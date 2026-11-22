use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway, a reviewed Poolside install and an isolated Docker daemon"]
fn acp_platform_poolside_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_POOLSIDE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_POOLSIDE_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("poolside-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "poolside", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Poolside platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "1.0.16");
    assert_eq!(artifact["revision"], "poolside-1.0.16-model-source-v4");
    assert_eq!(artifact["entry"], "pool");
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");
    let expected = match std::env::consts::ARCH {
        "x86_64" => "db70340b41c8bd76dbe2b5926a6e46cda804789b9774b3d4cfba3591f0de02f2",
        "aarch64" => "0c06c4f5a20f092193b9968172dc6c19fd90c1b2e22f53adebaeb97d6ac550d2",
        other => panic!("unreviewed Poolside architecture: {other}"),
    };
    assert_eq!(artifact["native_sha256"], expected);

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (row, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["source"], "session_model");
        assert_eq!(row["model"], "coexistence-fixture/selected-model");
        assert_eq!(row["done"], 1);
        assert_eq!(row["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = row["calls"].as_object().unwrap();
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), selected);
        assert_eq!(calls.contains_key("selected_read"), selected);
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(row["mcp_calls"], u32::from(phase == "mounted"));
        assert_eq!(
            row["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        if phase == "mounted" {
            assert_eq!(row["model_requests"], calls.len() as u64 + 1);
        }
    }
}

#[tokio::test]
#[ignore = "requires access to the pinned Poolside release from a disposable GNU Linux container"]
async fn acp_platform_poolside_contract_cli_local() {
    use acp_agent_registry::{install_builtin_agent, resolve_agent_spec};
    use acp_client::AcpClient;
    use acp_types::{AcpAgentConfig, PermissionMode};
    use std::{env, fs, time::Duration};

    const FIXTURE_ROOT: &str = "SHIJU_TEST_POOLSIDE_CLI_LOCAL_ROOT";
    let Some(root) = env::var_os(FIXTURE_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        for name in ["home", "config", "workspace"] {
            fs::create_dir(root.path().join(name)).unwrap();
        }
        let module = module_path!().split_once("::").unwrap().1;
        let test = format!("{module}::acp_platform_poolside_contract_cli_local");
        let status = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", &test])
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("HOME", root.path().join("home"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env(FIXTURE_ROOT, root.path())
            .current_dir(root.path().join("workspace"))
            .status()
            .unwrap();
        assert!(status.success(), "isolated Poolside cli_local contract failed");
        return;
    };

    let root = Path::new(&root);
    let workspace = root.join("workspace");
    install_builtin_agent("poolside").await.unwrap();
    let entry =
        root.join("config/acp-runtime/poolside-1.0.16-linux-cli-v1/pool").canonicalize().unwrap();
    let spec = resolve_agent_spec("poolside").unwrap();
    assert_eq!(Path::new(&spec.command), entry.as_path());
    assert_eq!(spec.args, ["acp"]);
    assert!(spec.env.is_empty());

    let ordinary = AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env };
    let client =
        |config| AcpClient::new("poolside", config).with_permission_mode(PermissionMode::DenyAll);
    let timeout = Duration::from_secs(30);
    let first = client(ordinary.clone());
    let result = tokio::time::timeout(timeout, first.create_session(&workspace)).await;
    first.close().await.unwrap();
    let session_id = result.unwrap().unwrap().session_id;
    assert!(!session_id.is_empty());

    let second = client(ordinary.clone());
    let result = tokio::time::timeout(timeout, second.load_session(&session_id, &workspace)).await;
    second.close().await.unwrap();
    assert_eq!(result.unwrap().unwrap().session_id, session_id);

    let mut managed = ordinary.clone();
    managed.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "unissued-fixture-reference".into());
    let managed = client(managed);
    let result = tokio::time::timeout(timeout, managed.create_session(&workspace)).await;
    managed.close().await.unwrap();
    let error = result.unwrap().unwrap_err().to_string();
    assert!(error.contains("no verified managed Linux launch"), "{error}");

    fs::write(&entry, b"unreviewed executable").unwrap();
    let third = client(ordinary);
    let result = tokio::time::timeout(timeout, third.load_session(&session_id, &workspace)).await;
    third.close().await.unwrap();
    let error = result.unwrap().unwrap_err().to_string();
    assert!(error.contains("Fixed Linux ACP executable checksum mismatch"), "{error}");
}
