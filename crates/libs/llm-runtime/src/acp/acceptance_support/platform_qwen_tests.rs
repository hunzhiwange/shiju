use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux gateway and the reviewed Qwen Linux installation"]
fn acp_platform_qwen_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_QWEN_GATEWAY_BIN")
        .expect("set SHIJU_ACP_QWEN_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("qwen-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "qwen", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Qwen platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let target = match std::env::consts::ARCH {
        "x86_64" => "x86_64-unknown-linux-gnu",
        "aarch64" => "aarch64-unknown-linux-gnu",
        other => panic!("unreviewed Qwen architecture: {other}"),
    };
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["target"], target);
    assert_eq!(artifact["version"], "0.26.7");
    assert_eq!(
        artifact["sha256"]["cli-entry.js"],
        "89c392458627a27448ffaed32102554f0cb72a70f4ec1e60527fa56068afb387"
    );
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/qwen/linux.json"
    ))
    .unwrap();
    assert_eq!(artifact["native_sha256"], manifest["targets"][target]["files"]);
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (row, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["source"], "session_model");
        assert_eq!(row["model"], "coexistence-fixture/qwen3-coder-plus");
        assert_eq!(row["done"], 1);
        assert_eq!(row["errors"], 0);
        let calls = row["calls"].as_object().unwrap();
        let selected = matches!(phase, "mounted" | "rejected");
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        for call in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(call), selected, "{phase}: {call}");
        }
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
            assert!(row["model_requests"].as_u64().unwrap() > calls.len() as u64);
        }
    }
}
