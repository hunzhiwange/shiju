//! 应用模型测试模块的连线哨兵。
//! 用于保证模型测试模块持续参与桌面端测试编译，避免模块注册被意外移除。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("models_tests"));
}
