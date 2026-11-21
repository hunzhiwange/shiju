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
    let mut config = agent_config::schema::AcpAgentConfig::default();
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
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "openclaw-negative",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig {
        command: root.path().join("missing-openclaw").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}

#[test]
fn acp_skill_coexistence_openclaw_restricted_launch_requires_interactive_reads() {
    use acp_types::PermissionMode;
    use vw_api_acp_model::acp_model_source::AcpModelSource::{CliLocal, SessionModel};

    let reads = Some(PermissionMode::ApproveReads);
    let launch = super::restricted_launch;
    assert_eq!(
        launch(SessionModel, reads, false, false, false),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
    assert!(!launch(CliLocal, reads, false, false, false));
    assert!(!launch(SessionModel, reads, true, false, false));
    assert!(!launch(SessionModel, reads, false, true, false));
    assert!(!launch(SessionModel, reads, false, false, true));
    assert!(!launch(SessionModel, Some(PermissionMode::DenyAll), false, false, false));
}
