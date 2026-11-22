use super::*;

#[test]
fn mimo_code_routes_only_through_the_reviewed_chat_transport() {
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        adapter: "openai-compatible".into(),
        url: "https://example.invalid/v1/chat/completions".into(),
        protocols: None,
    };
    assert_eq!(source_protocol("mimo-code", &api).unwrap(), model_protocols::ModelProtocol::Chat,);
    assert_eq!(verified_release("mimo-code"), Some("0.1.17"));
}

#[test]
fn mimo_code_rejects_an_unreviewed_binary_before_exposing_credentials() {
    let config = agent_config::schema::AcpAgentConfig {
        command: "mimo".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let identity = model_transport::RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: "session:mimo-code".into(),
        provider: "provider".into(),
        model: "provider/model".into(),
        generation: 0,
    };
    let mut profile = AcpLaunchProfile::new(&config, identity).unwrap();
    let error = mimo_code::configure(&mut profile, "http://127.0.0.1:12345/", "/workspace");
    #[cfg(target_os = "linux")]
    assert_eq!(
        error.unwrap_err().to_string(),
        "Managed MiMo Code requires reviewed Linux process and MCP sandboxing"
    );
    #[cfg(not(target_os = "linux"))]
    assert!(error.is_err());
    assert!(!profile.config().env.contains_key("SHIJU_MIMO_RELAY_TOKEN"));
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires a current Linux gateway and a privately built MiMo Code artifact"]
fn acp_platform_mimo_code_contract() {
    use serde_json::{Value, json};
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_MIMO_GATEWAY_BIN")
        .expect("set SHIJU_ACP_MIMO_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("mimo-code.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "mimo-code", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();

    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "MiMo Code gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let contract: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/mimo-code/linux.json"
    )))
    .unwrap();
    let target = format!("{}-unknown-linux-gnu", std::env::consts::ARCH);
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["target"], target);
    for field in ["version", "revision", "entry", "source_sha256", "patch_sha256", "lock_sha256"] {
        assert_eq!(artifact[field], contract[field], "artifact {field}");
    }
    assert_eq!(artifact["bun_sha256"], contract["targets"][target]["bun_sha256"]);
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        for name in ["native", "native_read"] {
            assert!(calls[name].as_str().is_some_and(|id| !id.is_empty()));
        }
        for name in ["selected", "selected_read"] {
            assert_eq!(calls.contains_key(name), selected);
        }
        let terminal_rejection = turn["terminal_mcp_rejection"] == true;
        assert!(!terminal_rejection || phase == "rejected");
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(turn["mcp_calls"], u64::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        if phase == "mounted" {
            assert_eq!(calls["mcp"], turn["mcp_bridge_call_id"]);
        }
        let continuations = calls.len() - usize::from(terminal_rejection);
        assert!(turn["model_requests"].as_u64().unwrap() > continuations as u64);
    }
}
