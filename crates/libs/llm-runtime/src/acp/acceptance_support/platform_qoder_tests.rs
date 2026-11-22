use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a current Linux gateway and an isolated, reviewed Qoder runtime"]
fn acp_platform_qoder_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_QODER_GATEWAY_BIN")
        .expect("set SHIJU_ACP_QODER_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("qoder-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "qoder", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Qoder platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(artifact["version"], "1.1.88");
    assert_eq!(artifact["revision"], "qoder-1.1.88-model-source-v3");
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/qoder/linux.json"
    ))
    .unwrap();
    assert_eq!(artifact["sha256"], contract["files"]);
    let target = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => panic!("unreviewed Qoder architecture: {other}"),
    };
    assert_eq!(artifact["ripgrep_sha256"], contract["ripgrep"][target]["sha256"]);

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (row, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["source"], "session_model");
        assert_eq!(row["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(row["done"], 1);
        assert_eq!(row["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = row["calls"].as_object().unwrap();
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), selected);
        assert_eq!(calls.contains_key("selected_read"), selected);
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(row["model_requests"], calls.len() as u64 + 1);
        assert_eq!(row["mcp_calls"], u32::from(phase == "mounted"));
        assert_eq!(
            row["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
    }
}
