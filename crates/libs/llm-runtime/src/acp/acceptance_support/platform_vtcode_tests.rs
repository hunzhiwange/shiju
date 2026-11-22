use agent_config::schema::AcpAgentConfig;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};
use vw_api_acp_model::acp_model_source::{
    AcpModelSource as Source, AcpModelSourceCapability as Capability,
};

use crate::acp::model_source::{drivers, launch_identity, prepare_profile};

#[path = "../../aisdk/convert_test_support.rs"]
mod models;

const AGENT: &str = "vtcode";
const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/vtcode/linux.json"
));

#[test]
fn acp_platform_vtcode_contract_keeps_unreviewed_managed_access_closed() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("vtcode");
    fs::write(&entry, "").unwrap();
    assert_managed_access_closed(entry.to_str().unwrap().into(), vec!["acp".into()]);
}

#[tokio::test]
#[ignore = "requires the fixed native VT Code in an isolated Linux VIBEWINDOW_CONFIG_DIR"]
async fn acp_platform_vtcode_contract_resolves_and_initializes_fixed_native_entry() {
    let config_root = std::env::var_os("VIBEWINDOW_CONFIG_DIR")
        .expect("set VIBEWINDOW_CONFIG_DIR to the isolated VT Code configuration");
    let config_root = Path::new(&config_root).canonicalize().unwrap();
    let contract: Value = serde_json::from_str(MANIFEST).unwrap();
    let target = &contract["targets"][std::env::consts::ARCH];
    let expected = config_root
        .join("acp-runtime")
        .join(contract["directory"].as_str().unwrap())
        .join(contract["bin"].as_str().unwrap());
    let spec = acp_agent_registry::resolve_agent_spec(AGENT).unwrap();
    assert_eq!(Path::new(&spec.command), expected);
    assert_eq!(spec.args, ["acp"]);
    acp_agent_registry::verify_fixed_linux_cli_launch(AGENT, &spec.command, &spec.args).unwrap();
    assert!(
        acp_agent_registry::verify_fixed_linux_cli_launch(AGENT, &spec.command, &["chat".into()])
            .is_err()
    );
    assert_eq!(spec.env["VT_ACP_ENABLED"], "1");
    assert_eq!(spec.env["VT_ACP_ZED_ENABLED"], "1");

    let sha256 = hex::encode(Sha256::digest(fs::read(&expected).unwrap()));
    assert_eq!(target["executable_sha256"].as_str(), Some(sha256.as_str()));
    assert_managed_access_closed(spec.command.clone(), spec.args.clone());

    let home = tempfile::tempdir().unwrap();
    let mut env = spec.env;
    for key in ["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"] {
        env.insert(key.into(), home.path().to_str().unwrap().into());
    }
    let client = acp_client::AcpClient::new(
        AGENT,
        acp_types::AcpAgentConfig { command: spec.command, args: spec.args, env },
    )
    .with_permission_mode(acp_types::PermissionMode::DenyAll);
    let started = tokio::time::timeout(Duration::from_secs(30), client.start()).await;
    let version = client.agent_version();
    client.close().await.unwrap();
    started.expect("ACP initialize timed out").expect("ACP initialize failed");
    assert_eq!(version.as_deref(), contract["version"].as_str());
}

fn assert_managed_access_closed(command: String, args: Vec<String>) {
    let capabilities = drivers::capabilities(AGENT);
    assert!(capabilities.contains(&Capability::Supported { source: Source::CliLocal }));
    assert!(capabilities.iter().any(|capability| matches!(
        capability,
        Capability::Unsupported { source: Source::SessionModel, .. }
    )));
    assert!(crate::acp::skill_mounts::validate(AGENT, true).is_err());
    assert!(acp_agent_registry::verify_vtcode_mcp_executable(&command).is_err());
    assert!(acp_agent_registry::verify_vtcode_skill_executable(&command).is_err());

    let model = models::test_model(
        "fixture",
        "openai-compatible",
        "http://127.0.0.1:34567/v1",
        "selected",
        "Selected",
    );
    assert_eq!(
        drivers::source_protocol(AGENT, &model.api).unwrap(),
        model_protocols::ModelProtocol::Chat,
    );
    let config = AcpAgentConfig { command, args, ..Default::default() };
    let identity = launch_identity(Default::default(), "project".into(), AGENT, &model, "linux");
    assert!(
        prepare_profile(AGENT, &config, &model, identity, || {
            panic!("unreviewed VT Code entry reached private state preparation")
        })
        .is_err()
    );
}
