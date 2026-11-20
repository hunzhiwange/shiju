#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_amp_acp_projects_only_the_validated_view() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("example");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: example\ndescription: fixture\n---\nBODY")
        .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into_owned(),
        }],
        root.path(),
        "amp-acceptance",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    assert!(super::super::validate("amp-acp", true).is_ok());
    let value: Vec<(std::path::PathBuf, std::path::PathBuf)> =
        serde_json::from_str(&super::selection(true, Some(&view)).unwrap()).unwrap();
    assert_eq!(value, [(view.0.root().join("example"), source.canonicalize().unwrap())]);
    let empty: Option<&super::super::MountedSkills> = None;
    assert_eq!(super::selection(true, empty).unwrap(), "[]");
    assert_eq!(super::selection(false, empty).unwrap(), "null");
    assert!(super::selection(false, Some(&view)).is_err());
    std::fs::write(source.join("SKILL.md"), "changed").unwrap();
    assert!(super::selection(true, Some(&view)).is_err());
}
