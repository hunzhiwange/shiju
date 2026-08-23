//! 会话类型模块的编译锚点测试。
//!
//! 该轻量用例确保测试子模块持续接入构建，具体会话 API 行为由客户端测试覆盖。

#[test]
fn module_test_anchor() {
    let stable_value = 1 + 1;
    assert_eq!(stable_value, 2);
}
