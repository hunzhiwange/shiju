//! 验证语言服务器测试模块已接入测试构建。
//!
//! 该哨兵确认语言服务器聚合测试模块仍参与测试构建。

#[test]
fn tests_module_is_wired() {
    assert!(module_path!().ends_with("tests"));
}
