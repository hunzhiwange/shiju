//! 验证项目状态缓存键、并发初始化、类型冲突与释放流程。

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

#[tokio::test]
async fn create_reuses_state_for_same_root_and_name() {
    let get_state = create(
        || "root-a".to_string(),
        "state_tests_create_reuses_state",
        || async { 7_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );

    let first = get_state().await;
    let second = get_state().await;
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(*first, 7);
}

#[tokio::test]
async fn create_uses_root_and_name_as_cache_key() {
    let root_a = format!("root-a-{}", Uuid::new_v4());
    let root_b = format!("root-b-{}", Uuid::new_v4());
    let get_a = create(
        move || root_a.clone(),
        "state_tests_keyed",
        || async { 1_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );
    let get_b = create(
        move || root_b.clone(),
        "state_tests_keyed",
        || async { 2_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );

    assert_eq!(*get_a().await, 1);
    assert_eq!(*get_b().await, 2);
}

#[tokio::test]
async fn create_uses_name_to_separate_states_within_same_root() {
    let root = format!("same-root-{}", Uuid::new_v4());
    let get_a = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_named_a",
        || async { 1_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );
    let get_b = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_named_b",
        || async { 2_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );

    let state_a = get_a().await;
    let state_b = get_b().await;
    assert_eq!(*state_a, 1);
    assert_eq!(*state_b, 2);
    assert!(!Arc::ptr_eq(&state_a, &state_b));

    dispose(&root).await;
}

#[tokio::test]
async fn create_initializes_once_when_called_concurrently() {
    let root = format!("concurrent-root-{}", Uuid::new_v4());
    let initialized = Arc::new(AtomicUsize::new(0));
    let get_state = Arc::new(create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_concurrent",
        {
            let initialized = initialized.clone();
            move || {
                let initialized = initialized.clone();
                async move {
                    initialized.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    42_u32
                }
            }
        },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    ));

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let get_state = get_state.clone();
        tasks.push(tokio::spawn(async move { get_state().await }));
    }

    let first = tasks.remove(0).await.expect("state initialization task should complete");
    for task in tasks {
        let state = task.await.expect("state initialization task should complete");
        assert!(Arc::ptr_eq(&first, &state));
    }
    assert_eq!(initialized.load(Ordering::SeqCst), 1);

    dispose(&root).await;
}

#[tokio::test]
async fn create_evaluates_root_for_each_request() {
    let prefix = format!("dynamic-root-{}", Uuid::new_v4());
    let root_calls = Arc::new(AtomicUsize::new(0));
    let initialized = Arc::new(AtomicUsize::new(0));
    let get_state = create(
        {
            let prefix = prefix.clone();
            let root_calls = root_calls.clone();
            move || format!("{prefix}-{}", root_calls.fetch_add(1, Ordering::SeqCst))
        },
        "state_tests_dynamic_root",
        {
            let initialized = initialized.clone();
            move || {
                let initialized = initialized.clone();
                async move { initialized.fetch_add(1, Ordering::SeqCst) + 1 }
            }
        },
        None::<fn(Arc<usize>) -> std::future::Ready<()>>,
    );

    let first = get_state().await;
    let second = get_state().await;
    assert_eq!(*first, 1);
    assert_eq!(*second, 2);
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(root_calls.load(Ordering::SeqCst), 2);

    dispose(&format!("{prefix}-0")).await;
    dispose(&format!("{prefix}-1")).await;
}

#[tokio::test]
async fn create_uses_initializer_from_first_invoked_getter_for_shared_key() {
    let root = format!("shared-key-root-{}", Uuid::new_v4());
    let first_initializer_calls = Arc::new(AtomicUsize::new(0));
    let second_initializer_calls = Arc::new(AtomicUsize::new(0));
    let get_first = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_first_invoked",
        {
            let calls = first_initializer_calls.clone();
            move || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    1_u32
                }
            }
        },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );
    let get_second = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_first_invoked",
        {
            let calls = second_initializer_calls.clone();
            move || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    2_u32
                }
            }
        },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );

    assert_eq!(*get_second().await, 2);
    assert_eq!(*get_first().await, 2);
    assert_eq!(first_initializer_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second_initializer_calls.load(Ordering::SeqCst), 1);

    dispose(&root).await;
}

