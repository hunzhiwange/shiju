use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and a private managed OpenClaw install"]
fn acp_platform_openclaw_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_OPENCLAW_GATEWAY_BIN")
        .expect("set SHIJU_ACP_OPENCLAW_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("openclaw-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "openclaw", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "OpenClaw platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "2026.10.10");
    assert_eq!(evidence["artifact"]["revision"], "openclaw-2026.10.10-model-source-v3");
    assert_eq!(evidence["artifact"]["entry"], "openclaw.mjs");
    assert_eq!(
        evidence["artifact"]["sha256"]["openclaw.mjs"],
        "469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902"
    );
    assert_eq!(evidence["artifact"]["native_sha256"].as_object().unwrap().len(), 3);

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
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert_eq!(turn["mcp_calls"], u32::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        // The probe only counts tool-bearing requests on /v1/chat/completions.
        assert_eq!(turn["model_requests"], calls.len() as u64 + 1);
    }
}
