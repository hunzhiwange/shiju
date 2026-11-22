use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway binary and a reviewed host-built Goose runtime"]
fn acp_platform_goose_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_GOOSE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_GOOSE_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let runtime = std::env::var_os("ACP_COEXISTENCE_GOOSE_BIN")
        .expect("set ACP_COEXISTENCE_GOOSE_BIN to the reviewed Linux Goose executable");
    let runtime = Path::new(&runtime).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("goose-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "goose", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .env("ACP_COEXISTENCE_GOOSE_BIN", runtime)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Goose platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "v1.57.0");
    assert_eq!(artifact["entry"], "goose");
    assert_eq!(artifact["target"], format!("{}-unknown-linux-gnu", std::env::consts::ARCH));
    assert_eq!(
        artifact["source_sha256"],
        "36fcab524e25ca563830757bf2da8a68716ea2a5d7ca2f055dec4647b78548d9"
    );
    assert_eq!(
        artifact["patch_sha256"],
        "3b75b96dc038b60f294de02354de34150a204e5e08f51a5bd37ff9095898ebcb"
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
        assert_eq!(turn["mcp_calls"], usize::from(phase == "mounted"));
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
        assert_eq!(turn["skill_approvals"][0], "tree");
        // The authenticated fixture responds only to /v1/chat/completions.
        // Each completed call and the native tree read require a continuation.
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64 + 1);
    }
}
