use agent_config::schema::AcpAgentConfig;
use serde_json::{Value, json};
use std::{path::Path, process::Command};

use crate::acp::model_source::{launch_identity, prepare_profile};

#[path = "../../aisdk/convert_test_support.rs"]
mod models;

const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/cursor/linux-artifacts.json"
));

#[test]
fn acp_platform_cursor_contract_rejects_unreviewed_launch() {
    let temp = tempfile::tempdir().unwrap();
    let entry = temp.path().join("dist-package/cursor-agent");
    std::fs::create_dir(entry.parent().unwrap()).unwrap();
    std::fs::write(&entry, "#!/bin/sh\nexit 1\n").unwrap();
    let config = AcpAgentConfig {
        command: entry.to_str().unwrap().into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let model = models::test_model(
        "fixture",
        "openai-compatible",
        "http://127.0.0.1:34567/v1",
        "selected",
        "Selected",
    );
    let identity =
        launch_identity(Default::default(), "project".into(), "cursor", &model, "ambient");
    assert!(prepare_profile("cursor", &config, &model, identity, || temp.path().into()).is_err());
}

#[test]
#[ignore = "requires a Linux ShiJu gateway and the fixed Cursor supplier archive"]
fn acp_platform_cursor_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_CURSOR_GATEWAY_BIN")
        .expect("set SHIJU_ACP_CURSOR_GATEWAY_BIN to the current Linux gateway binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("cursor-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "cursor", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Cursor gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let contract = &manifest["targets"][std::env::consts::ARCH];
    let artifact = &evidence["artifact"];
    let entry = contract["bin"].as_str().unwrap();
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(artifact["entry"], contract["bin"]);
    assert_eq!(artifact["version"], manifest["version"]);
    assert_eq!(artifact["archive_sha256"], contract["archive_sha256"]);
    assert_eq!(artifact["entry_sha256"], contract["files"][entry]);
    assert_eq!(artifact["node_sha256"], contract["files"]["dist-package/node"]);

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
        // Only authenticated Chat Completions requests reach the fixture counter.
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