#[tokio::test]
async fn create_rejects_type_conflict_for_existing_root_and_name() {
    let root = format!("type-conflict-root-{}", Uuid::new_v4());
    let get_u32 = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_type_conflict",
        || async { 32_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );
    let get_u64 = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_type_conflict",
        || async { 64_u64 },
        None::<fn(Arc<u64>) -> std::future::Ready<()>>,
    );

    assert_eq!(*get_u32().await, 32);
    let error = tokio::spawn(async move { get_u64().await })
        .await
        .expect_err("conflicting state type should panic");
    assert!(error.is_panic());

    dispose(&root).await;
}

#[tokio::test]
async fn dispose_calls_initialized_state_disposers_only_once() {
    let root = format!("dispose-root-{}", Uuid::new_v4());
    let disposed = Arc::new(AtomicUsize::new(0));

    let initialized = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "initialized",
        || async { 9_u32 },
        Some({
            let disposed = disposed.clone();
            move |_value: Arc<u32>| {
                let disposed = disposed.clone();
                async move {
                    disposed.fetch_add(1, Ordering::SeqCst);
                }
            }
        }),
    );
    let _lazy = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "lazy",
        || async { 10_u32 },
        Some({
            let disposed = disposed.clone();
            move |_value: Arc<u32>| {
                let disposed = disposed.clone();
                async move {
                    disposed.fetch_add(10, Ordering::SeqCst);
                }
            }
        }),
    );

    assert_eq!(*initialized().await, 9);
    dispose(&root).await;
    assert_eq!(disposed.load(Ordering::SeqCst), 1);
    dispose(&root).await;
    assert_eq!(disposed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn dispose_without_disposer_removes_state_and_allows_reinitialization() {
    let root = format!("dispose-without-handler-root-{}", Uuid::new_v4());
    let initialized = Arc::new(AtomicUsize::new(0));
    let get_state = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_dispose_without_handler",
        {
            let initialized = initialized.clone();
            move || {
                let initialized = initialized.clone();
                async move { initialized.fetch_add(1, Ordering::SeqCst) + 1 }
            }
        },
        None::<fn(Arc<usize>) -> std::future::Ready<()>>,
    );

    let first = get_state().await;
    assert_eq!(*first, 1);
    dispose(&root).await;

    let second = get_state().await;
    assert_eq!(*second, 2);
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(initialized.load(Ordering::SeqCst), 2);

    dispose(&root).await;
}

#[tokio::test]
async fn dispose_awaits_each_initialized_state_disposer() {
    let root = format!("multiple-disposers-root-{}", Uuid::new_v4());
    let disposed_total = Arc::new(AtomicUsize::new(0));
    let create_state = |name, value| {
        create(
            {
                let root = root.clone();
                move || root.clone()
            },
            name,
            move || async move { value },
            Some({
                let disposed_total = disposed_total.clone();
                move |value: Arc<usize>| {
                    let disposed_total = disposed_total.clone();
                    async move {
                        tokio::task::yield_now().await;
                        disposed_total.fetch_add(*value, Ordering::SeqCst);
                    }
                }
            }),
        )
    };
    let get_three = create_state("state_tests_dispose_three", 3_usize);
    let get_seven = create_state("state_tests_dispose_seven", 7_usize);

    assert_eq!(*get_three().await, 3);
    assert_eq!(*get_seven().await, 7);
    dispose(&root).await;

    assert_eq!(disposed_total.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn state_cache_recovers_after_mutex_poisoning() {
    let poisoned = std::panic::catch_unwind(|| {
        let _guard = STATE.lock().unwrap_or_else(|error| error.into_inner());
        panic!("poison state cache for recovery coverage");
    });
    assert!(poisoned.is_err());

    let root = format!("poison-recovery-root-{}", Uuid::new_v4());
    let get_state = create(
        {
            let root = root.clone();
            move || root.clone()
        },
        "state_tests_poison_recovery",
        || async { 17_u32 },
        None::<fn(Arc<u32>) -> std::future::Ready<()>>,
    );

    assert_eq!(*get_state().await, 17);
    dispose(&root).await;
}
