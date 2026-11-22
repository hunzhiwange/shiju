use agent_config::schema::AcpAgentConfig;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

use crate::acp::model_source::{drivers, launch_identity, prepare_profile};

#[path = "../../aisdk/convert_test_support.rs"]
mod models;

const AGENT: &str = "stakpak";
const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../patches/acp-model-source/stakpak/linux-cli.json"
));

#[test]
#[ignore = "requires a Linux gateway and a reviewed Stakpak source build in a private directory"]
fn acp_platform_stakpak_contract_gateway_coexistence() {
    let gateway = std::env::var_os("SHIJU_ACP_STAKPAK_GATEWAY_BIN")
        .expect("set SHIJU_ACP_STAKPAK_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("stakpak-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", AGENT, "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Stakpak platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(artifact["version"], "0.3.88");
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");
    assert_eq!(
        artifact["source_sha256"],
        "7ffae3233a6d97d4bf142392ce747a720b6c94717259ee6527deed91664eeb00"
    );
    assert_eq!(
        artifact["patch_sha256"],
        "4873acd8bbdbcb0224db458256821e2cde986cee2298b7bc7433ec5bb616d7c4"
    );

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

#[test]
#[ignore = "requires the pinned Linux CLI in an isolated VIBEWINDOW_CONFIG_DIR"]
fn acp_platform_stakpak_contract_resolves_pinned_cli_without_managed_access() {
    let config_root = std::env::var_os("VIBEWINDOW_CONFIG_DIR")
        .expect("set VIBEWINDOW_CONFIG_DIR to the isolated Stakpak acceptance configuration");
    let config_root = Path::new(&config_root).canonicalize().unwrap();
    let contract: Value = serde_json::from_str(MANIFEST).unwrap();
    let target = &contract["targets"][std::env::consts::ARCH];
    let expected = config_root
        .join("acp-runtime")
        .join(contract["directory"].as_str().unwrap())
        .join("stakpak");

    let spec = acp_agent_registry::resolve_agent_spec(AGENT).unwrap();
    assert_eq!(Path::new(&spec.command), expected);
    assert_eq!(spec.args, ["acp"]);

    let sha256 = hex::encode(Sha256::digest(fs::read(&expected).unwrap()));
    assert_eq!(target["executable_sha256"].as_str(), Some(sha256.as_str()));
    assert_managed_launch_rejected(spec.command, spec.args);
}

#[test]
fn acp_platform_stakpak_contract_rejects_ambient_managed_entry() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("stakpak");
    fs::write(&entry, "").unwrap();
    assert_managed_launch_rejected(entry.to_str().unwrap().into(), vec!["acp".into()]);
}

fn assert_managed_launch_rejected(command: String, args: Vec<String>) {
    use vw_api_acp_model::acp_model_source::{
        AcpModelSource as Source, AcpModelSourceCapability as Capability,
    };
    let capabilities = drivers::capabilities(AGENT);
    assert!(capabilities.contains(&Capability::Supported { source: Source::CliLocal }));
    assert!(capabilities.iter().any(|capability| matches!(
        capability,
        Capability::Unsupported { source: Source::SessionModel, .. }
    )));

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
            panic!("unreviewed Stakpak entry cannot prepare private state")
        })
        .is_err()
    );
}
