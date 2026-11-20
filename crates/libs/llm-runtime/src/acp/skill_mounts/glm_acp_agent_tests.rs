use super::super::MountedSkills;

#[test]
fn acp_skill_coexistence_glm_acp_agent_empty_view() {
    assert_eq!(super::selection(std::path::Path::new("."), None::<&MountedSkills>).unwrap(), "[]");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_glm_acp_agent_validated_workspace_view() {
    use std::fs;

    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let selected = workspace.join("selected");
    let outside = root.path().join("outside");
    fs::create_dir_all(&selected).unwrap();
    fs::create_dir(workspace.join("unselected")).unwrap();
    fs::create_dir(&outside).unwrap();
    for dir in [&selected, &outside] {
        fs::write(dir.join("SKILL.md"), "---\nname: selected\ndescription: fixture\n---\nBody.")
            .unwrap();
    }
    let mount = |path: &std::path::Path| {
        MountedSkills::prepare(
            &[acp_types::SessionSkillMount {
                name: "selected".into(),
                path: path.to_str().unwrap().into(),
            }],
            &workspace,
            "glm-coexistence",
            &Default::default(),
            &Default::default(),
        )
        .unwrap()
        .unwrap()
    };

    let view = mount(&selected);
    assert!(super::super::validate("glm-acp-agent", true).is_ok());
    let config: serde_json::Value =
        serde_json::from_str(&super::selection(&workspace, Some(&view)).unwrap()).unwrap();
    assert_eq!(
        config,
        serde_json::json!([[
            "selected",
            view.0.root().canonicalize().unwrap().join("selected"),
            selected.canonicalize().unwrap(),
        ]])
    );
    assert!(super::selection(&workspace, Some(&mount(&outside))).is_err());
    fs::write(selected.join("SKILL.md"), "changed").unwrap();
    assert!(super::selection(&workspace, Some(&view)).is_err());
}
