//! 真实宿主递归策略接线契约。
#[test]
fn execution_prompt_preserves_outer_task_and_allows_internal_orchestration() {
    let plan_abs = std::path::Path::new("/tmp/runtime-home/plan/task-orchestration");
    let task_file_abs = plan_abs.join("4-orchestration.md");
    let prompt = super::build_execution_prompt(
        "实现递归智能体编排",
        4,
        &[2, 3],
        plan_abs,
        &task_file_abs,
        "# plan.md\n整体架构",
        "# 子任务.md\n当前实现边界",
    );

    assert!(!prompt.contains("不要重新拆分任务"));
    assert!(prompt.contains("不得改写外层编号任务、批次依赖或任务文件所有权"));
    assert!(prompt.contains("可在当前任务边界内自主拆分内部工作流"));
    assert!(
        prompt.contains("Identify internal workstreams, ordering dependencies, and write sets")
    );
    assert!(prompt.contains("Retry only safely retryable failures, with a finite attempt limit"));
    assert!(prompt.contains("当前批次: 4"));
    assert!(prompt.contains("依赖批次: 2, 3"));
}
