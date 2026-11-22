use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway, a reviewed Poolside install and an isolated Docker daemon"]
fn acp_platform_poolside_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_POOLSIDE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_POOLSIDE_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("poolside-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "poolside", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Poolside platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], "1.0.16");
    assert_eq!(artifact["revision"], "poolside-1.0.16-model-source-v4");
    assert_eq!(artifact["entry"], "pool");
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");
    let expected = match std::env::consts::ARCH {
        "x86_64" => "db70340b41c8bd76dbe2b5926a6e46cda804789b9774b3d4cfba3591f0de02f2",
        "aarch64" => "0c06c4f5a20f092193b9968172dc6c19fd90c1b2e22f53adebaeb97d6ac550d2",
        other => panic!("unreviewed Poolside architecture: {other}"),
    };
    assert_eq!(artifact["native_sha256"], expected);

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (row, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["source"], "session_model");
        assert_eq!(row["model"], "coexistence-fixture/selected-model");
        assert_eq!(row["done"], 1);
        assert_eq!(row["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = row["calls"].as_object().unwrap();
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), selected);
        assert_eq!(calls.contains_key("selected_read"), selected);
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(row["mcp_calls"], u32::from(phase == "mounted"));
        assert_eq!(
            row["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        if phase == "mounted" {
            assert_eq!(row["model_requests"], calls.len() as u64 + 1);
        }
    }
}
