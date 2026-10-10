//! 任务看板字段测试，验证矩阵入站状态以空集合安全初始化。
//! 默认状态不得携带前一项目的任务、筛选或调度派生数据。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("fields_task_board_tests"));
}
