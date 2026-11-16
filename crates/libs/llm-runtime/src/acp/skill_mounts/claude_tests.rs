#[test]
fn acp_skill_mount_claude_accepts_native_plugin_contract() {
    assert!(super::super::validate("claude", true).is_ok());
    assert!(super::super::validate("claude", false).is_ok());
}

#[test]
fn acp_skill_mount_claude_creates_manifest_over_selected_view_only() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: example\ndescription: fixture\n---\nPRIVATE_BODY",
    )
    .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into(),
        }],
        root.path(),
        "session",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    super::prepare(&view.0).unwrap();
    let bytes =
        std::fs::read(view.0.root().parent().unwrap().join(".claude-plugin/plugin.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["skills"], "./skills/");
    assert!(!String::from_utf8(bytes).unwrap().contains("PRIVATE_BODY"));
    view.0.validate().unwrap();
}
