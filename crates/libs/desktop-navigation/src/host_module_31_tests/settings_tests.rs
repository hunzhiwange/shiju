#[test]
fn settings_tests_module_is_wired() {
    assert!(module_path!().ends_with("settings_tests"));
}
