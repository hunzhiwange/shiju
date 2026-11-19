use super::{drivers, launch_identity, prepare_profile};
use agent_config::schema::AcpAgentConfig;
use std::{path::Path, process::Command};
use vw_api_acp_model::acp_model_source::{
    AcpModelSource as Source, AcpModelSourceCapability as Capability,
    AcpModelSourceRejectionReason as Reason,
};

#[path = "../../aisdk/convert_test_support.rs"]
mod models;

const AGENT: &str = "glm-acp-agent";
const SCENARIO: &str = "SHIJU_GLM_CAPABILITY_TEST_SCENARIO";

fn assert_glm_capabilities(supported: bool) {
    let session = if supported {
        Capability::Supported { source: Source::SessionModel }
    } else {
        Capability::Unsupported { source: Source::SessionModel, reason: Reason::UnsupportedCli }
    };
    assert_eq!(
        drivers::capabilities(AGENT),
        [session, Capability::Supported { source: Source::CliLocal }]
    );
}

fn model() -> model_types::Model {
    models::test_model(
        "test-provider",
        "openai-compatible",
        "https://example.test/v1",
        "test-model",
        "Test Model",
    )
}

fn registered_config() -> AcpAgentConfig {
    let (agent, config) = crate::acp::config::lookup_acp_command(
        &Default::default(),
        &model(),
        &serde_json::json!({ "acp_agent": AGENT }),
    )
    .unwrap();
    assert_eq!(agent, AGENT);
    config
}

fn prepare(
    config: &AcpAgentConfig,
    root: &Path,
) -> anyhow::Result<super::profile::AcpLaunchProfile> {
    let model = model();
    let identity = launch_identity(Default::default(), "project".into(), AGENT, &model, "session");
    prepare_profile(AGENT, config, &model, identity, || root.to_path_buf())
}

#[tokio::test]
async fn glm_model_source_capability_matches_managed_launch() {
    let test = std::thread::current().name().unwrap().to_owned();
    let test_binary = std::env::current_exe().unwrap();

    // Keep registry and executable discovery overrides local to the child process.
    let Some(scenario) = std::env::var_os(SCENARIO) else {
        let temp = tempfile::tempdir().unwrap();
        let mut command = Command::new(test_binary);
        command
            .args(["--exact", &test, "--nocapture"])
            .env(SCENARIO, "install")
            .env("VIBEWINDOW_CONFIG_DIR", temp.path().join("GLM 受管"));
        if cfg!(windows) {
            command.env("LOCALAPPDATA", temp.path());
        }
        let result = command.status().unwrap();
        assert!(result.success(), "{result}");
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("VIBEWINDOW_CONFIG_DIR").unwrap());

    if scenario == "relative-node" {
        let spec = registered_config();
        assert_glm_capabilities(false);
        assert!(prepare(&spec, &root).err().unwrap().to_string().contains("Node.js"));
        return;
    }

    assert_glm_capabilities(false);
    assert!(
        prepare(&AcpAgentConfig { command: "npx".into(), ..Default::default() }, &root).is_err()
    );
    if !cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows")) {
        return;
    }

    // Use the same fixed supplier package, patch application and artifact hashes as task 2.
    acp_agent_registry::install_builtin_agent(AGENT).await.unwrap();
    let spec = registered_config();
    assert_glm_capabilities(true);
    let mut profile = prepare(&spec, &root).unwrap();
    drivers::glm_acp_agent::configure(&mut profile, "http://127.0.0.1:34567").unwrap();
    assert_eq!(profile.config().env["ACP_GLM_BASE_URL"], "http://127.0.0.1:34567/v1");

    let bin = root.join("relative-bin");
    std::fs::create_dir(&bin).unwrap();
    std::fs::write(bin.join(if cfg!(windows) { "node.exe" } else { "node" }), "").unwrap();
    let result = Command::new(test_binary)
        .args(["--exact", &test, "--nocapture"])
        .env(SCENARIO, "relative-node")
        .env("PATH", "relative-bin")
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(result.success(), "{result}");

    let legacy_args =
        AcpAgentConfig { args: vec!["-y".into(), "glm-acp-agent@1.17.1".into()], ..spec.clone() };
    assert!(prepare(&legacy_args, &root).is_err());

    std::fs::write(&spec.command, "tampered").unwrap();
    assert_glm_capabilities(false);
    assert!(prepare(&spec, &root).is_err());
}

#[test]
fn glm_model_source_capability_preserves_other_platform_gates() {
    let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Capability::Supported { source: Source::SessionModel }
    } else {
        Capability::Unsupported { source: Source::SessionModel, reason: Reason::UnsupportedCli }
    };
    assert_eq!(drivers::capabilities("kimi")[0], expected);
}
