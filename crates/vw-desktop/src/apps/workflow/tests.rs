//! 工作流应用测试入口的连线检查，确认顶层模块在测试配置下可访问。
//! 具体模型、状态、消息、画布和视图行为由相邻拆分测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}
