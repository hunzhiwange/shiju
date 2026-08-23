//! 应用窗口框架字段测试模块的连线哨兵。
//! 保证标题栏、侧栏及窗口装饰相关状态继续纳入应用状态测试编译面。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("chrome_fields_tests"));
}
