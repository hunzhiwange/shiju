//! 设置预览字段测试模块的连线哨兵。
//! 确保设置草稿与预览状态的字段拆分不会绕过既有测试入口。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("settings_preview_fields_tests"));
}
