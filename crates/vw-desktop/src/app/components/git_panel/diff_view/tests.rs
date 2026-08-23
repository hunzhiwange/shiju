//! 验证差异视图测试模块已正确接入父模块。
//! 此处只保留接线哨兵，具体渲染行为由相邻测试子模块覆盖。

#[test]
fn task_713_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("tests.rs"));
}
