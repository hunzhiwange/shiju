use super::super::MountedSkills;

#[test]
fn acp_skill_coexistence_junie_without_selection_has_no_override() {
    assert!(super::selection(None::<&MountedSkills>).unwrap().is_none());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_junie_registers_only_the_validated_view() {
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
        "junie-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("junie", true).is_ok());
    let bytes = super::selection(Some(&view)).unwrap().unwrap();
    let config: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(config["root"], serde_json::json!(view.0.root().canonicalize().unwrap()));
    assert_eq!(
        config["sources"],
        serde_json::json!({"selected": selected.canonicalize().unwrap()})
    );

    fs::write(body, "changed after selection").unwrap();
    assert!(super::selection(Some(&view)).is_err());
}
