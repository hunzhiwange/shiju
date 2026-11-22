use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and a private reviewed Nova install with test consent"]
fn acp_platform_nova_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_NOVA_GATEWAY_BIN")
        .expect("set SHIJU_ACP_NOVA_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("nova-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "nova", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Nova gateway contract failed at {}: {evidence}", evidence["stage"]);
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "1.1.65");
    assert_eq!(evidence["artifact"]["revision"], "nova-1.1.65-model-source-v5");
    assert_eq!(evidence["artifact"]["entry"], "scripts/bin-wrapper.mjs");
    assert_eq!(evidence["artifact"]["model_protocol"], "anthropic");
    assert_eq!(
        evidence["artifact"]["sha256"]["dist/cli.js"],
        "904c873dae2a9eba307496bfa72bbbf3df84372711166a0540a1b58bbd077249"
    );

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/selected-model");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), selected);
        assert_eq!(calls.contains_key("selected_read"), selected);
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(turn["mcp_calls"], if phase == "mounted" { 1 } else { 0 });
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => serde_json::json!(["once"]),
                "rejected" => serde_json::json!(["reject"]),
                _ => serde_json::json!([]),
            }
        );
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
