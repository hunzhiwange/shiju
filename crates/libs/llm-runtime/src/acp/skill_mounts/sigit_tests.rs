use super::super::MountedSkills;

#[test]
fn acp_skill_coexistence_sigit_empty_selection_registers_no_sources() {
    assert_eq!(super::selection(None::<&MountedSkills>).unwrap(), "[]");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_sigit_registers_only_the_validated_view() {
    use std::fs;

    let workspace = tempfile::tempdir().unwrap();
    let selected = workspace.path().join("selected");
    fs::create_dir(&selected).unwrap();
    fs::create_dir(workspace.path().join("unselected")).unwrap();
    let body = selected.join("SKILL.md");
    fs::write(&body, "---\nname: selected\ndescription: fixture\n---\nSelected.").unwrap();
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: selected.to_str().unwrap().into(),
        }],
        workspace.path(),
        "sigit-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("sigit", true).is_ok());
    let config: serde_json::Value =
        serde_json::from_str(&super::selection(Some(&view)).unwrap()).unwrap();
    assert_eq!(
        config,
        serde_json::json!([[
            view.0.root().canonicalize().unwrap().join("selected"),
            selected.canonicalize().unwrap(),
        ]])
    );

    fs::write(body, "changed after selection").unwrap();
    assert!(super::selection(Some(&view)).is_err());
}
