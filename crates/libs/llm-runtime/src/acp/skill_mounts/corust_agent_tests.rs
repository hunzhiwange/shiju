use super::super::MountedSkills;
use std::fs;

#[test]
fn acp_skill_coexistence_corust_agent_without_selection_preserves_private_config() {
    let profile = tempfile::tempdir().unwrap();
    let config = profile.path().join("corust-launch.json");
    fs::write(&config, "binding").unwrap();
    super::publish(profile.path(), profile.path(), None::<&MountedSkills>).unwrap();
    assert_eq!(fs::read_to_string(config).unwrap(), "binding");
    assert!(!profile.path().join(".agents").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_corust_agent_projects_only_workspace_selection() {
    let workspace = tempfile::tempdir().unwrap();
    let other_project = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    let source = workspace.path().join("selected");
    fs::create_dir(&source).unwrap();
    fs::create_dir(workspace.path().join("unselected")).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: selected\ndescription: fixture\n---\nRead the fixture.",
    )
    .unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: source.to_str().unwrap().into(),
        }],
        workspace.path(),
        "corust-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("corust-agent", true).is_ok());
    assert!(super::publish(profile.path(), other_project.path(), Some(&view)).is_err());
    assert!(!profile.path().join(".agents").exists());
    super::publish(profile.path(), workspace.path(), Some(&view)).unwrap();
    let scanner = profile.path().join(".agents/skills");
    assert_eq!(fs::read_link(scanner.join("selected")).unwrap(), source.canonicalize().unwrap(),);
    assert!(!scanner.join("unselected").exists());
}
