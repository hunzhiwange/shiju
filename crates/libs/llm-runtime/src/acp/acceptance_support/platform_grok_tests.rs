use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and the reviewed native Grok executable"]
fn acp_platform_grok_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_GROK_GATEWAY_BIN")
        .expect("set SHIJU_ACP_GROK_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("grok.json");

    let status = Command::new("/usr/bin/python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "grok", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Grok gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));

    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/grok/linux-artifacts.json"
    )))
    .unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    assert_eq!(evidence["artifact"]["version"], manifest["reported_version"]);
    assert_eq!(evidence["artifact"]["entry"], manifest["bin"]);
    assert_eq!(evidence["artifact"]["entry_sha256"], target["sha256"]);
    assert_eq!(evidence["artifact"]["model_endpoint"], "/v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let mounted = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert_eq!(turn["mcp_calls"], usize::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => serde_json::json!(["once"]),
                "rejected" => serde_json::json!(["reject"]),
                _ => serde_json::json!([]),
            }
        );
        let terminal_rejection = phase == "rejected" && turn["terminal_mcp_rejection"] == true;
        assert!(
            turn["model_requests"].as_u64().unwrap()
                > (calls.len() - usize::from(terminal_rejection)) as u64
        );
    }
}
