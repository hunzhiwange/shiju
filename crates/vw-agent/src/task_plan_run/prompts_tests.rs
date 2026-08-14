//! 规划提示词约束测试。

#[test]
fn build_planning_prompt_requires_fixed_plan_dir_and_tool_writes() {
    let prompt = super::build_planning_prompt("abc-123", "实现工作流执行");
    assert!(prompt.contains(".plans/task-abc-123/"));
    assert!(prompt.contains("必须通过写文件工具实际创建文件"));
    assert!(prompt.contains("忽略技能模板里的 `YYYY-MM-DD-<功能名称>` 路径"));
    assert!(prompt.contains("tasks.json"));
    assert!(prompt.contains("create-task-plans-runtime"));
}

#[test]
fn goal_mode_planning_prompt_requires_final_validation_task() {
    let prompt = super::build_goal_planning_prompt("goal-1", "实现目标模式");
    assert!(prompt.contains("持续目标模式"));
    assert!(prompt.contains("独立的目标验收任务"));
    assert!(prompt.contains("命令、退出码和结果"));
    assert!(prompt.contains("状态：已阻塞"));
}

#[test]
fn execution_prompt_preserves_outer_task_and_allows_internal_orchestration() {
    let prompt = super::build_execution_prompt(
        "实现递归智能体编排",
        4,
        &[2, 3],
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
