#[test]
fn acp_skill_mount_goose_accepts_reviewed_directory_contract() {
    assert!(super::super::validate("goose", true).is_ok());
    assert!(super::super::validate("goose", false).is_ok());
}

#[test]
fn acp_skill_coexistence_goose_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = agent_config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_mount_goose_unreviewed_executable_is_rejected_before_environment_changes() {
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
        "goose-negative",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig {
        command: root.path().join("missing-goose").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}

#[test]
fn acp_skill_coexistence_goose_selects_only_the_reviewed_view() {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: example\ndescription: fixture\n---\nBODY")
        .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        directory.path(),
        "goose-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    // Exercise receipt validation with bytes local to the test. The real
    // native entry point is covered separately by the gateway probe.
    let entry = directory.path().join("goose");
    let artifact = b"goose fixture";
    std::fs::write(&entry, artifact).unwrap();
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../acp-agent-registry/src/managed_runtime/contracts.json"
    ))
    .unwrap();
    let mut receipt = serde_json::json!({
        "version": contract["goose"]["version"],
        "source_sha256": contract["goose"]["source_sha256"],
        "patch_sha256": "3a47bbbdcf861d680ff051879d81d23ef872d035a01a0549f9cedecd65b54e06",
        "sha256": hex::encode(Sha256::digest(artifact)),
    });
    let receipt_path = directory.path().join("build-receipt.json");
    let mut config = agent_config::schema::AcpAgentConfig {
        command: entry.to_string_lossy().into_owned(),
        env: [("GOOSE_PATH_ROOT".into(), "private-config".into())].into(),
        ..Default::default()
    };
    std::fs::write(&receipt_path, receipt.to_string()).unwrap();
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));

    receipt["patch_sha256"] = contract["goose"]["patch_sha256"].clone();
    std::fs::write(&receipt_path, receipt.to_string()).unwrap();
    super::configure(&mut config, Some(&view)).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], view.0.root().to_str().unwrap());
    assert_eq!(config.env["GOOSE_PATH_ROOT"], "private-config");
}
