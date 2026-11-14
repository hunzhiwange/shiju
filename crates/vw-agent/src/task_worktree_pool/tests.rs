use super::*;
use tempfile::TempDir;
#[tokio::test]
async fn task_pool_capacity_maintain_preserves_target_without_creating_git_directories() {
    let root = TempDir::new().unwrap();
    let pool_root = root.path().join("pool");
    let service = TaskWorktreePoolService::with_pool_root_for_tests(pool_root.clone());
    let project = project::Info {
        id: "logical-capacity".into(),
        worktree: root.path().to_string_lossy().into_owned(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    let result = service.maintain(&project, 7).await.unwrap();
    assert_eq!(result.target_idle_count, 7);
    assert_eq!(result.idle_count, 7);
    assert!(!pool_root.exists());
    assert!(!root.path().join(".git").exists());
}

fn fixture() -> (TempDir, TaskWorktreePoolService, project::Info) {
    let root = TempDir::new().unwrap();
    let project = project::Info {
        id: "logical".into(),
        worktree: root.path().to_string_lossy().into_owned(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    let pool = TaskWorktreePoolService::with_pool_root_for_tests(root.path().join("pool"));
    (root, pool, project)
}

#[tokio::test]
async fn task_pool_capacity_recycle_preserves_files_and_other_active_runs() {
    let (root, pool, project) = fixture();
    std::fs::write(root.path().join("output.txt"), "user content").unwrap();
    pool.maintain(&project, 3).await.unwrap();
    let first = TaskId("first".into());
    let second = TaskId("second".into());
    let a = pool.acquire_and_prepare(&project, &first).await.unwrap().slot.unwrap();
    let b = pool.acquire_and_prepare(&project, &second).await.unwrap().slot.unwrap();
    assert_ne!(a.id, b.id);
    assert!(pool.recycle_slot(&project, &b.id).await.is_err());
    pool.recycle_task_slot(&project, &first, None).await.unwrap();
    let snapshot = pool.snapshot(&project).await.unwrap();
    assert_eq!(snapshot.busy_count, 1);
    assert_eq!(
        snapshot.slots.iter().find(|s| s.id == b.id).unwrap().leased_task_id.as_ref(),
        Some(&second)
    );
    assert_eq!(std::fs::read_to_string(root.path().join("output.txt")).unwrap(), "user content");
    pool.maintain(&project, 1).await.unwrap();
    assert_eq!(pool.snapshot(&project).await.unwrap().busy_count, 1);
}

#[tokio::test]
async fn task_pool_capacity_concurrent_acquire_is_idempotent_and_preserves_merge_locks() {
    let (_root, pool, project) = fixture();
    let task = TaskId("task".into());
    let (a, b) = tokio::join!(
        pool.acquire_and_prepare(&project, &task),
        pool.acquire_and_prepare(&project, &task)
    );
    assert_eq!(a.unwrap().slot.unwrap().id, b.unwrap().slot.unwrap().id);
    assert!(pool.try_lock_merge_target(&project, &task, "main").await.unwrap().locked);
    let other = TaskId("other".into());
    assert!(!pool.try_lock_merge_target(&project, &other, "main").await.unwrap().locked);
    pool.unlock_merge_target(&project, &task, "main").await.unwrap();
    assert!(pool.try_lock_merge_target(&project, &other, "main").await.unwrap().locked);
}

#[tokio::test]
async fn task_pool_capacity_slots_reuse_existing_agents_and_independent_runs() {
    let (_root, pool, project) = fixture();
    let task = TaskId("task".into());
    let a = pool.bind_run(&project, &task, "run-a", Some("existing-agent".into())).await.unwrap();
    let b = pool.bind_run(&project, &task, "run-b", Some("existing-agent".into())).await.unwrap();
    assert_ne!(a.id, b.id);
    assert_eq!(a.agent_id, b.agent_id);
    assert!(a.path.is_empty());
    assert!(a.branch.is_empty());
    assert!(pool.release_task_slot(&project, &task).await.is_err());
    pool.finish_run(&project, &task, "run-a", None).await.unwrap();
    assert!(!pool.finish_run(&project, &task, "run-a", None).await.unwrap());
    let snapshot = pool.snapshot(&project).await.unwrap();
    assert_eq!(snapshot.busy_count, 1);
    assert_eq!(
        snapshot.slots.iter().find(|s| s.id == b.id).unwrap().run_id.as_deref(),
        Some("run-b")
    );
}

#[tokio::test]
async fn task_pool_capacity_preserves_existing_git_files_and_state_partitions() {
    let (root, pool, project) = fixture();
    let old_slot = root.path().join("pool/slot-old");
    std::fs::create_dir_all(&old_slot).unwrap();
    std::fs::write(old_slot.join(".git"), "existing metadata").unwrap();
    std::fs::write(old_slot.join("uncommitted"), "preserve").unwrap();
    pool.maintain(&project, 4).await.unwrap();
    {
        let mut pools = pool.pools.lock().unwrap();
        let slots = &mut pools.get_mut(&project.id).unwrap().slots;
        slots[0].state = TaskPoolWorktreeSlotStateDto::Dead;
        slots[1].state = TaskPoolWorktreeSlotStateDto::Tainted;
        slots[2].state = TaskPoolWorktreeSlotStateDto::Recycling;
    }
    let snapshot = pool.snapshot(&project).await.unwrap();
    assert_eq!(
        (
            snapshot.idle_count,
            snapshot.dead_count,
            snapshot.tainted_count,
            snapshot.recycling_count
        ),
        (1, 1, 1, 1)
    );
    assert!(snapshot.slots.iter().all(|slot| slot.id != "slot-old"));
    assert_eq!(std::fs::read_to_string(old_slot.join("uncommitted")).unwrap(), "preserve");
    let owner = TaskId("owner".into());
    pool.try_lock_merge_target(&project, &owner, "main").await.unwrap();
    {
        let mut pools = pool.pools.lock().unwrap();
        pools
            .get_mut(&project.id)
            .unwrap()
            .merge_target_locks
            .get_mut("main")
            .unwrap()
            .locked_at_ms = now_epoch_ms().saturating_sub(MERGE_LOCK_STALE_AFTER_MS + 1);
    }
    assert!(
        pool.try_lock_merge_target(&project, &TaskId("new-owner".into()), "main")
            .await
            .unwrap()
            .locked
    );
}
