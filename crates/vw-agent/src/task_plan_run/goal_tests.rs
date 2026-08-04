//! 目标模式纯函数测试。

use super::{
    GoalTaskFilesState, PlanTaskEntry, build_goal_continuation_prompt,
    build_goal_evaluation_prompt, goal_evaluation_session_id, goal_turn_session_id,
    inspect_goal_task_files, parse_goal_evaluation,
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
fn goal_mode_evaluation_requires_strict_json() {
    assert_eq!(
        parse_goal_evaluation(r#"{"achieved":true,"reason":"验证通过"}"#)
            .expect("valid evaluation")
            .achieved,
        true
    );
    assert!(parse_goal_evaluation("```json\n{\"achieved\":true,\"reason\":\"ok\"}\n```").is_err());
    assert!(parse_goal_evaluation(r#"{"achieved":true,"reason":"","extra":1}"#).is_err());
}

#[test]
fn goal_mode_task_files_distinguish_pending_complete_and_blocked() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("1-goal.md");
    std::fs::write(&path, "# task\n\n状态：未开始\n").expect("write pending");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("pending"),
        GoalTaskFilesState::Pending(_)
    ));

    std::fs::write(&path, "# task\n\n状态：已阻塞\n\n阻塞原因：缺少凭据\n")
        .expect("write blocked");
    assert!(matches!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("blocked"),
        GoalTaskFilesState::Blocked(reason) if reason.contains("缺少凭据")
    ));

    std::fs::remove_file(path).expect("remove task");
    assert_eq!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")]).expect("complete"),
        GoalTaskFilesState::Complete
    );
}

#[test]
fn goal_mode_session_ids_are_stable() {
    assert_eq!(goal_turn_session_id("task.1", 2), "task-1-goal-2");
    assert_eq!(
        goal_evaluation_session_id("task.1", 2),
        "task-1-goal-eval-2"
    );
}

#[test]
fn goal_evaluation_rejects_unsettled_or_unsynthesized_descendants() {
    let prompt = build_goal_evaluation_prompt(
        "完成递归智能体目标",
        "# plan.md",
        "[AgentTool 后代终态]\n- session=child state=failed error=验证失败",
        &GoalTaskFilesState::Complete,
    );

    assert!(prompt.contains("active、waiting、queued 或 running"));
    assert!(prompt.contains("failed 或 cancelled"));
    assert!(prompt.contains("父会话验证并综合"));
    assert!(prompt.contains("achieved 必须为 false"));
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
