#[test]
fn native_task_commands_use_global_agent_scheduler_store() {
    let source = include_str!("lib.rs");
    assert!(source.contains("create_native_task("));
    assert!(source.contains("load_native_task("));
    assert!(source.contains("list_native_tasks("));
    for forbidden in ["task::create_task(", "task::load_task(", "task::load_all_tasks("] {
        assert!(!source.contains(forbidden), "legacy native task call remains: {forbidden}");
    }
}
