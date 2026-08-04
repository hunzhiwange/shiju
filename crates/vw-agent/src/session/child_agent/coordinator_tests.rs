use super::coordinator::{ChildAgentCoordinator, ChildAgentTaskState, WaitCondition};
use std::time::Duration;

#[test]
fn coordinator_blocking_waits_for_recursive_descendants_until_quiescent() {
    let coordinator = ChildAgentCoordinator::new(8);
    coordinator
        .register("root", "child", "planner", "coordinate descendants")
        .expect("child should register");
    coordinator.mark_running("root", "child").expect("child should start");

    let waiting_coordinator = coordinator.clone();
    let waiter = std::thread::spawn(move || {
        waiting_coordinator
            .wait_for_descendants_blocking("root", &[], Duration::from_secs(1))
            .expect("blocking recursive wait should succeed")
    });

    coordinator.mark_succeeded("root", "child", "child complete").expect("child should complete");
    coordinator
        .register("child", "grandchild", "reviewer", "review the child result")
        .expect("grandchild should register during quiescence confirmation");
    coordinator.mark_running("child", "grandchild").expect("grandchild should start");
    coordinator
        .mark_succeeded("child", "grandchild", "review complete")
        .expect("grandchild should complete");

    let outcome = waiter.join().expect("blocking wait thread should join");
    assert!(!outcome.timed_out);
    assert_eq!(
        outcome.tasks.iter().map(|task| task.child_session_id.as_str()).collect::<Vec<_>>(),
        vec!["child", "grandchild"]
    );
    assert!(outcome.tasks.iter().all(|task| task.state.is_terminal()));
}

#[tokio::test]
async fn coordinator_waits_for_recursive_descendant_tree_until_quiescent() {
    let coordinator = ChildAgentCoordinator::new(8);
    coordinator
        .register("root", "child", "planner", "coordinate descendants")
        .expect("child should register");
    coordinator.mark_running("root", "child").expect("child should start");

    let waiting_coordinator = coordinator.clone();
    let waiter = tokio::spawn(async move {
        waiting_coordinator
            .wait_for_descendants("root", &[], Duration::from_secs(1))
            .await
            .expect("recursive wait should succeed")
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished(), "running child must keep tree wait pending");

    coordinator.mark_succeeded("root", "child", "child complete").expect("child should complete");
    tokio::task::yield_now().await;
    assert!(
        !waiter.is_finished(),
        "tree wait must confirm no descendant was registered after the child completed"
    );

    coordinator
        .register("child", "grandchild", "reviewer", "review the child result")
        .expect("grandchild should register while waiting");
    coordinator.mark_running("child", "grandchild").expect("grandchild should start");
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished(), "running grandchild must keep tree wait pending");

    coordinator
        .mark_succeeded("child", "grandchild", "review complete")
        .expect("grandchild should complete");
    let outcome = tokio::time::timeout(Duration::from_millis(200), waiter)
        .await
        .expect("terminal descendant notification should wake wait")
        .expect("wait task should join");

    assert!(!outcome.timed_out);
    assert_eq!(
        outcome.tasks.iter().map(|task| task.child_session_id.as_str()).collect::<Vec<_>>(),
        vec!["child", "grandchild"]
    );
    assert!(outcome.tasks.iter().all(|task| task.state.is_terminal()));
}

#[test]
fn coordinator_recursively_cancels_owned_descendants_only() {
    let coordinator = ChildAgentCoordinator::new(8);
    let child_cancellation = coordinator
        .register("root", "child", "planner", "coordinate descendants")
        .expect("child should register");
    let grandchild_cancellation = coordinator
        .register("child", "grandchild", "reviewer", "review")
        .expect("grandchild should register");
    let unrelated_cancellation = coordinator
        .register("other-root", "other-child", "coder", "unrelated work")
        .expect("unrelated child should register");
    coordinator.mark_running("root", "child").expect("child should start");
    coordinator.mark_running("child", "grandchild").expect("grandchild should start");
    coordinator.mark_running("other-root", "other-child").expect("unrelated child should start");

    let error = coordinator
        .cancel_descendants("root", &["other-child".to_string()])
        .expect_err("another tree must not be cancelled");
    assert!(error.to_string().contains("does not belong to parent"));
    assert!(!child_cancellation.is_cancelled());
    assert!(!grandchild_cancellation.is_cancelled());
    assert!(!unrelated_cancellation.is_cancelled());

    assert_eq!(coordinator.cancel_descendants("root", &[]).expect("tree cancel should succeed"), 2);
    assert!(child_cancellation.is_cancelled());
    assert!(grandchild_cancellation.is_cancelled());
    assert!(!unrelated_cancellation.is_cancelled());

    let descendants =
        coordinator.descendant_snapshots("root", &[]).expect("owner should inspect descendants");
    assert_eq!(
        descendants.iter().map(|task| task.child_session_id.as_str()).collect::<Vec<_>>(),
        vec!["child", "grandchild"]
    );
    assert!(descendants.iter().all(|task| task.state == ChildAgentTaskState::Cancelled));
    assert!(
        !coordinator
            .mark_succeeded("child", "grandchild", "late result")
            .expect("late completion should be ignored"),
        "recursive cancellation must preserve terminal immutability"
    );
}

#[test]
fn coordinator_rejects_descendant_cycles() {
    let coordinator = ChildAgentCoordinator::new(8);
    coordinator.register("root", "child", "planner", "coordinate").expect("child should register");
    coordinator.register("child", "root", "reviewer", "cycle").expect("cycle edge should register");

    let error = coordinator
        .descendant_snapshots("root", &[])
        .expect_err("recursive traversal must reject cycles");
    assert!(error.to_string().contains("descendant cycle detected"));
}

