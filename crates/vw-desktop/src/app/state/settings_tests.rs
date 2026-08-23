//! 设置状态聚合测试模块的连线哨兵。
//! 确保各设置子域拆分后仍通过统一测试入口参与构建。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("settings_tests"));
}
