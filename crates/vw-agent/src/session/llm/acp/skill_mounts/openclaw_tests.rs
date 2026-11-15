#[test]
fn acp_skill_mount_openclaw_selected_mounts_require_owned_gateway_contract() {
    assert!(super::super::validate("openclaw", true).is_ok());
}

#[test]
fn acp_skill_mount_openclaw_empty_selection_preserves_existing_cli_use() {
    assert!(super::super::validate("openclaw", false).is_ok());
}

#[test]
fn acp_skill_mount_openclaw_empty_selection_disables_native_skill_roots() {
    let mut config = crate::config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_mount_openclaw_unreviewed_executable_cannot_receive_mounts() {
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
        "openclaw-negative",
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = crate::config::schema::AcpAgentConfig {
        command: root.path().join("missing-openclaw").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}
