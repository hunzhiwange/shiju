#[test]
fn acp_skill_coexistence_qoder_no_selection_preserves_config() {
    let profile = tempfile::tempdir().unwrap();
    let destination = profile.path().join("skills");
    super::publish(profile.path(), None::<&super::super::MountedSkills>).unwrap();
    assert!(!destination.exists());
    std::fs::create_dir(&destination).unwrap();
    super::publish(profile.path(), None::<&super::super::MountedSkills>).unwrap();
    assert!(destination.is_dir());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_qoder_publishes_only_selected_view() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    let selected = workspace.path().join("example");
    let unselected = workspace.path().join("unselected");
    for source in [&selected, &unselected] {
        std::fs::create_dir(source).unwrap();
        std::fs::write(
            source.join("SKILL.md"),
            "---\nname: example\ndescription: fixture\n---\nRead a fixture.",
        )
        .unwrap();
    }
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: selected.to_str().unwrap().into(),
        }],
        workspace.path(),
        "qoder-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("qoder", true).is_ok());
    let destination = profile.path().join("skills");
    super::publish(profile.path(), Some(&view)).unwrap();
    assert_eq!(std::fs::read_link(&destination).unwrap(), view.0.root());
    assert!(destination.join("example/SKILL.md").is_file());
    assert!(!destination.join("unselected").exists());
    assert!(super::publish(profile.path(), Some(&view)).is_err());

    let other_profile = tempfile::tempdir().unwrap();
    std::fs::remove_file(selected.join("SKILL.md")).unwrap();
    assert!(super::publish(other_profile.path(), Some(&view)).is_err());
    assert!(!other_profile.path().join("skills").exists());
}
