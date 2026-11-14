use super::*;
use crate::app::task::WorktreeState;
fn slot(id: &str, run: &str) -> WorktreeSlotSnapshot {
    WorktreeSlotSnapshot {
        agent_id: Some("worker".into()),
        run_id: Some(run.into()),
        id: id.into(),
        path: String::new(),
        branch: String::new(),
        state: WorktreeState::Busy,
        leased_task_id: Some(format!("task-{run}")),
        taint_reason: None,
    }
}
fn entry() -> DelegateAgentSettingsEntry {
    let mut e = DelegateAgentSettingsEntry::from_config("worker", None);
    e.enabled = true;
    e.label = "Worker".into();
    e
}
#[test]
fn task_pool_employee_uses_existing_agent_keys() {
    let rows = project_activity(&[slot("1", "a")], &[entry()], &[]);
    assert_eq!(rows[0].key.as_deref(), Some("worker"));
    assert_eq!(rows[0].label, "Worker");
    assert!(rows[0].available);
    assert_eq!(rows[0].current_task_count, 1);
}
#[test]
fn task_pool_employee_activity_ignores_worktree_count() {
    let rows = project_activity(&[slot("1", "a"), slot("2", "b")], &[entry()], &[]);
    assert_eq!(rows.len(), 2); // Both logical slots have no disk worktree.
    assert_eq!(rows[1].task_id.as_deref(), Some("task-b"));
}
#[test]
fn task_pool_employee_multiple_runs_remain_distinct() {
    let rows = project_activity(&[slot("1", "a"), slot("2", "b")], &[entry()], &[]);
    assert_eq!(rows[0].key, rows[1].key);
    assert_eq!(rows[0].run_id.as_deref(), Some("a"));
    assert_eq!(rows[1].run_id.as_deref(), Some("b"));
}
#[test]
fn task_pool_employee_capacity_is_preserved_independently_of_agent_definition_count() {
    for capacity in [0, 1, 7, 2] {
        let slots = (0..capacity).map(|i| slot(&i.to_string(), &i.to_string())).collect::<Vec<_>>();
        for entries in [vec![], vec![entry()]] {
            assert_eq!(project_activity(&slots, &entries, &[]).len(), capacity);
        }
    }
}
#[test]
fn task_pool_employee_disabled_or_deleted_agent_keeps_run_history() {
    let mut disabled = entry();
    disabled.enabled = false;
    for entries in [vec![disabled], vec![]] {
        let rows = project_activity(&[slot("1", "history")], &entries, &[]);
        assert!(!rows[0].available);
        assert_eq!(rows[0].key.as_deref(), Some("worker"));
        assert_eq!(rows[0].run_id.as_deref(), Some("history"));
    }
}

#[test]
fn task_pool_employee_gateway_projection_preserves_run_identity() {
    use crate::app::task::WorktreePoolSnapshot;
    use vw_gateway_client::vw_api_types::id::{ProjectId, TaskId};
    use vw_gateway_client::vw_api_types::task::{
        TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto, TaskPoolWorktreeSnapshotResponse,
    };
    let snapshot = WorktreePoolSnapshot::from_gateway_response(TaskPoolWorktreeSnapshotResponse {
        project_id: ProjectId("p".into()),
        repo_root: String::new(),
        pool_root: String::new(),
        base_branch: String::new(),
        idle_count: 0,
        busy_count: 1,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: vec![],
        slots: vec![TaskPoolWorktreeSlotDto {
            agent_id: Some("worker".into()),
            run_id: Some("real-run".into()),
            id: "logical-1".into(),
            path: String::new(),
            branch: String::new(),
            state: TaskPoolWorktreeSlotStateDto::Busy,
            leased_task_id: Some(TaskId("task-1".into())),
            taint_reason: None,
        }],
    });
    let rows = project_activity(&snapshot.slots, &[entry()], &[]);
    assert_eq!(rows[0].run_id.as_deref(), Some("real-run"));
    assert_eq!(rows[0].key.as_deref(), Some("worker"));
    assert_eq!(rows[0].task_id.as_deref(), Some("task-1"));
    assert_eq!(snapshot.busy_count, 1);
}
