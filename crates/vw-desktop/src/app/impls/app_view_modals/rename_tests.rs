//! 重命名行为已迁入 desktop-workspace；此处保留宿主测试入口契约。

#[test]
fn rename_tests_module_is_wired() {
    assert!(module_path!().ends_with("rename_tests"));
}
