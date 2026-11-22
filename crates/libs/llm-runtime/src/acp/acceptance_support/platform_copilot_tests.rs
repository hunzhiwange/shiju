use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway and a privately installed reviewed Copilot runtime"]
fn acp_platform_copilot_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_COPILOT_GATEWAY_BIN")
        .expect("set SHIJU_ACP_COPILOT_GATEWAY_BIN to the current Linux gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("copilot-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "copilot", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .expect("launch the Copilot gateway probe");
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Copilot platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);

    let (platform, package) = match std::env::consts::ARCH {
        "x86_64" => ("Linux-x86_64", "@github/copilot-linux-x64"),
        "aarch64" => ("Linux-aarch64", "@github/copilot-linux-arm64"),
        other => panic!("unsupported Copilot test target: {other}"),
    };
    assert_eq!(evidence["platform"], platform);
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "1.0.105");
    assert_eq!(artifact["platform_package"], package);
    assert_eq!(artifact["revision"], "copilot-1.0.105-model-source-linux-v3");
    assert_eq!(artifact["entry"], "embedded/index.js");
    assert_eq!(artifact["model_protocol"], "/v1/chat/completions");

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
        assert_eq!(turn["mcp_calls"], u8::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            },
        );
        let requests = turn["model_requests"].as_u64().unwrap();
        if turn["terminal_mcp_rejection"] == true {
            assert_eq!(phase, "rejected");
            assert!(requests >= calls.len() as u64);
        } else {
            assert!(requests > calls.len() as u64);
        }
    }
}
