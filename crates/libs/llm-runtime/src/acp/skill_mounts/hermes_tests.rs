#[test]
fn acp_skill_mount_hermes_accepts_reviewed_directory_contract() {
    assert!(super::super::validate("hermes", true).is_ok());
    assert!(super::super::validate("hermes", false).is_ok());
}

#[test]
fn acp_skill_mount_hermes_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = agent_config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_mount_hermes_unreviewed_executable_is_rejected_before_environment_changes() {
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
        "hermes-negative",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig {
        command: root.path().join("missing-hermes").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_hermes_keeps_native_root_and_only_appends_active_view() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().canonicalize().unwrap();
    let native = workspace.join(".hermes/skills");
    let home = workspace.join("private");
    std::fs::create_dir_all(&native).unwrap();
    std::fs::create_dir(&home).unwrap();
    let roots =
        super::publish_skills(&home, &workspace, None::<&super::super::MountedSkills>).unwrap();
    assert_eq!(roots, vec![native.clone()]);
    assert_eq!(home.join("skills").canonicalize().unwrap(), native);

    let selected = workspace.join("selected");
    std::fs::create_dir(&selected).unwrap();
    std::fs::write(
        selected.join("SKILL.md"),
        "---\nname: selected\ndescription: fixture\n---\nBODY",
    )
    .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_string_lossy().into(),
        }],
        &workspace,
        "hermes-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mounted_home = workspace.join("mounted-home");
    std::fs::create_dir(&mounted_home).unwrap();
    let roots = super::publish_skills(&mounted_home, &workspace, Some(&view)).unwrap();
    assert_eq!(roots, vec![native, selected, view.0.root().canonicalize().unwrap()]);
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_hermes_rejects_a_link_as_the_native_root() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = temporary.path().canonicalize().unwrap();
    let home = workspace.join("private");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(workspace.join(".hermes")).unwrap();
    std::os::unix::fs::symlink(&home, workspace.join(".hermes/skills")).unwrap();
    assert!(
        super::publish_skills(&home, &workspace, None::<&super::super::MountedSkills>).is_err()
    );
    assert!(!home.join("skills").exists());
}
