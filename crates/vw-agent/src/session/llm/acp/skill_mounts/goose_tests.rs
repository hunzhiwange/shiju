#[test]
fn acp_skill_mount_goose_accepts_reviewed_directory_contract() {
    assert!(super::super::validate("goose", true).is_ok());
    assert!(super::super::validate("goose", false).is_ok());
}

#[test]
fn acp_skill_mount_goose_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = crate::config::schema::AcpAgentConfig::default();
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
        &[vw_acp::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "goose-negative",
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = crate::config::schema::AcpAgentConfig {
        command: root.path().join("missing-goose").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}
