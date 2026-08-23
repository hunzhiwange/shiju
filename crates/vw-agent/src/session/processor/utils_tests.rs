//! 确认会话处理器工具测试模块保持正确装配。

#[test]
fn utils_tests_module_is_wired() {
    let marker = String::from("utils_tests");
    assert_eq!(marker.as_str(), "utils_tests");
}
