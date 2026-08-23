// 测试调度策略不再读取旧版审核开关，并保持显式策略行为。

#[test]
fn agent_scheduler_policy_no_longer_reads_legacy_review_switch() -> anyhow::Result<()> {
    use crate::agent_scheduler::domain::TaskId;
    use crate::agent_scheduler::policy::{AgentSchedulerPolicyConfig, AgentSchedulerPolicyService};
    use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
    use crate::agent_scheduler::store::AgentSchedulerStore;
    use crate::agent_scheduler::store::tasks::NewAgentTask;
    use serde_json::json;
    use vw_api_types::common::TimestampMs;

    let source = include_str!("agent_scheduler/policy.rs");
    assert!(!source.contains("code_review_enabled"));

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-agent-scheduler-policy-review-cutover");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-policy-review-cutover")?;
    store.create_task(
        &project_path,
        NewAgentTask {
            id: task_id.clone(),
            title: "Policy task review cutover".to_string(),
            kind: AgentTaskKind::Finite,
            status: AgentTaskStatus::Submitted,
            spec_uri: "shared/tasks/task-policy-review-cutover/spec.md".to_string(),
            meta_uri: "shared/tasks/task-policy-review-cutover/meta.json".to_string(),
            assigned_to: None,
            team_id: None,
            room_id: None,
            project_room_id: None,
            delegated_to_team: None,
            schedule: None,
            timezone: None,
            next_scheduled_at: None,
            last_executed_at: None,
            dag: Vec::new(),
            metadata: json!({
                "task_board.status_key": "code_complete",
                "task_board.status_role": "review_ready",
                "merge_source_branch": "feature/review-cutover",
                "merge_target_branch": "main",
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/review-cutover"
            }),
            created_at_ms: TimestampMs(1_000),
        },
    )?;

    let outcome = AgentSchedulerPolicyService::new(&store).apply_project_policy(
        &project_path,
        TimestampMs(2_000),
        &AgentSchedulerPolicyConfig::default(),
    )?;

    assert!(outcome.review_task_ids.is_empty());
    assert!(outcome.skipped_review_task_ids.is_empty());
    assert!(outcome.merge_task_ids.is_empty());
    assert!(outcome.blocked_task_ids.is_empty());
    let task = store.load_task(&project_path, &task_id)?.expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");

    Ok(())
}
