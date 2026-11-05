#[test]
fn acp_skill_mount_kiro_selected_mounts_use_reviewed_native_contract() {
    assert!(super::super::validate("kiro", true).is_ok());
}

#[test]
fn acp_skill_mount_kiro_empty_selection_preserves_existing_cli_use() {
    assert!(super::super::validate("kiro", false).is_ok());
}

#[test]
fn acp_skill_mount_kiro_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = crate::config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_mount_kiro_unreviewed_executable_is_rejected_before_environment_changes() {
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
        "kiro-negative",
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = crate::config::schema::AcpAgentConfig {
        command: root.path().join("missing-kiro").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}
