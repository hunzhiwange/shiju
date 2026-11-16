// Task 20 ownership: Gateway DTO -> executor snapshot -> settings/slot adapters -> domain projection: retain host conversion contract; local identity assertions also live in desktop-task-model::desktop_split_20_tests.
use super::agent_activity::*;
use desktop_settings_agents::state::DelegateAgentSettingsEntry;
use desktop_task_executor::executor::state::WorktreeSlotSnapshot;
use desktop_task_executor::executor::state::WorktreeState;
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
fn task_pool_employee_gateway_projection_preserves_run_identity() {
    use desktop_task_executor::executor::state::WorktreePoolSnapshot;
    use vw_api_task::{
        TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto, TaskPoolWorktreeSnapshotResponse,
    };
    use {vw_api_transport::id::ProjectId, vw_api_transport::id::TaskId};
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
