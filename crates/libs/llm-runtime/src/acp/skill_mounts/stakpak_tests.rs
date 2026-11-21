use super::super::MountedSkills;

#[test]
fn acp_skill_coexistence_stakpak_leaves_empty_selection_alone() {
    let home = tempfile::tempdir().unwrap();
    super::publish_at(home.path(), None::<&MountedSkills>).unwrap();
    assert!(!home.path().join(".stakpak").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_stakpak_publishes_only_the_selected_snapshot() {
    use std::fs;

    let workspace = tempfile::tempdir().unwrap();
    let selected = workspace.path().join("selected");
    fs::create_dir(&selected).unwrap();
    fs::create_dir(workspace.path().join("unselected")).unwrap();
    let body = "---\nname: selected\ndescription: fixture\n---\nSelected.";
    fs::write(selected.join("SKILL.md"), body).unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_str().unwrap().into(),
        }],
        workspace.path(),
        "stakpak-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    let home = tempfile::tempdir().unwrap();
    fs::create_dir(home.path().join(".stakpak")).unwrap();
    assert!(super::super::validate("stakpak", true).is_ok());
    super::publish_at(home.path(), Some(&view)).unwrap();
    let snapshot = home.path().join(".stakpak/skills");
    assert_eq!(fs::read_to_string(snapshot.join("selected/SKILL.md")).unwrap(), body);
    assert_eq!(fs::read_dir(snapshot).unwrap().count(), 1);
    assert!(!workspace.path().join(".stakpak").exists());
}
