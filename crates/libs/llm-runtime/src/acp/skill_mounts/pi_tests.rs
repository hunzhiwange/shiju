#[test]
fn acp_skill_mount_pi_accepts_reviewed_directory_contract() {
    assert!(super::super::validate("pi", true).is_ok());
    assert!(super::super::validate("pi", false).is_ok());
}

#[test]
fn acp_skill_mount_pi_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = agent_config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_mount_pi_unreviewed_executable_is_rejected_before_environment_changes() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: example\ndescription: fixture\n---\nBODY")
        .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "pi-negative",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig {
        command: root.path().join("missing-pi").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}

#[test]
fn acp_skill_coexistence_pi_restricts_the_exception_to_reviewed_read_mode() {
    use acp_types::PermissionMode::*;
    use vw_api_acp_model::acp_model_source::AcpModelSource::*;

    assert_eq!(
        super::restricted_launch("pi", SessionModel, Some(ApproveReads), false),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
    assert!(!super::restricted_launch("pi", CliLocal, Some(ApproveReads), false));
    assert!(!super::restricted_launch("pi", SessionModel, Some(DenyAll), false));
    assert!(!super::restricted_launch("pi", SessionModel, Some(ApproveReads), true));
    assert!(!super::restricted_launch("pi", SessionModel, Some(ApproveAll), false));
    assert!(!super::restricted_launch("other", SessionModel, Some(ApproveReads), false));
}

#[cfg(target_os = "linux")]
#[test]
fn acp_platform_pi_contract_denies_skill_view_without_reviewed_sandbox() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("fixture");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: fixture\ndescription: fixture\n---\nBODY")
        .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "fixture".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "pi-linux",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig::default();
    let error = super::super::configure_native_scanner("pi", &mut config, Some(&view)).unwrap_err();
    assert_eq!(
        error.to_string(),
        "acp option error: Pi skill mounts require a reviewed Linux sandbox"
    );
    assert!(!config.env.contains_key(super::ROOT_ENV));
    super::super::configure_native_scanner("pi", &mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[tokio::test]
#[ignore = "requires a private Linux npm prefix with the two fixed upstream Pi packages"]
async fn acp_platform_pi_contract_native() {
    use sha2::{Digest, Sha256};
    use std::{env, fs, path::Path, process::Command, time::Duration};

    let prefix = env::var_os("SHIJU_ACP_PI_NATIVE_PREFIX")
        .expect("set SHIJU_ACP_PI_NATIVE_PREFIX to the private npm prefix");
    let prefix = Path::new(&prefix).canonicalize().unwrap();
    const CHILD_ROOT: &str = "SHIJU_TEST_PI_NATIVE_ROOT";
    let Some(root) = env::var_os(CHILD_ROOT) else {
        let root = tempfile::tempdir().unwrap();
        for directory in ["home", "config", "workspace"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let paths = env::join_paths([prefix.join("bin")].into_iter().chain(env::split_paths(
            &env::var_os("PATH").expect("Node must be available on PATH"),
        )))
        .unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let test = format!("{module}::acp_platform_pi_contract_native");
        let output = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", &test, "--nocapture"])
            .env_clear()
            .env("PATH", paths)
            .env("HOME", root.path().join("home"))
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env("SHIJU_ACP_PI_NATIVE_PREFIX", prefix)
            .env(CHILD_ROOT, root.path())
            .current_dir(root.path().join("workspace"))
            .output()
            .unwrap();
        assert!(
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "Pi native contract failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };

    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../patches/acp-model-source/pi/linux.json"
    ))
    .unwrap();
    let root = Path::new(&root);
    for (record, command, digest) in
        [("adapter", "pi-acp", "upstream_entry_sha256"), ("engine", "pi", "entry_sha256")]
    {
        let record = &manifest[record];
        let package = prefix.join("lib/node_modules").join(record["package"].as_str().unwrap());
        let entry = package.join(record["entry"].as_str().unwrap());
        assert_eq!(prefix.join("bin").join(command).canonicalize().unwrap(), entry);
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(package.join("package.json")).unwrap()).unwrap();
        assert_eq!(metadata["name"], record["package"]);
        assert_eq!(metadata["version"], record["version"]);
        assert_eq!(
            hex::encode(Sha256::digest(fs::read(&entry).unwrap())),
            record[digest].as_str().unwrap()
        );
    }

    let spec = acp_agent_registry::resolve_agent_spec("pi").unwrap();
    assert_eq!(
        Path::new(&spec.command).canonicalize().unwrap(),
        prefix.join("lib/node_modules/pi-acp/dist/index.js")
    );
    assert!(spec.args.is_empty() && spec.env.is_empty());
    let client = acp_client::AcpClient::new(
        "pi",
        acp_types::AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env },
    )
    .with_full_access(true)
    .with_permission_mode(acp_types::PermissionMode::ApproveAll);
    let created = tokio::time::timeout(
        Duration::from_secs(30),
        client.create_session(root.join("workspace")),
    )
    .await;
    client.close().await.unwrap();
    assert!(!created.unwrap().unwrap().session_id.is_empty());
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
#[ignore = "requires a Linux gateway and a private reviewed Pi install"]
fn acp_platform_pi_contract() {
    use std::{path::Path, process::Command};

    let gateway = std::env::var_os("SHIJU_ACP_PI_GATEWAY_BIN")
        .expect("set SHIJU_ACP_PI_GATEWAY_BIN to the Linux ShiJu gateway");
    let gateway = Path::new(&gateway).canonicalize().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let temp = tempfile::tempdir().unwrap();
    let report = temp.path().join("pi-platform.json");
    let status = Command::new("python3")
        .arg(root.join("scripts/verify_acp_skill_coexistence.py"))
        .args(["--agent", "pi", "--gateway-bin"])
        .arg(gateway)
        .arg("--output")
        .arg(&report)
        .status()
        .unwrap();
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(status.success(), "Pi gateway contract failed at {}: {evidence}", evidence["stage"]);
    assert_eq!(evidence["passed"], true);
    assert_eq!(evidence["platform"], format!("Linux-{}", std::env::consts::ARCH));
    assert_eq!(evidence["artifact"]["version"], "0.0.22");
    assert_eq!(evidence["artifact"]["engine_version"], "0.73.1");
    assert_eq!(evidence["artifact"]["entry"], "dist/index.js");
    assert_eq!(evidence["artifact"]["model_protocol"], "openai-chat-completions");
    assert_eq!(
        evidence["artifact"]["engine_sha256"],
        "e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa"
    );
    let turns = evidence["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 4);
    for (turn, phase) in turns.iter().zip(["baseline", "mounted", "removed", "rejected"]) {
        assert_eq!(turn["phase"], phase);
        assert_eq!(turn["source"], "session_model");
        assert_eq!(turn["model"], "coexistence-fixture/deepseek-v4.1-flash-1017");
        assert_eq!(turn["done"], 1);
        assert_eq!(turn["errors"], 0);
        let selected = matches!(phase, "mounted" | "rejected");
        let calls = turn["calls"].as_object().unwrap();
        assert!(calls.contains_key("native"));
        assert!(calls.contains_key("native_read"));
        assert_eq!(calls.contains_key("selected"), selected);
        assert_eq!(calls.contains_key("selected_read"), selected);
        assert_eq!(calls.contains_key("mcp"), selected);
        assert_eq!(turn["mcp_calls"], if phase == "mounted" { 1 } else { 0 });
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
