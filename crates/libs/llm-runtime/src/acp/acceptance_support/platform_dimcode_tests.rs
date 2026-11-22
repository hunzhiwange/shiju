#![cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

use serde_json::{Value, json};
use std::{path::Path, process::Command};

const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/dimcode/linux-artifacts.json"
));

#[test]
#[ignore = "requires a Linux gateway and a reviewed DimCode executable"]
fn acp_platform_dimcode_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_DIMCODE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_DIMCODE_GATEWAY_BIN to the Linux gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let output = tempfile::tempdir().unwrap();
    let report = output.path().join("dimcode.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "dimcode", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "DimCode gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));

    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], manifest["version"]);
    assert_eq!(artifact["revision"], manifest["revision"]);
    assert_eq!(artifact["entry"], manifest["bin"]);
    assert_eq!(artifact["sha256"], target["managed_sha256"]);
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/selected-model");
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
