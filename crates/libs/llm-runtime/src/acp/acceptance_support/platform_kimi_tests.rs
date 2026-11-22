use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux ShiJu gateway and the pinned Kimi executable"]
fn acp_platform_kimi_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_KIMI_GATEWAY_BIN")
        .expect("set SHIJU_ACP_KIMI_GATEWAY_BIN to the current Linux gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("kimi.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "kimi", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Kimi gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));

    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/kimi/linux.json"
    )))
    .unwrap();
    let artifact = &evidence["artifact"];
    let target = &manifest["targets"][std::env::consts::ARCH];
    assert_eq!(artifact["target"], target["platform"]);
    assert_eq!(artifact["version"], manifest["version"]);
    assert_eq!(artifact["sha256"], target["sha256"]);
    assert_eq!(artifact["args"], manifest["args"]);
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    let phases = [
        ("baseline", false, 0, json!([])),
        ("mounted", true, 1, json!(["once"])),
        ("removed", false, 0, json!([])),
        ("rejected", true, 0, json!(["reject"])),
    ];
    assert_eq!(turns.len(), phases.len());
    for (turn, (phase, mounted, mcp_calls, decisions)) in turns.iter().zip(phases) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        assert_eq!(turn["mcp_calls"], mcp_calls);
        assert_eq!(turn["mcp_decisions"], decisions);
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
