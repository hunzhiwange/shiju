//! 工作区字段测试模块的连线哨兵。
//! 保证工作区布局和资源选择状态拆分后仍参与桌面端测试编译。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("fields_workspace_tests"));
}
