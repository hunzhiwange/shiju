//! 目标模式纯函数测试。

use super::{
    GoalTaskFilesState, PlanTaskEntry, build_goal_continuation_prompt, goal_turn_session_id,
    inspect_goal_task_files,
};

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
fn goal_task_files_require_completed_status_and_validation_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("1-goal.md");
    std::fs::write(&path, "# task\n\n状态：已完成\n\n完成条件：功能已实现\n")
        .expect("write incomplete completed task");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("missing evidence"),
        GoalTaskFilesState::Pending(reason) if reason.contains("验证命令")
    ));

    std::fs::write(
        &path,
        "# task\n\n状态：已完成\n\n完成条件：功能已实现\n\n验证命令：cargo test -p vw-agent --lib\n\n验证结果：退出码：1\n",
    )
    .expect("write failed validation");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("failed evidence"),
        GoalTaskFilesState::Pending(reason) if reason.contains("退出码")
    ));

    std::fs::write(
        &path,
        "# task\n\n状态：已完成\n\n完成条件：功能已实现\n\n验证命令：cargo test -p vw-agent --lib\n\n验证结果：退出码：0；测试通过\n",
    )
    .expect("write completed task");
    assert_eq!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("complete"),
        GoalTaskFilesState::Complete
    );

    std::fs::write(&path, "# task\n\n状态：已阻塞\n\n阻塞原因：缺少凭据\n").expect("write blocked");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("blocked"),
        GoalTaskFilesState::Blocked(reason) if reason.contains("缺少凭据")
    ));

    std::fs::remove_file(path).expect("remove task");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("missing task"),
        GoalTaskFilesState::Pending(reason) if reason.contains("不存在")
    ));
}

#[test]
fn goal_mode_session_ids_are_stable() {
    assert_eq!(goal_turn_session_id("task.1", 2), "task-1-goal-2");
}

#[test]
fn goal_continuation_keeps_outer_plan_and_reuses_recursive_policy() {
    let prompt = build_goal_continuation_prompt(
        "完成递归智能体目标",
        "# plan.md",
        "后代证据尚未综合",
        &GoalTaskFilesState::Pending("1-实现.md".to_string()),
    );

    assert!(prompt.contains("外层编号任务、批次依赖和任务文件所有权保持不变"));
    assert!(prompt.contains("自主拆分内部工作流"));
    assert!(prompt.contains("Launch independent read-only or non-overlapping work"));
    assert!(prompt.contains("verify evidence and synthesize required results"));
}
