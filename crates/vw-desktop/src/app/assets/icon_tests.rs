//! 验证图标资产测试模块保持接入主测试树。
//! 该轻量冒烟测试用于捕获条件编译或模块声明变更造成的测试脱接。

#[test]
fn task_640_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_tests.rs"));
}
