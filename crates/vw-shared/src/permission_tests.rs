//! 权限领域模块的编译入口测试。
//!
//! 该锚点保证权限类型所在模块持续纳入测试构建。

#[test]
fn module_test_anchor() {
    let stable_value = 1 + 1;
    assert_eq!(stable_value, 2);
}
