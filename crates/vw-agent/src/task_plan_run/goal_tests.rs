//! 真实宿主递归策略接线契约。
use super::*;
fn task(file: &str) -> PlanTaskEntry {
    PlanTaskEntry {
        id: "1".to_string(),
        title: "验收".to_string(),
        file: file.to_string(),
        batch: 1,
        depends_on_batches: Vec::new(),
        status: "未开始".to_string(),
    }
}

#[test]
fn goal_continuation_keeps_outer_plan_and_reuses_recursive_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    let task = task("1-实现.md");
    let prompt = build_goal_continuation_prompt(
        "完成递归智能体目标",
        temp.path(),
        std::slice::from_ref(&task),
        "# plan.md",
        "后代证据尚未综合",
        &GoalTaskFilesState::Pending("1-实现.md".to_string()),
        "子任务 [1] 已完成实现，测试退出码 0",
    );

    assert!(prompt.contains("外层编号任务、批次依赖和任务文件所有权保持不变"));
    assert!(prompt.contains("自主拆分内部工作流"));
    assert!(prompt.contains("Launch independent read-only or non-overlapping work"));
    assert!(prompt.contains("verify evidence and synthesize required results"));
    assert!(prompt.contains(&temp.path().to_string_lossy().to_string()));
    assert!(prompt.contains(&temp.path().join(&task.file).to_string_lossy().to_string()));
    assert!(prompt.contains("子任务 [1] 已完成实现，测试退出码 0"));
    assert!(prompt.contains("不要重复实现已经达成的产物"));
}
