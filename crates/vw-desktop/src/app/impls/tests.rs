//! 验证应用实现层测试模块已接入测试构建。
//!
//! 该哨兵确认实现层聚合测试仍接入，防止模块重排时整组测试被遗漏。

#[test]
fn tests_module_is_wired() {
    assert!(module_path!().ends_with("tests"));
}
