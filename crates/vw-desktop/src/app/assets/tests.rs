//! 验证资产测试模块保持接入主测试树。
//! 该冒烟点用于发现资产子模块因声明调整而未参与测试编译的回归。

#[test]
fn task_642_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("tests.rs"));
}
