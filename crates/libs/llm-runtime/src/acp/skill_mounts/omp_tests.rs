use super::*;

#[test]
fn acp_skill_coexistence_omp_limits_custom_directories_to_the_selected_view() {
    assert!(super::super::validate("omp", false).is_ok());
    assert_eq!(
        super::super::validate("omp", true).is_ok(),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("skills");
    let source = directory.path().join("selected");
    let unselected = directory.path().join("unselected");
    let settings_path = directory.path().join("omp-selected-skills.yml");
    for path in [&root, &source, &unselected] {
        fs::create_dir(path).unwrap();
    }
    std::os::unix::fs::symlink(&source, root.join("selected")).unwrap();

    let empty: serde_json::Value = serde_json::from_slice(&settings(None).unwrap()).unwrap();
    assert_eq!(empty, json!({"skills": {"customDirectories": []}}));
    let selected: serde_json::Value =
        serde_json::from_slice(&settings(Some(&root)).unwrap()).unwrap();
    assert_eq!(selected, json!({"skills": {"customDirectories": [&root]}}));

    let original_policy = "(deny file-write*)\n(deny process-fork process-exec)\n(deny network*)\n";
    let policy = allow_selection(original_policy.into(), &settings_path, &root).unwrap();
    assert!(policy.starts_with(original_policy));
    assert_eq!(policy.matches("(allow file-read-data").count(), 3);
    assert!(policy.contains(&format!("(literal {})", quote(&settings_path).unwrap())));
    assert!(policy.contains(&format!("(subpath {})", quote(&root).unwrap())));
    assert!(policy.contains(&format!("(subpath {})", quote(&source).unwrap())));
    assert!(!policy.contains(&quote(&unselected).unwrap()));
}
