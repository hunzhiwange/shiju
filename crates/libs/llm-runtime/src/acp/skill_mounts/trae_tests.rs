use super::super::MountedSkills;
use std::fs;

#[test]
fn acp_skill_coexistence_trae_without_selection_preserves_private_config() {
    let profile = tempfile::tempdir().unwrap();
    let config = profile.path().join("binding.json");
    fs::write(&config, "binding").unwrap();
    super::publish(profile.path(), None::<&MountedSkills>).unwrap();
    assert_eq!(fs::read_to_string(config).unwrap(), "binding");
    assert!(!profile.path().join("skills").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_trae_projects_only_the_validated_view() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    let source = workspace.path().join("selected");
    fs::create_dir_all(source.join("references")).unwrap();
    fs::create_dir(workspace.path().join("unselected")).unwrap();
    let body = "---\nname: selected\ndescription: fixture\n---\nRead the fixture.";
    fs::write(source.join("SKILL.md"), body).unwrap();
    fs::write(source.join("references/value.txt"), "attachment").unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: source.to_str().unwrap().into(),
        }],
        workspace.path(),
        "trae-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("trae", true).is_ok());
    super::publish(profile.path(), Some(&view)).unwrap();
    let destination = profile.path().join("skills");
    let selected = destination.join("selected");
    assert_eq!(fs::read_link(&destination).unwrap(), view.0.root());
    assert_eq!(fs::read_to_string(selected.join("SKILL.md")).unwrap(), body);
    assert_eq!(fs::read_to_string(selected.join("references/value.txt")).unwrap(), "attachment",);
    assert!(!destination.join("unselected").exists());
    assert!(super::publish(profile.path(), Some(&view)).is_err());

    let next = tempfile::tempdir().unwrap();
    fs::remove_file(source.join("SKILL.md")).unwrap();
    assert!(super::publish(next.path(), Some(&view)).is_err());
    assert!(!next.path().join("skills").exists());
}
