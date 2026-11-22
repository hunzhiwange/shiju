use serde_json::{Value, json};
use std::{path::Path, process::Command};

const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/cortex-code/linux-artifacts.json"
));

#[test]
#[ignore = "requires a Linux shiju gateway and a private Cortex runtime fixture"]
fn acp_platform_cortex_code_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_CORTEX_GATEWAY_BIN")
        .expect("set SHIJU_ACP_CORTEX_GATEWAY_BIN to the current Linux gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("cortex-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "cortex-code", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], manifest["version"]);
    assert_eq!(evidence["artifact"]["entry"], "cortex");
    assert_eq!(evidence["artifact"]["native_sha256"], target["native_sha256"]);
    assert!(status.success(), "Cortex gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
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
        for key in ["native", "native_read"] {
            assert!(calls[key].as_str().is_some_and(|id| !id.is_empty()));
        }
        for key in ["selected", "selected_read", "mcp"] {
            assert_eq!(
                calls.get(key).and_then(Value::as_str).is_some_and(|id| !id.is_empty()),
                mounted
            );
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
