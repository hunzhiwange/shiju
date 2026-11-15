use super::*;
fn slot<'a>(_id: &str, run: &'a str) -> ActivitySlot<'a> {
    ActivitySlot { agent_id: Some("worker"), run_id: Some(run),
        leased_task_id: Some(match run { "a" => "task-a", "b" => "task-b", "history" => "task-history", _ => "task-run" }), busy: true }
}
fn entry() -> AgentEntry<'static> {
    AgentEntry { key: "worker", label: "Worker", enabled: true }
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
        let slots = (0..capacity).map(|_i| slot("slot", "run")).collect::<Vec<_>>();
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
