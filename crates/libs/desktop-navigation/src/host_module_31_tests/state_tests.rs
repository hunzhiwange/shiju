//! 启动状态组合的测试注册哨兵；具体领域状态由对应设置库拥有。

#[test]
fn state_tests_module_is_wired() {
    assert!(module_path!().ends_with("state_tests"));
}
