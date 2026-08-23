//! 验证 Git 面板 UI 测试子模块已正确接入。
//! 具体按钮和共享样式契约由同目录下的细分测试模块承担。

#[test]
fn task_722_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("ui_tests.rs"));
}