#[test]
fn coordinator_retains_terminal_ancestors_until_descendants_finish() {
    let coordinator = ChildAgentCoordinator::new(1);
    coordinator
        .register("root", "child-1", "planner", "coordinate")
        .expect("first child should register");
    coordinator
        .register("child-1", "grandchild", "reviewer", "review")
        .expect("grandchild should register");
    coordinator.mark_running("child-1", "grandchild").expect("grandchild should start");
    coordinator
        .mark_succeeded("root", "child-1", "child complete")
        .expect("first child should complete");
    coordinator
        .register("root", "child-2", "coder", "second task")
        .expect("second child should register");
    coordinator
        .mark_succeeded("root", "child-2", "second complete")
        .expect("second child should complete");

    let active_tree = coordinator
        .descendant_snapshots("root", &[])
        .expect("active descendant tree should remain");
    assert_eq!(
        active_tree.iter().map(|task| task.child_session_id.as_str()).collect::<Vec<_>>(),
        vec!["child-1", "grandchild", "child-2"]
    );

    coordinator
        .mark_succeeded("child-1", "grandchild", "review complete")
        .expect("grandchild should complete");
    let retained =
        coordinator.descendant_snapshots("root", &[]).expect("bounded tree should remain readable");
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].child_session_id, "child-2");
}

#[tokio::test]
async fn coordinator_wait_returns_on_terminal_transition_without_polling() {
    let coordinator = ChildAgentCoordinator::new(8);
    coordinator
        .register("parent-1", "child-1", "reviewer", "review the patch")
        .expect("task should register");
    coordinator.mark_running("parent-1", "child-1").expect("task should start");

    let waiting_coordinator = coordinator.clone();
    let waiter = tokio::spawn(async move {
        waiting_coordinator
            .wait("parent-1", &["child-1".to_string()], WaitCondition::All, Duration::from_secs(1))
            .await
            .expect("wait should succeed")
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished(), "non-terminal task must keep wait pending");

    coordinator
        .mark_succeeded("parent-1", "child-1", "review complete")
        .expect("task should complete");
    let outcome = tokio::time::timeout(Duration::from_millis(200), waiter)
        .await
        .expect("terminal notification should wake wait immediately")
        .expect("wait task should join");

    assert!(!outcome.timed_out);
    assert_eq!(outcome.tasks.len(), 1);
    assert_eq!(outcome.tasks[0].state, ChildAgentTaskState::Succeeded);
    assert_eq!(outcome.tasks[0].output.as_deref(), Some("review complete"));
}

#[tokio::test]
async fn coordinator_wait_any_and_all_have_distinct_completion_conditions() {
    let coordinator = ChildAgentCoordinator::new(8);
    for child in ["child-1", "child-2"] {
        coordinator.register("parent-1", child, "reviewer", child).expect("task should register");
        coordinator.mark_running("parent-1", child).expect("task should start");
    }
    coordinator.mark_succeeded("parent-1", "child-1", "first").expect("first task should complete");

    let any = coordinator
        .wait(
            "parent-1",
            &["child-1".to_string(), "child-2".to_string()],
            WaitCondition::Any,
            Duration::from_millis(10),
        )
        .await
        .expect("any wait should succeed");
    assert!(!any.timed_out);

    let all = coordinator
        .wait(
            "parent-1",
            &["child-1".to_string(), "child-2".to_string()],
            WaitCondition::All,
            Duration::from_millis(10),
        )
        .await
        .expect("all wait should succeed");
    assert!(all.timed_out);
    assert_eq!(
        all.tasks.iter().find(|task| task.child_session_id == "child-2").unwrap().state,
        ChildAgentTaskState::Running
    );
}

#[test]
fn coordinator_rejects_cross_parent_access_and_cancels_with_a_token() {
    let coordinator = ChildAgentCoordinator::new(8);
    let cancellation = coordinator
        .register("parent-1", "child-1", "coder", "implement")
        .expect("task should register");
    coordinator.mark_running("parent-1", "child-1").expect("task should start");

    let error = coordinator
        .snapshots("parent-2", &["child-1".to_string()])
        .expect_err("another parent must not inspect the task");
    assert!(error.to_string().contains("does not belong to parent"));

    assert!(
        coordinator.cancel("parent-1", "child-1").expect("cancel should succeed"),
        "first cancel should transition the task"
    );
    assert!(cancellation.is_cancelled());
    let snapshot = coordinator
        .snapshots("parent-1", &["child-1".to_string()])
        .expect("owner should inspect task")
        .pop()
        .expect("snapshot");
    assert_eq!(snapshot.state, ChildAgentTaskState::Cancelled);
    assert!(
        !coordinator
            .mark_succeeded("parent-1", "child-1", "late result")
            .expect("late completion should be ignored"),
        "terminal state must be immutable"
    );
}

#[test]
fn coordinator_bounds_retained_terminal_tasks_per_parent() {
    let coordinator = ChildAgentCoordinator::new(2);
    for child in ["child-1", "child-2", "child-3"] {
        coordinator.register("parent-1", child, "worker", child).expect("task should register");
        coordinator.mark_succeeded("parent-1", child, child).expect("task should complete");
    }

    let retained = coordinator.snapshots("parent-1", &[]).expect("list should succeed");
    assert_eq!(retained.len(), 2);
    assert_eq!(retained[0].child_session_id, "child-2");
    assert_eq!(retained[1].child_session_id, "child-3");
}
