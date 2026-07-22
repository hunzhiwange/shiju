#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[test]
fn project_dirs_points_at_current_application_namespace() {
    let dirs = super::project_dirs().expect("project dirs should resolve on desktop platforms");

    let config_dir = dirs.config_dir().to_string_lossy();
    assert!(config_dir.ends_with("vibe-window-dev") || config_dir.ends_with("vibe-window"));
}
