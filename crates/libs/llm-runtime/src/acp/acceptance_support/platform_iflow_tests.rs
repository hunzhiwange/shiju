use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[tokio::test]
#[ignore = "requires a GNU Linux shiju gateway, Node.js and access to the pinned npm packages"]
async fn acp_platform_iflow_contract_native_approval() {
    const INSTALL_ROOT: &str = "SHIJU_TEST_IFLOW_INSTALL_ROOT";
    let Some(root) = std::env::var_os(INSTALL_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let home = root.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                &format!("{module}::acp_platform_iflow_contract_native_approval"),
            ])
            .env("HOME", home)
            .env("VIBEWINDOW_CONFIG_DIR", &config)
            .env(INSTALL_ROOT, &config)
            .status()
            .unwrap();
        assert!(status.success(), "iFlow native approval gateway contract failed");
        return;
    };
    acp_agent_registry::install_builtin_agent("iflow").await.unwrap();
    let runtime = Path::new(&root).join("acp-runtime/iflow-0.5.19-model-source-v4");
    let evidence = run_probe(&runtime);
    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 1);
    let baseline = &turns[0];
    assert_eq!(baseline["source"], "session_model");
    assert_eq!(baseline["skill_approvals"], json!(["native", "native_read"]));
    assert!(baseline["calls"]["native"].is_string());
    assert!(baseline["calls"]["native_read"].is_string());
    assert_eq!(baseline["mcp_calls"], 0);
    assert_eq!(baseline["mcp_decisions"], json!([]));
    assert_eq!(baseline["done"], 1);
}

fn run_probe(runtime: &Path) -> Value {
    let gateway = std::env::var_os("SHIJU_ACP_IFLOW_GATEWAY_BIN")
        .expect("set SHIJU_ACP_IFLOW_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("iflow-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "iflow", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .env("IFLOW_COEXISTENCE_RUNTIME", runtime)
        .arg("--baseline-only")
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "iFlow gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));

    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/iflow/linux-mcp-artifacts.json"
    )))
    .unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "0.5.19");
    assert_eq!(artifact["revision"], manifest["revision"]);
    assert_eq!(artifact["entry"], "bundle/entry.js");
    assert_eq!(artifact["args"], json!(["--experimental-acp"]));
    assert_eq!(artifact["integrity"], manifest["integrity"]);
    assert_eq!(artifact["sha256"], manifest["patched_files"]);
    assert_eq!(artifact["ripgrep_sha256"], target["ripgrep_sha256"]);
    assert_eq!(artifact["pty_sha256"], target["pty_sha256"]);

    evidence
}
