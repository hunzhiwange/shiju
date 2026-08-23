//! 工作区任务字段测试模块的连线哨兵。
//! 守护任务选择、执行与工作区上下文状态的测试模块接线。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("workspace_task_fields_tests"));
}
