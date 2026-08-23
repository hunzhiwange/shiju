//! 模型提供商测试模块的连线哨兵。
//! 确保提供商相关测试随应用模块一同编译，防止测试注册在重构中脱落。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("provider_tests"));
}
