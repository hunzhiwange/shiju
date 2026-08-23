//! 应用状态聚合测试模块的连线哨兵。
//! 确保拆分后的字段模块仍由顶层应用状态测试入口统一编译。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("app_state_tests"));
}
