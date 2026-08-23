//! 验证应用视图浮层测试模块已接入测试构建。
//!
//! 该哨兵只验证浮层聚合测试模块仍在编译图中，不虚构额外界面契约。

#[test]
fn app_view_modals_tests_module_is_wired() {
    assert!(module_path!().ends_with("app_view_modals_tests"));
}
