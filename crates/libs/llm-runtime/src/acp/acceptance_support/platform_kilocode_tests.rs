use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires the current Linux gateway and privately prepared Kilocode artifact"]
fn acp_platform_kilocode_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_KILOCODE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_KILOCODE_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("kilocode-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "kilocode", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Kilocode gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unexpected Linux target {other}"),
    };
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/kilocode/linux.json"
    ))
    .unwrap();
    assert_eq!(evidence["artifact"]["version"], "7.8.18");
    assert_eq!(evidence["artifact"]["revision"], contract["revision"]);
    assert_eq!(evidence["artifact"]["sha256"], contract["targets"][arch]["patched_sha256"]);
    assert_eq!(evidence["artifact"]["entry"], "reviewed/kilocode");
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
        for name in ["native", "native_read"] {
            assert!(calls[name].as_str().is_some_and(|id| !id.is_empty()));
        }
        for name in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(name), mounted);
        }
        assert_eq!(turn["mcp_calls"], if phase == "mounted" { 1 } else { 0 });
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => serde_json::json!(["once"]),
                "rejected" => serde_json::json!(["reject"]),
                _ => serde_json::json!([]),
            }
        );
        assert_eq!(turn["terminal_mcp_rejection"], Value::Null);
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
