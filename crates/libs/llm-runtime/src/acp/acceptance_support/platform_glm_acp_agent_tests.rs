use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway binary and npm access for the private GLM install"]
fn acp_platform_glm_acp_agent_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_GLM_GATEWAY_BIN")
        .expect("set SHIJU_ACP_GLM_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("glm-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "glm-acp-agent", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "GLM platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let machine = match std::env::consts::ARCH {
        "x86_64" => "Linux-x86_64",
        "aarch64" => "Linux-aarch64",
        other => panic!("unexpected Linux target {other}"),
    };
    assert_eq!(evidence["platform"], machine);
    assert_eq!(evidence["artifact"]["version"], "1.17.1");
    assert_eq!(evidence["artifact"]["revision"], "glm-acp-agent-1.17.1-model-source-v3");
    assert_eq!(evidence["artifact"]["entry"], "dist/index.js");
    assert_eq!(evidence["artifact"]["model_endpoint"], "/v1/chat/completions");
    assert_eq!(
        evidence["artifact"]["sha256"]["dist/index.js"],
        "d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98"
    );

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
