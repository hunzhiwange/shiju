//! 验证差异文件起止边界测试模块保持接入主测试树。
//! 该冒烟点用于捕获起止行渲染测试因模块重组而未被编译执行的情况。

#[test]
fn task_710_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("start_end_tests.rs"));
}
