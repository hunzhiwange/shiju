//! 验证可见任务会话协调器的状态、归属和事件等待边界。

use super::task_session::{TaskSessionCoordinator, TaskSessionState, TaskSessionWaitCondition};
use std::time::{Duration, Instant};

#[tokio::test]
async fn task_session_wait_wakes_on_terminal_transition() {
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register("parent-1", "task-1").expect("task should register");
    coordinator.mark_running("parent-1", "task-1").expect("task should start");

    let waiting_coordinator = coordinator.clone();
    let waiter = tokio::spawn(async move {
        waiting_coordinator
            .wait(
                "parent-1",
                &["task-1".to_string()],
                TaskSessionWaitCondition::All,
                Duration::from_secs(1),
            )
            .await
            .expect("wait should succeed")
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished(), "running task must keep wait pending");

    assert!(coordinator.mark_succeeded("parent-1", "task-1").expect("task should complete"));
    assert!(
        !coordinator
            .mark_failed("parent-1", "task-1", "late failure")
            .expect("terminal transition should be idempotent")
    );

    let outcome = tokio::time::timeout(Duration::from_millis(200), waiter)
        .await
        .expect("terminal notification should wake wait")
        .expect("wait task should join");
    assert!(!outcome.timed_out);
    assert_eq!(outcome.tasks[0].state, TaskSessionState::Succeeded);
}

#[tokio::test]
async fn task_session_wait_times_out_without_state_change() {
    let coordinator = TaskSessionCoordinator::new();
    for task_id in ["task-1", "task-2"] {
        coordinator.register("parent-1", task_id).expect("task should register");
        coordinator.mark_running("parent-1", task_id).expect("task should start");
    }
    coordinator.mark_succeeded("parent-1", "task-1").expect("first task should complete");

    let any = coordinator
        .wait(
            "parent-1",
            &["task-1".to_string(), "task-1".to_string(), "task-2".to_string()],
            TaskSessionWaitCondition::Any,
            Duration::ZERO,
        )
        .await
        .expect("any snapshot should succeed");
    assert!(!any.timed_out);
    assert_eq!(any.tasks.len(), 2, "target IDs should be deduplicated");

    let started = Instant::now();
    let all = coordinator
        .wait(
            "parent-1",
            &["task-1".to_string(), "task-2".to_string()],
            TaskSessionWaitCondition::All,
            Duration::from_millis(15),
        )
        .await
        .expect("all wait should succeed");
    assert!(all.timed_out);
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(all.tasks[1].state, TaskSessionState::Running);
}

#[tokio::test]
async fn task_session_scope_rejects_unknown_ids() {
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register("parent-1", "task-1").expect("task should register");

    let unknown = coordinator
        .snapshots("parent-1", &["missing".to_string()])
        .expect_err("unknown task must fail");
    assert!(unknown.to_string().contains("unknown task session 'missing'"));

    let cross_parent = coordinator
        .wait("parent-2", &["task-1".to_string()], TaskSessionWaitCondition::All, Duration::ZERO)
        .await
        .expect_err("another parent must not access the task");
    assert!(cross_parent.to_string().contains("does not belong to parent 'parent-2'"));
}

#[test]
fn task_session_begin_turn_rejects_busy_and_reopens_terminal_task() {
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register("parent-1", "task-1").expect("task should register");
    let busy = coordinator.begin_turn("parent-1", "task-1").expect_err("queued task is busy");
    assert!(busy.to_string().contains("busy"));

    coordinator.mark_running("parent-1", "task-1").expect("task should start");
    coordinator.mark_failed("parent-1", "task-1", "old failure").expect("task should fail");
    coordinator.begin_turn("parent-1", "task-1").expect("terminal task should reopen");
    let snapshot =
        coordinator.snapshots("parent-1", &["task-1".to_string()]).expect("snapshot").remove(0);
    assert_eq!(snapshot.state, TaskSessionState::Running);
    assert_eq!(snapshot.error, None);
}
