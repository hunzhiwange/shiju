//! 目标模式纯函数测试。

use super::goal::inspect_goal_task_file;
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
    assert_eq!(
        inspect_goal_task_file(temp.path(), &task("1-goal.md")).expect("single task complete"),
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
fn goal_task_files_accept_expected_failure_only_with_successful_completion_check() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("1-goal.md"),
        "# task\n\n状态：已完成\n\n完成条件：预期失败已由外层检查确认\n\n验证命令：sh -c 'run-test; test $? -eq 1'\n\n验证结果：内层测试 EXIT=1（预期）；外层检查退出码：0\n",
    )
    .expect("write expected failure evidence");

    assert_eq!(
        inspect_goal_task_files(temp.path(), &[task("1-goal.md")])
            .expect("successful completion check"),
        GoalTaskFilesState::Complete
    );
}

#[test]
fn execution_prompt_separates_expected_failure_from_completion_evidence() {
    let plan_abs = std::path::Path::new("/tmp/runtime-home/plan/task-contract");
    let prompt = super::prompts::build_execution_prompt(
        "验证预期失败证据",
        1,
        &[],
        plan_abs,
        &plan_abs.join("1-contract.md"),
        "# plan.md",
        "# 子任务.md",
    );

    assert!(prompt.contains("预期失败命令必须由退出 0 的外层检查证明"));
    assert!(prompt.contains("顶层 `验证命令：` 只能记录完成后实际运行且退出 0 的检查"));
    assert!(prompt.contains("内层预期 `EXIT=1` 不得写成完成证据"));
}

#[test]
fn goal_mode_session_ids_are_stable() {
    assert_eq!(goal_turn_session_id("task.1", 2), "task-1-goal-2");
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
