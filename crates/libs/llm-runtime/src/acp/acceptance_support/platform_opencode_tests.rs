use agent_config::schema::AcpAgentConfig;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

use crate::acp::model_source::{configure_profile, drivers, launch_identity, prepare_profile};

use crate::convert_test_support as models;

const AGENT: &str = "opencode";
const RELAY: &str = "http://127.0.0.1:34567";

fn chat_model() -> model_types::Model {
    models::test_model(
        "fixture",
        "openai-compatible",
        &format!("{RELAY}/v1/chat/completions"),
        "selected",
        "Selected",
    )
}

fn linux_contract() -> (Value, &'static str) {
    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/opencode/linux.json"
    )))
    .unwrap();
    let target = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        arch => panic!("unexpected GNU architecture: {arch}"),
    };
    (manifest, target)
}

#[test]
fn acp_platform_opencode_contract_rejects_ambient_entry() {
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("opencode");
    std::fs::write(&binary, b"unreviewed").unwrap();
    let config = AcpAgentConfig {
        command: binary.to_str().unwrap().into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    let model = chat_model();
    let identity = launch_identity(Default::default(), "project".into(), AGENT, &model, "ambient");
    assert!(prepare_profile(AGENT, &config, &model, identity, || root.path().into()).is_err());
}

#[tokio::test]
#[ignore = "requires the reviewed Linux installation in an isolated acceptance configuration"]
async fn acp_platform_opencode_contract_reviewed_entry_and_chat_profile() {
    let root = tempfile::tempdir().unwrap();
    let (command, args) = acp_agent_registry::reviewed_agent_command(AGENT)
        .expect("OpenCode artifact verification failed")
        .expect("reviewed OpenCode is missing");
    assert_eq!(args, ["acp"]);
    let entry = Path::new(&command).canonicalize().unwrap();
    assert!(entry.ends_with("reviewed/opencode"));
    let (manifest, target) = linux_contract();
    let mut artifact = File::open(&entry).unwrap();
    let mut hash = Sha256::new();
    let mut block = [0; 64 * 1024];
    loop {
        let size = artifact.read(&mut block).unwrap();
        if size == 0 {
            break;
        }
        hash.update(&block[..size]);
    }
    let digest = hex::encode(hash.finalize());
    assert_eq!(manifest["targets"][target]["patched_sha256"].as_str(), Some(digest.as_str()));

    let config = AcpAgentConfig { command, args, ..Default::default() };
    let model = chat_model();
    let identity = launch_identity(Default::default(), "project".into(), AGENT, &model, "chat");
    let mut profile =
        prepare_profile(AGENT, &config, &model, identity, || root.path().into()).unwrap();
    let route = drivers::launch_route(AGENT, &profile).unwrap();
    assert!(!route.conversion_required);
    configure_profile(
        AGENT,
        &mut profile,
        RELAY,
        root.path().to_str().unwrap(),
        &Default::default(),
    )
    .await
    .unwrap();
    let env = &profile.config().env;
    let settings: Value = serde_json::from_str(&env["OPENCODE_CONFIG_CONTENT"]).unwrap();
    let provider = &settings["provider"]["session-provider"];
    assert_eq!(settings["model"], "session-provider/selected");
    assert_eq!(provider["npm"], "@ai-sdk/openai-compatible");
    assert_eq!(provider["options"]["baseURL"], format!("{RELAY}/v1"));
    assert_eq!(provider["options"]["apiKey"], "{env:VW_OPENCODE_RELAY_TOKEN}");
    assert!(Path::new(&env["XDG_DATA_HOME"]).starts_with(root.path()));
    assert_eq!(env["OPENCODE_DISABLE_PROJECT_CONFIG"], "1");
    eprintln!("OpenCode {} linux-{target} {digest} Chat Completions", manifest["version"]);
}

#[test]
#[ignore = "requires the current Linux gateway and privately installed OpenCode"]
fn acp_platform_opencode_contract_gateway_tools_and_revocation() {
    let gateway = std::env::var_os("SHIJU_ACP_OPENCODE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_OPENCODE_GATEWAY_BIN to the current Linux shiju gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("opencode-platform.json");
    let status = std::process::Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", AGENT, "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "OpenCode gateway probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    let (manifest, target) = linux_contract();
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["version"], manifest["version"]);
    assert_eq!(artifact["sha256"], manifest["targets"][target]["patched_sha256"]);
    assert_eq!(artifact["model_endpoint"], "/v1/chat/completions");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        assert_eq!(turn["mcp_calls"], u64::from(phase == "mounted"));
        let calls = turn["calls"].as_object().unwrap();
        for name in ["native", "native_read"] {
            assert!(calls[name].as_str().is_some_and(|id| !id.is_empty()));
        }
        for name in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(name), matches!(phase, "mounted" | "rejected"));
        }
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => serde_json::json!(["once"]),
                "rejected" => serde_json::json!(["reject"]),
                _ => serde_json::json!([]),
            }
        );
        assert!(
            turn["model_requests"].as_u64().unwrap()
                > (calls.len() - usize::from(turn["terminal_mcp_rejection"] == true)) as u64
        );
    }
}
