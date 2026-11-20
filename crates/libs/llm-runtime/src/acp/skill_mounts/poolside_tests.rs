#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_poolside_projects_only_the_validated_view() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("example");
    let other = root.path().join("other");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::fs::write(source.join("SKILL.md"), "---\nname: example\ndescription: fixture\n---\nBODY")
        .unwrap();
    let view = super::super::MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "example".into(),
            path: source.to_string_lossy().into_owned(),
        }],
        root.path(),
        "poolside-acceptance",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    assert!(super::super::validate("poolside", true).is_ok());
    let selected = super::selection(true, Some(&view)).unwrap().unwrap();
    let path = std::path::Path::new(&selected);
    assert_eq!(path, view.0.root().canonicalize().unwrap());
    assert!(path.join("example").is_symlink());
    assert!(!path.join("other").exists());
    let empty: Option<&super::super::MountedSkills> = None;
    assert_eq!(super::selection(true, empty).unwrap(), Some(String::new()));
    assert_eq!(super::selection(false, empty).unwrap(), None);
    assert!(super::selection(false, Some(&view)).is_err());
    std::fs::write(source.join("SKILL.md"), "changed").unwrap();
    assert!(super::selection(true, Some(&view)).is_err());
}
