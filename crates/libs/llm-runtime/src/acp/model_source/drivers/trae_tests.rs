use super::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;

#[test]
fn trae_unreviewed_or_conflicting_launch_cannot_publish_credentials() {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig {
            command: "traecli".into(),
            args: vec!["acp".into(), "serve".into()],
            ..Default::default()
        },
        RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "trae-rejected".into(),
            provider: "provider".into(),
            model: "model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_supports_tool_call = true;
    profile.model_supports_temperature = true;
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    let rejection = configure(&mut profile, "http://127.0.0.1:12345").unwrap_err();
    if cfg!(target_os = "linux") {
        assert!(rejection.to_string().contains("relay-only network isolation"));
    }
    profile
        .overlay_environment(HashMap::from([("COCO_NEXTCODE_URL".into(), "conflict".into())]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert_eq!(std::fs::read_dir(profile.directory()).unwrap().count(), 0);
    assert!(!profile.config().env.contains_key("VW_TRAE_RELAY_TOKEN"));
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires the current Linux gateway and a reviewed TRAE session-model runtime"]
fn acp_platform_trae_contract() {
    use serde_json::{Value, json};
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_TRAE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_TRAE_GATEWAY_BIN to the current Linux shiju binary");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("trae-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "trae", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "TRAE platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let artifact = &evidence["artifact"];
    assert_eq!(artifact["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(artifact["revision"], "trae-0.121.3-linux-model-source-v1");

    let contract: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../patches/acp-model-source/trae/linux.json"
    )))
    .unwrap();
    let target = &contract["targets"][std::env::consts::ARCH];
    assert_eq!(artifact["version"], contract["version"]);
    assert_eq!(artifact["entry"], contract["entry"]);
    assert_eq!(artifact["args"], contract["args"]);
    assert_eq!(artifact["archive_sha256"], target["archive_sha256"]);
    assert_eq!(artifact["sha256"]["native/trae-cli"], target["executable_sha256"]);
    assert_eq!(artifact["elf_machine"], target["elf_machine"]);

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
        for name in ["native", "native_read"] {
            assert!(calls.contains_key(name), "{phase}: missing {name}");
        }
        for name in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(name), mounted, "{phase}: {name}");
        }
        let terminal_rejection = phase == "rejected" && turn["terminal_mcp_rejection"] == true;
        assert_eq!(turn["model_requests"], calls.len() as u64 + 1 - u64::from(terminal_rejection));
        assert_eq!(turn["mcp_calls"], u64::from(phase == "mounted"));
        assert_eq!(
            turn["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
    }
    // The provider fixture rejects every request outside this protocol.
    eprintln!("TRAE platform artifact {artifact} used POST /v1/chat/completions");
}
