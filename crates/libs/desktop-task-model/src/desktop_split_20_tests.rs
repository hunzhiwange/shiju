//! Local activity identity preservation; the gateway DTO and settings adapters remain host contracts.
use crate::agent_activity::{ActivitySlot, AgentEntry, project_activity};

#[test]
fn task_pool_employee_gateway_projection_preserves_run_identity() {
    let slots = [ActivitySlot {
        agent_id: Some("worker"),
        run_id: Some("real-run"),
        leased_task_id: Some("task-1"),
        busy: true,
    }];
    let entries = [AgentEntry { key: "worker", label: "Worker", enabled: true }];
    let rows = project_activity(&slots, &entries, &[]);
    assert_eq!(rows[0].run_id.as_deref(), Some("real-run"));
    assert_eq!(rows[0].key.as_deref(), Some("worker"));
    assert_eq!(rows[0].task_id.as_deref(), Some("task-1"));
}
