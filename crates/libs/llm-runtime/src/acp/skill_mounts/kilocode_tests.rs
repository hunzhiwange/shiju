#[test]
fn acp_skill_mount_kilocode_selected_mounts_use_reviewed_native_contract() {
    assert!(super::super::validate("kilocode", true).is_ok());
}

#[test]
fn acp_skill_mount_kilocode_empty_selection_preserves_existing_cli_use() {
    assert!(super::super::validate("kilocode", false).is_ok());
}

#[test]
fn acp_skill_coexistence_kilocode_empty_selection_and_conflicting_override_fail_closed() {
    let mut config = agent_config::schema::AcpAgentConfig::default();
    super::configure(&mut config, None).unwrap();
    assert_eq!(config.env[super::ROOT_ENV], "");
    assert!(super::configure(&mut config, None).is_err());
}

#[test]
fn acp_skill_coexistence_kilocode_unreviewed_executable_is_rejected_before_environment_changes() {
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
        "kilocode-negative",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let mut config = agent_config::schema::AcpAgentConfig {
        command: root.path().join("missing-kilocode").to_string_lossy().into(),
        ..Default::default()
    };
    assert!(super::configure(&mut config, Some(&view)).is_err());
    assert!(!config.env.contains_key(super::ROOT_ENV));
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_kilocode_limits_sandbox_reads_to_selected_sources() {
    let fixture = tempfile::tempdir().unwrap();
    let sources = fixture.path().join("sources");
    let selected = sources.join("selected");
    let unselected = sources.join("unselected");
    let view = fixture.path().join("view");
    for path in [&selected, &unselected, &view] {
        std::fs::create_dir_all(path).unwrap();
    }
    std::os::unix::fs::symlink(&selected, view.join("selected")).unwrap();
    let (root, reads) = super::selection(&view).unwrap();
    assert_eq!(root, view.canonicalize().unwrap());
    assert_eq!(reads, [selected.canonicalize().unwrap()]);
    std::fs::create_dir(view.join("untrusted")).unwrap();
    assert!(super::selection(&view).is_err());
}

#[test]
fn acp_skill_coexistence_kilocode_requires_host_mcp_tool_policy() {
    let parse = |definition| serde_json::from_value::<acp_types::AcpMcpServer>(definition).unwrap();
    use serde_json::json;
    let mut definition = json!({
        "name": "coexistence",
        "command": "/usr/bin/python3",
        "args": [],
        "env": [],
    });
    assert!(super::validate_mcp(&[parse(definition.clone())]).is_err());
    definition["_meta"] = json!({"shiju.omp.toolPolicy": {"allow": ["read_probe"], "deny": []}});
    assert!(super::validate_mcp(&[parse(definition)]).is_ok());
}
