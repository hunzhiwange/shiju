use agent_config::schema::AcpAgentConfig;

#[test]
fn acp_skill_coexistence_cursor_preserves_empty_config_and_rejects_override() {
    assert!(super::super::validate("cursor", true).is_ok());
    let mut config = AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert!(!config.env.contains_key(super::ROOT_ENV));
    config.env.insert(super::ROOT_ENV.into(), "/untrusted".into());
    assert!(super::configure(&mut config, None).is_err());
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_cursor_adds_only_the_trusted_view() {
    let workspace = tempfile::tempdir().unwrap();
    let source = workspace.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: example\ndescription: fixture\n---\nRead a fixture.",
    )
    .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_str().unwrap().into(),
        }],
        workspace.path(),
        "cursor-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    let mut config = AcpAgentConfig {
        command: "cursor-agent".into(),
        args: vec!["acp".into()],
        env: [("CURSOR_CONFIG_DIR".into(), "/private-config".into())].into(),
        ..Default::default()
    };
    super::configure(&mut config, Some(&view)).unwrap();
    assert_eq!(config.command, "cursor-agent");
    assert_eq!(config.args, ["acp"]);
    assert_eq!(config.env["CURSOR_CONFIG_DIR"], "/private-config");
    assert_eq!(config.env[super::ROOT_ENV], view.0.root().to_str().unwrap());
    assert_eq!(config.env.len(), 2);

    std::fs::remove_file(source.join("SKILL.md")).unwrap();
    config.env.remove(super::ROOT_ENV);
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}
