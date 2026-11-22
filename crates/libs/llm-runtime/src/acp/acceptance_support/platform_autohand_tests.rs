use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a current Linux shiju gateway and a private AutoHand installation"]
fn acp_platform_autohand_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_AUTOHAND_GATEWAY_BIN")
        .expect("set SHIJU_ACP_AUTOHAND_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("autohand-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "autohand", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert!(status.success(), "AutoHand platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "0.9.11 (46a28b16)");
    assert_eq!(artifact["revision"], "autohand-0.9.11-model-source-v1");
    assert_eq!(artifact["entry"], "dist/index.js");
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");
    assert_eq!(
        artifact["sha256"]["dist/acp-LDTU5B5V.js"],
        "9fb1a8986092536b5cea385301a471fe1770c922ea5de76ccd6cb511c0192863"
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
        if mounted {
            assert_eq!(turn["mcp_bridge_call_id"], calls["mcp"]);
        }
        assert_eq!(turn["mcp_calls"], if phase == "mounted" { 1 } else { 0 });
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
