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

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[tokio::test]
#[ignore = "requires the pinned official TRAE archive in a disposable GNU Linux container"]
async fn acp_platform_trae_contract_cli_local() {
    use acp_agent_registry::{install_builtin_agent, resolve_agent_spec, reviewed_agent_command};
    use acp_client::AcpClient;
    use acp_types::{AcpAgentConfig, PermissionMode};
    use agent_client_protocol::{McpServer, McpServerStdio};
    use std::{env, fs, path::Path, process::Command, time::Duration};

    const FIXTURE_ROOT: &str = "SHIJU_TEST_TRAE_CLI_LOCAL_ROOT";
    let Some(root) = env::var_os(FIXTURE_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        for name in ["home", "config", "workspace", "tmp"] {
            fs::create_dir(root.path().join(name)).unwrap();
        }
        let module = module_path!().split_once("::").unwrap().1;
        let test = format!("{module}::acp_platform_trae_contract_cli_local");
        let status = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", &test])
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("HOME", root.path().join("home"))
            .env("TMPDIR", root.path().join("tmp"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env(FIXTURE_ROOT, root.path())
            .current_dir(root.path().join("workspace"))
            .status()
            .unwrap();
        assert!(status.success(), "isolated TRAE cli_local contract failed");
        return;
    };

    let root = Path::new(&root);
    let workspace = root.join("workspace");
    install_builtin_agent("trae").await.unwrap();
    let entry = root
        .join("config/acp-runtime/trae-0.121.3-linux-native-v1/traecli")
        .canonicalize()
        .unwrap();
    let spec = resolve_agent_spec("trae").unwrap();
    assert_eq!(Path::new(&spec.command), entry.as_path());
    assert_eq!(spec.args, ["acp", "serve"]);
    assert!(spec.env.is_empty());
    assert_eq!(reviewed_agent_command("trae").unwrap(), None);
    assert!(AcpClient::ensure_desktop_native_access("trae", false).is_err());

    let ordinary = AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env };
    let client =
        |config| AcpClient::new("trae", config).with_permission_mode(PermissionMode::DenyAll);
    let timeout = Duration::from_secs(30);
    let native = client(ordinary.clone());
    let result = tokio::time::timeout(timeout, native.create_session(&workspace)).await;
    native.close().await.unwrap();
    assert!(!result.unwrap().unwrap().session_id.is_empty());

    let marker = root.join("mcp-started");
    let mcp = McpServer::Stdio(
        McpServerStdio::new("unreviewed_mcp", "/usr/bin/touch")
            .args(vec![marker.to_str().unwrap().into()]),
    );
    let mounted = client(ordinary.clone()).with_mcp_servers(vec![mcp]);
    let result = tokio::time::timeout(timeout, mounted.create_session(&workspace)).await;
    mounted.close().await.unwrap();
    let error = result.unwrap().unwrap_err().to_string();
    assert!(error.contains("Session MCP has no verified Linux launch boundary"), "{error}");
    assert!(!marker.exists());

    let mut managed = ordinary.clone();
    managed.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "unissued-fixture-reference".into());
    let managed = client(managed);
    let result = tokio::time::timeout(timeout, managed.create_session(&workspace)).await;
    managed.close().await.unwrap();
    let error = result.unwrap().unwrap_err().to_string();
    assert!(error.contains("no verified managed Linux launch"), "{error}");

    fs::write(&entry, b"unreviewed executable").unwrap();
    let replaced = client(ordinary);
    let result = tokio::time::timeout(timeout, replaced.create_session(&workspace)).await;
    replaced.close().await.unwrap();
    let error = result.unwrap().unwrap_err().to_string();
    assert!(error.contains("Fixed Linux ACP executable checksum mismatch"), "{error}");
}
