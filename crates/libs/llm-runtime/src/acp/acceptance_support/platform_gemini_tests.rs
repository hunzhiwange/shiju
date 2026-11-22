use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway binary and the private reviewed Gemini runtime"]
fn acp_platform_gemini_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_GEMINI_GATEWAY_BIN")
        .expect("set SHIJU_ACP_GEMINI_GATEWAY_BIN to the current Linux shiju gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let runtime = std::env::var_os("GEMINI_COEXISTENCE_RUNTIME")
        .expect("set GEMINI_COEXISTENCE_RUNTIME to the reviewed Linux Gemini installation");
    let runtime = Path::new(&runtime).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("gemini-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "gemini", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .env("GEMINI_COEXISTENCE_RUNTIME", runtime)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Gemini platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "0.66.0");
    assert_eq!(evidence["artifact"]["revision"], "gemini-0.66.0-model-source-v3");
    assert_eq!(evidence["artifact"]["entry"], "bundle/gemini.js");
    assert_eq!(
        evidence["artifact"]["integrity"],
        "sha512-EAtsJ9/JxiphHlduzvhoCkb7I0LdOxyugB5NpUtVqKVGZIWjV3JmurvK89ZrvMbV3kwk5SdWINz1M5ZN/2K/Lw=="
    );
    assert_eq!(
        evidence["artifact"]["model_endpoint"],
        "/v1beta/models/gemini-2.5-flash:streamGenerateContent"
    );
    assert_eq!(
        evidence["artifact"]["sha256"]["bundle/gemini-EOCRFAPZ.js"],
        "f26f50b8eafd82b198d477bfd52ca9ab053ff0788f724ddd17d06656d015ef43"
    );

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/gemini-2.5-flash");
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
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
