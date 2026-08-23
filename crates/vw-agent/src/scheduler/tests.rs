//! 验证调度器在全局与项目作用域中的注册、查询、停止和重置。

use super::*;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[test]
fn task_new_preserves_fields_and_defaults_to_empty_instance_scope() {
    let run: RunFn = Arc::new(|| Box::pin(async { Ok(()) }));
    let task = Task::new("cleanup", Duration::from_secs(17), run.clone());

    assert_eq!(task.id, "cleanup");
    assert_eq!(task.interval, Duration::from_secs(17));
    assert!(Arc::ptr_eq(&task.run, &run));
    assert_eq!(task.scope, Scope::Instance(String::new()));
}

#[test]
fn entry_new_starts_without_tasks_or_timers() {
    let entry = Entry::new();

    assert!(entry.tasks.is_empty());
    assert!(entry.timers.is_empty());
}

#[test]
fn extra_handles_empty_multiple_and_duplicate_pairs() {
    assert!(extra::<0>([]).is_empty());

    let map = extra([("task", json!("tick")), ("attempt", json!(1)), ("task", json!("tock"))]);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get("task"), Some(&json!("tock")));
    assert_eq!(map.get("attempt"), Some(&json!(1)));
}

fn counting_task(id: &str, counter: Arc<AtomicUsize>) -> Task {
    Task::new(
        id,
        Duration::from_secs(3600),
        Arc::new(move || {
            let counter = counter.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }),
    )
}

async fn wait_for_counter(counter: &AtomicUsize, minimum: usize) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if counter.load(Ordering::SeqCst) >= minimum {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("counter did not reach {minimum}");
}

async fn yield_until_counter(counter: &AtomicUsize, minimum: usize) {
    for _ in 0..100 {
        if counter.load(Ordering::SeqCst) >= minimum {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("counter did not reach {minimum}");
}

#[test]
fn reset_instance_ignores_missing_instance() {
    let instance = format!("missing-{}", uuid::Uuid::new_v4());
    assert!(!INSTANCES.lock().unwrap_or_else(|error| error.into_inner()).contains_key(&instance));

    reset_instance(&instance);

    assert!(!INSTANCES.lock().unwrap_or_else(|error| error.into_inner()).contains_key(&instance));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn spawn_without_current_runtime_executes_future() {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);

    let _handle = spawn(async move {
        sender.send(()).expect("receiver should remain available");
    });

    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("global runtime should execute the spawned future");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn task_handle_abort_cancels_spawned_future() {
    let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
    let handle = spawn(async move {
        let _ = started_sender.send(());
        std::future::pending::<()>().await;
    });
    started_receiver.await.expect("spawned future should start");

    handle.abort();

    let error = handle.0.await.expect_err("aborted future should be cancelled");
    assert!(error.is_cancelled());
}

#[tokio::test]
async fn instance_register_runs_task_and_reset_removes_entry() {
    let id = format!("instance-run-{}", uuid::Uuid::new_v4());
    let instance = format!("inst-{}", uuid::Uuid::new_v4());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut task = counting_task(&id, counter.clone());
    task.scope = Scope::Instance(instance.clone());

    register(task);
    wait_for_counter(&counter, 1).await;
    assert!(INSTANCES.lock().unwrap().contains_key(&instance));

    reset_instance(&instance);
    assert!(!INSTANCES.lock().unwrap().contains_key(&instance));
}

#[tokio::test]
async fn instance_register_replaces_existing_timer_with_same_id() {
    let id = format!("replace-{}", uuid::Uuid::new_v4());
    let instance = format!("inst-{}", uuid::Uuid::new_v4());
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));

    let mut first_task = counting_task(&id, first.clone());
    first_task.scope = Scope::Instance(instance.clone());
    register(first_task);
    wait_for_counter(&first, 1).await;

    let mut second_task = counting_task(&id, second.clone());
    second_task.scope = Scope::Instance(instance.clone());
    register(second_task);
    wait_for_counter(&second, 1).await;

    let instances = INSTANCES.lock().unwrap();
    let entry = instances.get(&instance).expect("instance entry should exist");
    assert_eq!(entry.tasks.len(), 1);
    assert_eq!(entry.timers.len(), 1);
    drop(instances);
    reset_instance(&instance);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn instance_timer_waits_full_interval_before_next_run() {
    let id = format!("interval-wait-{}", uuid::Uuid::new_v4());
    let instance = format!("inst-{}", uuid::Uuid::new_v4());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut task = counting_task(&id, counter.clone());
    task.interval = Duration::from_secs(60);
    task.scope = Scope::Instance(instance.clone());

    register(task);
    yield_until_counter(&counter, 1).await;
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(59)).await;
    tokio::task::yield_now().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    yield_until_counter(&counter, 2).await;
    assert_eq!(counter.load(Ordering::SeqCst), 2);
    reset_instance(&instance);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn reset_instance_aborts_future_timer_ticks() {
    let id = format!("reset-abort-{}", uuid::Uuid::new_v4());
    let instance = format!("inst-{}", uuid::Uuid::new_v4());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut task = counting_task(&id, counter.clone());
    task.interval = Duration::from_secs(60);
    task.scope = Scope::Instance(instance.clone());

    register(task);
    yield_until_counter(&counter, 1).await;
    tokio::task::yield_now().await;
    reset_instance(&instance);

    tokio::time::advance(Duration::from_secs(120)).await;
    tokio::task::yield_now().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn global_register_ignores_duplicate_task_id() {
    let id = format!("global-{}", uuid::Uuid::new_v4());
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));

    let mut first_task = counting_task(&id, first.clone());
    first_task.scope = Scope::Global;
    register(first_task);
    wait_for_counter(&first, 1).await;

    let mut second_task = counting_task(&id, second.clone());
    second_task.scope = Scope::Global;
    register(second_task);
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(second.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn run_inner_invokes_failing_runner_and_returns() {
    let calls = Arc::new(AtomicUsize::new(0));
    let run: RunFn = Arc::new({
        let calls = calls.clone();
        move || {
            let calls = calls.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err("boom".to_string())
            })
        }
    });

    run_inner("failing-task", run).await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn run_once_invokes_successful_task_runner_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let task = counting_task("successful-task", calls.clone());

    run_once(task).await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
