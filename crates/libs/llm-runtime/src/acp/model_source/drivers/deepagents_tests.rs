use super::*;

#[test]
fn deepagents_selected_model_uses_chat_protocol() {
    let api = serde_json::from_value(serde_json::json!({
        "id": "fixture", "adapter": "openai", "url": "https://example.invalid/v1/chat/completions"
    }))
    .unwrap();
    assert_eq!(source_protocol("deepagents", &api).unwrap(), model_protocols::ModelProtocol::Chat);
}

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &agent_config::schema::AcpAgentConfig {
            command: "deepagents-acp".into(),
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session:deepagents".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile
}

#[test]
fn deepagents_private_model_configuration_is_revoked() {
    let mut p = profile();
    deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").unwrap();
    assert_eq!(p.config().args, ["--model", "openai:selected-model", "--workspace", "/workspace"]);
    assert_eq!(p.config().env["OPENAI_BASE_URL"], "http://127.0.0.1:12345/v1");
    assert_eq!(p.config().env["OPENAI_API_KEY"], p.binding.bearer_token());
    let binding = p.binding.clone();
    let token = binding.bearer_token().to_owned();
    let path = p.directory().to_owned();
    drop(p);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, binding.identity()));
}

#[test]
fn deepagents_rejects_overrides_and_nonlocal_relays() {
    for endpoint in
        ["https://example.com", "http://127.0.0.1:12345/v1", "http://user@127.0.0.1:12345"]
    {
        let mut p = profile();
        assert!(deepagents::configure(&mut p, endpoint, "/workspace").is_err());
        assert!(!p.config().env.contains_key("OPENAI_API_KEY"));
    }
    for key in ["OPENAI_API_BASE", "NODE_OPTIONS", "LANGSMITH_TRACING"] {
        let p = profile();
        let mut config = p.config().clone();
        config.env.remove(super::super::profile::REFERENCE_ENV);
        config.env.insert(key.into(), "override".into());
        let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
        p.model_supports_tool_call = true;
        assert!(deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
    }
    let p = profile();
    let mut config = p.config().clone();
    config.env.remove(super::super::profile::REFERENCE_ENV);
    config.args = vec!["--model".into(), "other".into()];
    let mut p = AcpLaunchProfile::new(&config, p.binding.identity().clone()).unwrap();
    assert!(deepagents::configure(&mut p, "http://127.0.0.1:12345", "/workspace").is_err());
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires a current Linux shiju gateway and an isolated reviewed DeepAgents install"]
fn acp_platform_deepagents_contract() {
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_DEEPAGENTS_GATEWAY_BIN")
        .expect("set SHIJU_ACP_DEEPAGENTS_GATEWAY_BIN to the current Linux gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let runtime = std::env::var_os("ACP_COEXISTENCE_DEEPAGENTS_RUNTIME")
        .expect("set ACP_COEXISTENCE_DEEPAGENTS_RUNTIME to the isolated v5 install");
    let runtime = Path::new(&runtime).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("deepagents-platform.json");

    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "deepagents", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .env("ACP_COEXISTENCE_DEEPAGENTS_RUNTIME", runtime)
        .env_remove("ACP_DEEPAGENTS_V4_BASELINE")
        .status()
        .unwrap();
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "DeepAgents platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "0.1.42");
    assert_eq!(evidence["artifact"]["revision"], 5);
    assert_eq!(evidence["artifact"]["entry"], "dist/cli.js");
    assert_eq!(evidence["artifact"]["model_endpoint"], "/v1/chat/completions");
    assert_eq!(
        evidence["artifact"]["sha256"]["dist/cli.js"],
        "55aace3a7b5eb077e8cb6314465f6fb965d8ee844d134e7f361bbca6f5396d4c"
    );

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
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), mounted);
        assert_eq!(calls.contains_key("selected_read"), mounted);
        assert_eq!(calls.contains_key("mcp"), mounted);
        assert_eq!(turn["mcp_calls"], u32::from(phase == "mounted"));
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
