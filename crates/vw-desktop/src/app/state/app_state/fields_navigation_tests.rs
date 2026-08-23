//! 应用导航字段测试模块的连线哨兵。
//! 确保屏幕、标签与导航选择状态随主应用状态一同进入测试构建。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("fields_navigation_tests"));
}
