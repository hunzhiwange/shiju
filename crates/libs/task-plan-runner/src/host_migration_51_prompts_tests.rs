//! 规划提示词约束测试。

#[test]
fn build_planning_prompt_requires_fixed_plan_dir_and_tool_writes() {
    crate::test_host::install();
    let plan_dir = std::path::Path::new("/tmp/user/.shiju-harness-dev/storage/plan/task-abc-123");
    let prompt =
        super::prompts::build_planning_prompt_for_dir("abc-123", "实现工作流执行", plan_dir);
    assert!(prompt.contains("`/tmp/user/.shiju-harness-dev/storage/plan/task-abc-123/`"));
    assert!(!prompt.contains("仓库相对路径"));
    assert!(!prompt.contains(".plans/task-abc-123"));
    assert!(prompt.contains("必须通过写文件工具实际创建文件"));
    assert!(prompt.contains("忽略技能模板里的 `YYYY-MM-DD-<功能名称>` 路径"));
    assert!(prompt.contains("tasks.json"));
    assert!(prompt.contains("create-task-plans-runtime"));
}


#[test]
fn plan_dir_abs_uses_user_storage_and_sanitizes_task_id() {
    crate::test_host::install();
    let home = std::path::Path::new("/tmp/user");

    assert_eq!(
        super::plan_dir_abs(home, "unsafe/id.part"),
        config_runtime_paths::task_plan_storage_dir(home).join("task-unsafeid-part")
    );
}


#[test]
fn goal_mode_planning_prompt_requires_final_validation_task() {
    crate::test_host::install();
    let prompt = super::prompts::build_goal_planning_prompt_for_dir(
        "goal-1",
        "实现目标模式",
        std::path::Path::new("/tmp/user/.shiju-harness-dev/storage/plan/task-goal-1"),
    );
    assert!(prompt.contains("持续目标模式"));
    assert!(prompt.contains("独立的目标验收任务"));
    assert!(prompt.contains("命令、退出码和结果"));
    assert!(prompt.contains("状态：已阻塞"));
}


#[test]
fn execution_prompt_names_canonical_plan_and_task_paths() {
    crate::test_host::install();
    let plan_abs = std::path::Path::new("/tmp/runtime-home/plan/task-dynamic");
    let task_file_abs = plan_abs.join("1-dynamic.md");
    let prompt = super::build_execution_prompt(
        "执行动态任务",
        1,
        &[],
        plan_abs,
        &task_file_abs,
        "# plan.md\n整体架构",
        "# 子任务.md\n当前实现边界",
    );

    assert!(prompt.contains(&format!("`{}`", plan_abs.display())));
    assert!(prompt.contains(&format!("`{}`", task_file_abs.display())));
    assert!(prompt.contains("唯一可写进度源"));
    assert!(prompt.contains("不搜索、不新建、不更新仓库内同名 `.plans` 文件"));
}
