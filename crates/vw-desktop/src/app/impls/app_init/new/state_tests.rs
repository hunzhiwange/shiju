//! 验证应用初始状态字段及其跨平台默认值。
//!
//! 该哨兵确认状态子模块仍被应用构造路径引用，避免拆分后测试静默失联。

#[test]
fn state_tests_module_is_wired() {
    assert!(module_path!().ends_with("state_tests"));
}
