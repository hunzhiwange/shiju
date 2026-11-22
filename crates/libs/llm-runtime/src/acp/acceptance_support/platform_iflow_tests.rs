use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and the reviewed iFlow npm installation"]
fn acp_platform_iflow_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_IFLOW_GATEWAY_BIN")
        .expect("set SHIJU_ACP_IFLOW_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let runtime = std::env::var_os("IFLOW_COEXISTENCE_RUNTIME")
        .expect("set IFLOW_COEXISTENCE_RUNTIME to the reviewed iFlow npm installation");
    let runtime = Path::new(&runtime).canonicalize().unwrap();
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

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        let mounted = matches!(phase, "mounted" | "rejected");
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        assert_eq!(turn["mcp_calls"], usize::from(phase == "mounted"));
        assert_eq!(
            turn["skill_approvals"],
            if mounted {
                json!(["native", "native_read", "selected", "selected_read"])
            } else {
                json!(["native", "native_read"])
            }
        );
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        // The model handler accepts only authenticated Chat Completions requests
        // and checks each continuation against the tool call it issued.
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
