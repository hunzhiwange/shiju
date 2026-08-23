//! 应用核心字段测试模块的连线哨兵。
//! 用于发现核心状态拆分后遗漏测试模块注册的问题。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("core_fields_tests"));
}
