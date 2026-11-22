use serde_json::{Value, json};
use std::{path::Path, process::Command};

#[test]
#[ignore = "requires a Linux shiju gateway and the reviewed Auggie npm runtime"]
fn acp_platform_auggie_contract() {
    let gateway = std::env::var_os("SHIJU_ACP_AUGGIE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_AUGGIE_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("auggie-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "auggie", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Auggie gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "0.37.1");
    assert_eq!(evidence["artifact"]["revision"], "auggie-0.37.1-model-source-v2");
    assert_eq!(evidence["artifact"]["entry"], "augment.mjs");
    assert_eq!(evidence["artifact"]["model_endpoint"], "/v1/chat/completions");
    assert_eq!(
        evidence["artifact"]["sha256"],
        json!({
            "augment.mjs": "c8d0c76c7ef7eaa1a4837f7cda2eec3eeecd9d8bf28da2db9697993d98300ba9",
            "session-model.mjs": "7d0afac3f9242e473a40a42f75aebf7bc48fd611aee50f375a04d92afc3fe162",
        })
    );

    let turns = evidence["turns"].as_array().unwrap();
    let phases = [
        ("baseline", false, 0, json!([])),
        ("mounted", true, 1, json!(["once"])),
        ("removed", false, 0, json!([])),
        ("rejected", true, 0, json!(["reject"])),
    ];
    assert_eq!(turns.len(), phases.len());
    for (turn, (phase, mounted, mcp_calls, decisions)) in turns.iter().zip(phases) {
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
        assert_eq!(turn["mcp_calls"], mcp_calls);
        assert_eq!(turn["mcp_decisions"], decisions);
        if mounted {
            assert_eq!(turn["mcp_bridge_call_id"], calls["mcp"]);
        }
        assert!(turn["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}

#[test]
fn acp_platform_auggie_contract_rejects_special_history_mode() {
    use crate::acp::model_source::{drivers::auggie::history_directory, profile::AcpLaunchProfile};
    use agent_config::schema::AcpAgentConfig;
    use model_transport::RelayIdentity;
    use std::os::unix::fs::PermissionsExt;

    let fixture = tempfile::tempdir().unwrap();
    let profile = AcpLaunchProfile::new(
        &AcpAgentConfig::default(),
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: fixture.path().display().to_string(),
            session: "history-permissions".into(),
            provider: "fixture".into(),
            model: "selected".into(),
            generation: 0,
        },
    )
    .unwrap();
    let history = history_directory(&profile).unwrap();
    std::fs::set_permissions(&history, std::fs::Permissions::from_mode(0o1700)).unwrap();
    let result = history_directory(&profile);
    std::fs::remove_dir(history).unwrap();
    assert!(result.unwrap_err().to_string().contains("insecure Auggie history directory"));
}
