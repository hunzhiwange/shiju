//! 工作流消息测试模块的入口检查，保证拆分处理器用例被父模块编译。
//! 具体状态转换分别由应用、画布、文档、节点和运行测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}
