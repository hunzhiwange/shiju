//! 任务领域模块的编译入口测试。
//!
//! 该锚点确保任务模块持续接入 crate 的测试构建。

#[test]
fn module_test_anchor() {
    let stable_value = 1 + 1;
    assert_eq!(stable_value, 2);
}
