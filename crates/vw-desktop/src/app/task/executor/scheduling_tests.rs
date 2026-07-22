#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("scheduling_tests"));
}

fn task(id: &str, priority: u32, order: u32) -> super::models::Task {
    let mut task = super::models::Task::default();
    task.id = id.to_string();
    task.priority = priority;
    task.order = order;
    task
}

#[test]
fn simulate_task_execution_step_covers_all_status_transitions() {
    use super::TaskStatus;

    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Pool),
        Some(TaskStatus::Pending)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Pending),
        Some(TaskStatus::Planning)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Planning),
        Some(TaskStatus::Running)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Running),
        Some(TaskStatus::CodeComplete)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Failed),
        Some(TaskStatus::Pending)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::CodeComplete),
        Some(TaskStatus::CodeReview)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::CodeReview),
        Some(TaskStatus::PrSubmitted)
    );
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::PrSubmitted),
        Some(TaskStatus::Completed)
    );
    assert_eq!(super::simulate_task_execution_step("/repo", "task", TaskStatus::Paused), None);
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Completed),
        Some(TaskStatus::Archived)
    );
    assert_eq!(super::simulate_task_execution_step("/repo", "task", TaskStatus::Archived), None);
}

#[test]
fn simulate_task_execution_step_uses_state_machine() {
    use super::TaskStatus;
    use crate::app::task::{TaskStateMachine, TaskTransitionIntent};

    let machine = TaskStateMachine::default_lifecycle();

    // 模拟步进必须与状态机 Simulation 意图返回的默认下一状态一致，
    // 不再硬编码各状态的下一步。
    for status in TaskStatus::all() {
        assert_eq!(
            super::simulate_task_execution_step("/repo", "task", status),
            machine.next_for_intent(status, TaskTransitionIntent::Simulation),
            "simulate step must delegate to the state machine for {status:?}",
        );
    }

    // 终态由状态机决定：Completed 沿默认线性边推进到 Archived，
    // Archived/Paused 没有下一步。
    assert_eq!(
        super::simulate_task_execution_step("/repo", "task", TaskStatus::Completed),
        Some(TaskStatus::Archived)
    );
    assert_eq!(super::simulate_task_execution_step("/repo", "task", TaskStatus::Archived), None);
    assert_eq!(super::simulate_task_execution_step("/repo", "task", TaskStatus::Paused), None);
}

#[test]
fn count_helpers_treat_missing_statuses_as_zero() {
    let tasks_by_status = std::collections::HashMap::new();

    assert_eq!(super::count_running_tasks(&tasks_by_status), 0);
    assert_eq!(super::get_pool_and_pending_count(&tasks_by_status), 0);
    assert_eq!(super::get_total_task_count(&tasks_by_status), 0);
}

#[test]
fn count_helpers_sum_expected_status_buckets() {
    use super::TaskStatus;

    let mut tasks_by_status = std::collections::HashMap::new();
    tasks_by_status.insert(TaskStatus::Running, vec![task("r1", 1, 1), task("r2", 1, 2)]);
    tasks_by_status.insert(TaskStatus::Planning, vec![task("pl1", 1, 3)]);
    tasks_by_status.insert(TaskStatus::Pending, vec![task("p1", 1, 4), task("p2", 1, 5)]);
    tasks_by_status.insert(TaskStatus::Pool, vec![task("pool1", 1, 6)]);
    tasks_by_status.insert(TaskStatus::Completed, vec![task("done", 1, 7)]);

    assert_eq!(super::count_running_tasks(&tasks_by_status), 3);
    assert_eq!(super::get_pool_and_pending_count(&tasks_by_status), 4);
    assert_eq!(super::get_total_task_count(&tasks_by_status), 7);
}

#[test]
fn get_next_tasks_for_execution_returns_empty_for_project_without_tasks() {
    let temp = tempfile::TempDir::new().expect("temp project should be created");

    let next = super::get_next_tasks_for_execution(temp.path().to_string_lossy().as_ref(), 10, &[]);

    assert!(next.is_empty());
}

#[test]
fn select_next_tasks_orders_by_priority_then_order_and_truncates() {
    use super::TaskStatus;

    let mut tasks_by_status = std::collections::HashMap::new();
    tasks_by_status.insert(
        TaskStatus::Pending,
        vec![task("priority-two", 2, 0), task("later", 1, 3), task("earlier", 1, 1)],
    );

    let next = super::select_next_tasks(&tasks_by_status, 2, &[]);

    assert_eq!(next, vec![("earlier".to_string(), 1, 1), ("later".to_string(), 1, 3)]);
}

#[test]
fn select_next_tasks_excludes_ids_and_ignores_non_pending_tasks() {
    use super::TaskStatus;

    let mut tasks_by_status = std::collections::HashMap::new();
    tasks_by_status
        .insert(TaskStatus::Pending, vec![task("excluded", 1, 0), task("selected", 2, 0)]);
    tasks_by_status.insert(TaskStatus::Pool, vec![task("pool", 0, 0)]);
    tasks_by_status.insert(TaskStatus::Running, vec![task("running", 0, 0)]);

    let next = super::select_next_tasks(&tasks_by_status, 10, &["excluded".to_string()]);

    assert_eq!(next, vec![("selected".to_string(), 2, 0)]);
}

#[test]
fn select_next_tasks_returns_empty_when_limit_is_zero() {
    use super::TaskStatus;

    let tasks_by_status =
        std::collections::HashMap::from([(TaskStatus::Pending, vec![task("pending", 1, 0)])]);

    assert!(super::select_next_tasks(&tasks_by_status, 0, &[]).is_empty());
}

#[test]
fn executor_event_variants_keep_payloads() {
    use super::TaskStatus;

    let events = vec![
        super::ExecutorEvent::TaskStarted { task_id: "a".to_string() },
        super::ExecutorEvent::TaskProgress {
            task_id: "b".to_string(),
            message: "half".to_string(),
        },
        super::ExecutorEvent::TaskCompleted { task_id: "c".to_string() },
        super::ExecutorEvent::TaskFailed { task_id: "d".to_string(), error: "bad".to_string() },
        super::ExecutorEvent::StatusChanged {
            task_id: "e".to_string(),
            from: TaskStatus::Pending,
            to: TaskStatus::Running,
        },
    ];

    assert!(matches!(&events[0], super::ExecutorEvent::TaskStarted { task_id } if task_id == "a"));
    assert!(
        matches!(&events[1], super::ExecutorEvent::TaskProgress { task_id, message } if task_id == "b" && message == "half")
    );
    assert!(
        matches!(&events[2], super::ExecutorEvent::TaskCompleted { task_id } if task_id == "c")
    );
    assert!(
        matches!(&events[3], super::ExecutorEvent::TaskFailed { task_id, error } if task_id == "d" && error == "bad")
    );
    assert!(matches!(
        &events[4],
        super::ExecutorEvent::StatusChanged { task_id, from, to }
            if task_id == "e" && *from == TaskStatus::Pending && *to == TaskStatus::Running
    ));
}

#[test]
fn select_next_tasks_keeps_equal_priority_and_order_candidates() {
    use super::TaskStatus;

    let tasks_by_status = std::collections::HashMap::from([(
        TaskStatus::Pending,
        vec![task("first", 1, 1), task("second", 1, 1), task("later", 1, 2)],
    )]);

    let next = super::select_next_tasks(&tasks_by_status, 3, &[]);

    assert_eq!(
        next,
        vec![
            ("first".to_string(), 1, 1),
            ("second".to_string(), 1, 1),
            ("later".to_string(), 1, 2),
        ]
    );
}
