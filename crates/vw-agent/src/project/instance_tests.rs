//! 验证项目实例上下文的访问、状态复用与分代释放。

use super::*;
use crate::app::agent::project::{Info, TimeInfo};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

// 上下文夹具同时设置项目目录和工作树，用于验证两种路径边界。
fn test_context(directory: &str, worktree: &str) -> Context {
    Context {
        directory: directory.to_string(),
        worktree: worktree.to_string(),
        project: Info {
            id: "test-project".to_string(),
            worktree: worktree.to_string(),
            vcs: None,
            name: Some("Test project".to_string()),
            icon: None,
            commands: None,
            time: TimeInfo { created: 1, updated: 2, initialized: None },
            sandboxes: Vec::new(),
        },
    }
}

#[test]
fn accessors_return_safe_defaults_without_context() {
    assert_eq!(directory(), "");
    assert_eq!(worktree(), "");
    assert!(project().is_none());
    assert!(!contains_path("/tmp/outside"));
}

#[tokio::test]
async fn dispose_without_context_preserves_safe_defaults() {
    dispose().await;

    assert_eq!(directory(), "");
    assert_eq!(worktree(), "");
    assert!(project().is_none());
}

#[tokio::test]
async fn context_accessors_and_contains_path_cover_directory_worktree_and_root_boundaries() {
    let context = test_context("/workspace/source", "/sandbox/worktree");
    let expected_project = context.project.clone();

    CONTEXT
        .scope(context, async move {
            assert_eq!(directory(), "/workspace/source");
            assert_eq!(worktree(), "/sandbox/worktree");
            assert_eq!(project(), Some(expected_project));
            assert!(contains_path("/workspace/source/child.txt"));
            assert!(contains_path("/sandbox/worktree/child.txt"));
            assert!(!contains_path("/outside/child.txt"));
        })
        .await;

    CONTEXT
        .scope(test_context("/workspace/source", "/"), async {
            assert!(!contains_path("/outside/child.txt"));
        })
        .await;
}

#[tokio::test]
// 释放后重新初始化视为新一代状态，每一代的清理器都只能执行一次。
async fn state_reuses_initialized_value_and_dispose_releases_each_generation() {
    let init_count = Arc::new(AtomicUsize::new(0));
    let dispose_count = Arc::new(AtomicUsize::new(0));
    let init_count_for_state = init_count.clone();
    let dispose_count_for_state = dispose_count.clone();
    let get_state = state(
        "instance-test-state",
        move || {
            let init_count = init_count_for_state.clone();
            async move {
                init_count.fetch_add(1, Ordering::SeqCst);
                42usize
            }
        },
        Some(move |value: Arc<usize>| {
            let dispose_count = dispose_count_for_state.clone();
            async move {
                assert_eq!(*value, 42);
                dispose_count.fetch_add(1, Ordering::SeqCst);
            }
        }),
    );

    CONTEXT
        .scope(test_context("/virtual/state-project", "/virtual/state-worktree"), async {
            let first = get_state().await;
            let second = get_state().await;
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(init_count.load(Ordering::SeqCst), 1);

            dispose().await;
            assert_eq!(dispose_count.load(Ordering::SeqCst), 1);

            let reinitialized = get_state().await;
            assert_eq!(*reinitialized, 42);
            assert!(!Arc::ptr_eq(&first, &reinitialized));
            assert_eq!(init_count.load(Ordering::SeqCst), 2);

            dispose().await;
        })
        .await;

    assert_eq!(dispose_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn provide_scopes_context_runs_init_once_and_disposes() {
    let temp = TempDir::new().expect("tempdir should create");
    let dir = temp.path().to_path_buf();
    let init_count = Arc::new(AtomicUsize::new(0));

    let init_count_first = init_count.clone();
    let observed = provide(
        &dir,
        Some(Box::new(move || {
            let init_count = init_count_first.clone();
            Box::pin(async move {
                init_count.fetch_add(1, Ordering::SeqCst);
            })
        })),
        {
            let dir = dir.clone();
            move || {
                Box::pin(async move {
                    assert_eq!(directory(), dir.to_string_lossy().to_string());
                    assert!(contains_path(dir.join("child.txt")));
                    assert!(project().is_some());
                    worktree()
                })
            }
        },
    )
    .await
    .expect("provide should succeed");
    assert!(!observed.is_empty());

    provide(
        &dir,
        Some(Box::new(|| {
            Box::pin(async {
                panic!("cached context should not run init again");
            })
        })),
        || {
            Box::pin(async {
                assert!(project().is_some());
                dispose().await;
            })
        },
    )
    .await
    .expect("cached provide should succeed");
    assert_eq!(init_count.load(Ordering::SeqCst), 1);

    assert!(directory().is_empty());
}

#[tokio::test]
async fn dispose_all_tolerates_context_removed_after_snapshot() {
    let key = format!("/virtual/dispose-race-{}", uuid::Uuid::new_v4());
    let context = Arc::new(test_context(&key, "/virtual/dispose-race-worktree"));
    let mut cache = CACHE.lock().await;
    cache.insert(key.clone(), context);

    let dispose_task = tokio::spawn(dispose_all());
    tokio::task::yield_now().await;

    let key_to_remove = key.clone();
    let remove_task =
        tokio::spawn(async move { CACHE.lock().await.remove(&key_to_remove).is_some() });
    tokio::task::yield_now().await;
    drop(cache);

    dispose_task.await.expect("dispose_all should tolerate a concurrently removed context");
    let removed = remove_task.await.expect("cache removal task should complete");
    assert!(removed);
    assert!(!CACHE.lock().await.contains_key(&key));
}
