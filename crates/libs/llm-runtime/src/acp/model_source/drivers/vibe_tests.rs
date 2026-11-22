use super::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;

fn profile() -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new(
        &AcpAgentConfig { command: "vibe-acp".into(), ..Default::default() },
        RelayIdentity {
            tenant: Default::default(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "chosen-\"模型".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.model_api = Some(model_types::ApiInfo {
        protocols: None,
        id: "selected".into(),
        url: "https://example.invalid/v1".into(),
        adapter: "openai-compatible".into(),
    });
    profile.model_supports_temperature = true;
    profile
}

#[test]
fn acp_model_source_vibe_private_binding_pins_primary_and_auxiliary_models() {
    let mut profile = profile();
    profile.model_context_limit = 1_000_000;
    profile.model_output_limit = 384_000;
    configure(&mut profile, "http://127.0.0.1:12345").unwrap();
    let path = profile.directory().join("session-model.toml");
    let value: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for model in [&value["models"][0], &value["compaction_model"]] {
        assert_eq!(model["max_context_length"].as_integer(), Some(1_000_000));
        assert_eq!(model["max_output_tokens"].as_integer(), Some(384_000));
    }
    assert_eq!(value["active_model"].as_str(), Some("session-selected"));
    assert_eq!(value["models"][0]["name"].as_str(), Some("chosen-\"模型"));
    assert_eq!(value["compaction_model"]["name"].as_str(), Some("chosen-\"模型"));
    assert_eq!(value["utility_models"]["title"].as_str(), Some("session-selected"));
    assert_eq!(value["utility_models"]["smart_approve"].as_str(), Some("session-selected"));
    assert!(profile.config().args.iter().any(|arg| arg == "--session-model-config"));
    assert!(!profile.config().args.join(" ").contains(profile.binding.bearer_token()));
    assert_eq!(profile.config().env["VIBE_HOME"], profile.directory().to_string_lossy());
    let binding = profile.binding.clone();
    let token = binding.bearer_token().to_owned();
    let identity = binding.identity().clone();
    drop(profile);
    assert!(!path.exists());
    assert!(!binding.accepts(&token, &identity));
}

#[test]
fn acp_model_source_vibe_rejects_unsupported_protocol_before_publication() {
    let mut profile = profile();
    profile.model_api.as_mut().unwrap().adapter = "unknown".into();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

#[test]
fn acp_model_source_vibe_rejects_conflicting_native_home() {
    let mut profile = profile();
    profile
        .overlay_environment(std::collections::HashMap::from([(
            "VIBE_HOME".into(),
            "/conflicting-native-config".into(),
        )]))
        .unwrap();
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

#[test]
fn acp_model_source_vibe_rejects_temperature_and_missing_metadata() {
    let mut profile = profile();
    profile.model_supports_temperature = false;
    assert!(
        configure(&mut profile, "http://127.0.0.1:12345")
            .unwrap_err()
            .to_string()
            .contains("temperature")
    );
    assert!(!profile.directory().join("session-model.toml").exists());
    profile.model_api = None;
    assert!(configure(&mut profile, "http://127.0.0.1:12345").is_err());
}

#[test]
fn acp_model_source_vibe_selects_native_protocol_and_rejects_remote_relay() {
    for (adapter, expected, base) in [
        ("openai-compatible", "openai", "http://127.0.0.1:12345/v1"),
        ("openai-responses", "openai-responses", "http://127.0.0.1:12345/v1"),
        ("anthropic", "anthropic", "http://127.0.0.1:12345"),
    ] {
        let mut profile = profile();
        profile.model_api.as_mut().unwrap().adapter = adapter.into();
        configure(&mut profile, "http://127.0.0.1:12345").unwrap();
        let value: toml::Value = toml::from_str(
            &std::fs::read_to_string(profile.directory().join("session-model.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(value["providers"][0]["api_style"].as_str(), Some(expected));
        assert_eq!(value["providers"][0]["api_base"].as_str(), Some(base));
        assert!(
            !std::fs::read_to_string(profile.directory().join("session-model.toml"))
                .unwrap()
                .contains("example.invalid")
        );
    }
    let mut profile = profile();
    assert!(configure(&mut profile, "https://example.invalid").is_err());
    assert!(!profile.directory().join("session-model.toml").exists());
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires a Linux gateway and the reviewed Vibe installation"]
fn acp_platform_vibe_contract() {
    use serde_json::{Value, json};
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_VIBE_GATEWAY_BIN")
        .expect("set SHIJU_ACP_VIBE_GATEWAY_BIN to the current Linux shiju binary");
    let runtime = std::env::var_os("VIBE_COEXISTENCE_RUNTIME")
        .expect("set VIBE_COEXISTENCE_RUNTIME to the installed reviewed Vibe runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let output_dir = tempfile::tempdir().unwrap();
    let output = output_dir.path().join("vibe-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "vibe", "--gateway-bin"])
        .arg(Path::new(&gateway).canonicalize().unwrap())
        .arg("--output")
        .arg(&output)
        .env("VIBE_COEXISTENCE_RUNTIME", Path::new(&runtime).canonicalize().unwrap())
        .status()
        .unwrap();
    let evidence: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(status.success(), "Vibe platform probe failed: {evidence}");
    assert_eq!(evidence["passed"], true);
    let arch = std::env::consts::ARCH;
    assert_eq!(evidence["platform"], format!("Linux-{arch}"));
    let artifact = &evidence["artifact"];
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../../../../../patches/acp-model-source/vibe/linux.json"
    ))
    .unwrap();
    let target = format!("{arch}-unknown-linux-gnu");
    assert_eq!(artifact["target"], target);
    assert_eq!(artifact["version"], "3.0.4");
    assert_eq!(artifact["entry"], "bin/vibe-acp");
    assert_eq!(artifact["entry_sha256"], manifest["launcher_sha256"]);
    assert_eq!(artifact["wheel_sha256"], manifest["targets"][&target]["wheel_sha256"]);
    assert_eq!(artifact["model_protocol"], "POST /v1/chat/completions");
    assert_eq!(artifact["mcp_transport"], "http");

    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (row, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(row["phase"], phase);
        assert_eq!(row["source"], "session_model");
        assert_eq!(row["model"], "deepseek-v4.1-flash-1017");
        assert_eq!(row["done"], 1);
        assert_eq!(row["errors"], 0);
        assert_eq!(row["skill_approvals"], json!(["native_read"]));
        let calls = row["calls"].as_object().unwrap();
        assert!(calls.contains_key("native") && calls.contains_key("native_read"));
        let mounted = matches!(phase, "mounted" | "rejected");
        for call in ["selected", "selected_read", "mcp"] {
            assert_eq!(calls.contains_key(call), mounted, "{phase}: {call}");
        }
        assert_eq!(row["mcp_calls"], u32::from(phase == "mounted"));
        assert_eq!(
            row["mcp_decisions"],
            match phase {
                "mounted" => json!(["once"]),
                "rejected" => json!(["reject"]),
                _ => json!([]),
            }
        );
        assert!(row["model_requests"].as_u64().unwrap() > calls.len() as u64);
    }
}
