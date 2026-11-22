use agent_config::schema::AcpAgentConfig;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::acp::model_source::{configure_profile, drivers, launch_identity, prepare_profile};

#[path = "../../aisdk/convert_test_support.rs"]
mod models;

const AGENT: &str = "codex";
const RELAY: &str = "http://127.0.0.1:34567";
const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/codex/linux-engine.json"
));

#[tokio::test]
async fn acp_platform_codex_contract_rejects_ambient_entry() {
    let root = tempfile::tempdir().unwrap();
    let command = root.path().join("codex-acp");
    std::fs::write(&command, "").unwrap();
    let config = AcpAgentConfig { command: command.to_str().unwrap().into(), ..Default::default() };
    let model = models::test_model("fixture", "openai-responses", RELAY, "selected", "Selected");
    let identity = launch_identity(Default::default(), "project".into(), AGENT, &model, "ambient");
    assert!(prepare_profile(AGENT, &config, &model, identity, || root.path().into()).is_err());
}

#[tokio::test]
#[ignore = "requires the reviewed Linux runtime in an isolated acceptance configuration"]
async fn acp_platform_codex_contract_reviewed_launch_and_protocols() {
    let root = tempfile::tempdir().unwrap();
    let (entry, args) = acp_agent_registry::reviewed_agent_command(AGENT)
        .expect("reviewed Codex installation did not verify")
        .expect("reviewed Codex installation is missing");
    let entry_path = Path::new(&entry);
    assert!(entry_path.is_absolute());
    assert!(entry_path.ends_with("node_modules/@agentclientprotocol/codex-acp/dist/index.js"));
    let contract: Value = serde_json::from_str(MANIFEST).unwrap();
    let digest = hex::encode(Sha256::digest(std::fs::read(entry_path).unwrap()));
    assert_eq!(contract["bridge_sha256"].as_str(), Some(digest.as_str()));
    assert!(args.is_empty());
    let config = AcpAgentConfig { command: entry, args, ..Default::default() };

    for (adapter, path, conversion) in [
        ("openai-responses", "/v1/responses", false),
        ("openai-compatible", "/v1/chat/completions", true),
    ] {
        let model = models::test_model(
            "fixture",
            adapter,
            &format!("{RELAY}{path}"),
            "selected",
            "Selected",
        );
        let identity =
            launch_identity(Default::default(), "project".into(), AGENT, &model, adapter);
        let mut profile =
            prepare_profile(AGENT, &config, &model, identity, || root.path().into()).unwrap();
        assert_eq!(drivers::launch_route(AGENT, &profile).unwrap().conversion_required, conversion);
        assert_eq!(profile.config().command, config.command);
        configure_profile(
            AGENT,
            &mut profile,
            RELAY,
            root.path().to_str().unwrap(),
            &Default::default(),
        )
        .await
        .unwrap();
        assert_eq!(profile.config().args, ["cli", "--shiju-model-source"]);
        assert_eq!(profile.config().env["CODEX_PATH"], "");
        let native: Value = toml::from_str(
            &std::fs::read_to_string(profile.directory().join("config.toml")).unwrap(),
        )
        .unwrap();
        let provider = native["model_provider"].as_str().unwrap();
        assert_eq!(native["model"], "selected");
        assert_eq!(native["model_providers"][provider]["wire_api"], "responses");
        assert_eq!(native["model_providers"][provider]["base_url"], format!("{RELAY}/v1"));
        assert_eq!(native.get("web_search"), conversion.then_some(&json!("disabled")));
        assert_ne!(
            Path::new(&profile.config().env["SHIJU_CODEX_HISTORY_HOME"]),
            profile.directory()
        );

        // Exercise the host's ACP process and handshake with the actual reviewed bridge.
        let launch = profile.config();
        let client = acp_client::AcpClient::new(
            AGENT,
            acp_types::AcpAgentConfig {
                command: launch.command.clone(),
                args: launch.args.clone(),
                env: launch.env.clone(),
            },
        )
        .with_permission_mode(acp_types::PermissionMode::DenyAll);
        let started =
            tokio::time::timeout(std::time::Duration::from_secs(30), client.start()).await;
        let version = client.agent_version();
        client.close().await.unwrap();
        started.expect("ACP handshake timed out").expect("ACP handshake failed");
        assert_eq!(version.as_deref(), contract["bridge_version"].as_str());
    }
}
