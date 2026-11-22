use crate::acp::model_source::{drivers, profile::AcpLaunchProfile};
use agent_config::schema::AcpAgentConfig;
use serde_json::Value;
use vw_api_acp_model::acp_model_source::{
    AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
};

#[test]
fn acp_platform_junie_contract() {
    let manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/junie/linux-artifacts.json"
    )))
    .unwrap();
    let target = &manifest["targets"][std::env::consts::ARCH];
    let (archive_arch, machine) = match std::env::consts::ARCH {
        "x86_64" => ("amd64", 62),
        "aarch64" => ("aarch64", 183),
        arch => panic!("Unexpected Junie GNU target {arch}"),
    };
    assert_eq!(manifest["version"], "4100.5");
    assert_eq!(manifest["reported_version"].as_str(), drivers::verified_release("junie"));
    assert_eq!(manifest["entry"], "junie-app/bin/junie");
    assert_eq!(manifest["args"], serde_json::json!(["--acp=true"]));
    assert_eq!(target["elf_machine"], machine);
    assert_eq!(
        target["archive_url"],
        format!(
            "https://github.com/JetBrains/junie/releases/download/4100.5/junie-release-4100.5-linux-{archive_arch}.zip"
        )
    );

    assert_eq!(
        drivers::capabilities("junie")[0],
        AcpModelSourceCapability::Unsupported {
            source: AcpModelSource::SessionModel,
            reason: AcpModelSourceRejectionReason::UnsupportedCli,
        }
    );
    let api = model_types::ApiInfo {
        id: "selected-model".into(),
        url: "http://127.0.0.1:34567/v1".into(),
        adapter: "openai-compatible".into(),
        protocols: None,
    };
    assert_eq!(
        drivers::source_protocol("junie", &api).unwrap(),
        model_protocols::ModelProtocol::Chat
    );
    let workspace = tempfile::tempdir().unwrap();
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: workspace.path().join("junie").to_string_lossy().into_owned(),
            args: vec!["--acp=true".into()],
            ..Default::default()
        },
        model_transport::RelayIdentity {
            tenant: Default::default(),
            project: "junie-platform".into(),
            session: "junie-platform".into(),
            provider: "provider".into(),
            model: "selected-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    let error = drivers::junie::configure(
        &mut profile,
        "http://127.0.0.1:34567",
        workspace.path().to_str().unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "Junie managed launch requires Linux process and MCP isolation");
    assert!(!profile.config().env.contains_key("SHIJU_JUNIE_CONFIG"));
    assert!(!profile.directory().join("junie.json").exists());
    assert!(profile.session_model_id.is_none());
}
