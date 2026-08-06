#![cfg(not(target_arch = "wasm32"))]

use super::{
    MERGE_LOCK_STALE_AFTER_MS, TaskWorktreePoolService, is_managed_slot_path,
    normalize_merge_target, now_epoch_ms, sanitize_branch_token, slot_id_from_path,
};
use crate::app::agent::project;
use git2::{Repository, RepositoryInitOptions, WorktreeAddOptions};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use vw_api_types::id::TaskId;
use vw_api_types::task::TaskPoolWorktreeSlotStateDto;

/// 用仅含失败 stub 的 PATH 屏蔽真实 `git` 可执行文件。
///
/// 仅清空 PATH 不够：shell 侧解析会通过补全 PATH / 常见目录
/// 再次找到系统 Git。
struct PathGuard {
    old: Option<OsString>,
    _bin_dir: TempDir,
}

impl PathGuard {
    fn without_git() -> Self {
        let bin_dir = TempDir::new().expect("git stub bin dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let stub = bin_dir.path().join("git");
            std::fs::write(&stub, "#!/bin/sh\necho 'git stub: blocked' >&2\nexit 1\n")
                .expect("write git stub");
            let mut perms = std::fs::metadata(&stub).expect("stub metadata").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&stub, perms).expect("stub permissions");
        }
        #[cfg(windows)]
        {
            let stub = bin_dir.path().join("git.cmd");
            std::fs::write(&stub, "@echo off\r\necho git stub: blocked 1>&2\r\nexit /b 1\r\n")
                .expect("write git stub");
        }
        let old = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", bin_dir.path()) };
        Self { old, _bin_dir: bin_dir }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var("PATH", value) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

fn signature() -> git2::Signature<'static> {
    git2::Signature::now("Vibe Window", "vw@example.com").expect("signature")
}

fn commit_tree(repo: &Repository, message: &str) -> git2::Oid {
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
        index.write().expect("index write");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree");
    let sig = signature();
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).expect("commit")
}

fn git2_current_branch(path: &Path) -> String {
    crate::git_repository::current_branch(path).expect("current branch")
}

#[test]
fn worktree_pool_helpers_normalize_merge_target_and_slot_paths() {
    assert_eq!(normalize_merge_target("  release/v1  "), "release/v1");
}

#[test]
fn worktree_pool_path_helpers_reject_invalid_metadata_and_sanitize_tokens() {
    assert_eq!(sanitize_branch_token("  task / alpha!?  "), "task---alpha");
    assert_eq!(sanitize_branch_token("...___---"), "task");
    assert_eq!(slot_id_from_path(Path::new("/pool/slot-9")).unwrap(), "slot-9");
    assert!(slot_id_from_path(Path::new("/")).unwrap_err().to_string().contains("no slot id"));

    let temp = tempfile::tempdir().expect("tempdir");
    let pool_root = temp.path().join("pool");
    let managed = pool_root.join("slot-1");
    let manual = pool_root.join("manual");
    assert!(is_managed_slot_path(&managed, &pool_root));
    assert!(!is_managed_slot_path(&manual, &pool_root));
    assert!(!is_managed_slot_path(&temp.path().join("slot-outside"), &pool_root));

    let worktree = temp.path().join("worktree");
    std::fs::create_dir_all(&worktree).expect("worktree dir");
    assert_eq!(super::worktree_git_dir(&worktree).unwrap_err(), "missing .git metadata");

    std::fs::write(worktree.join(".git"), "not git metadata").expect("invalid git metadata");
    assert_eq!(super::worktree_git_dir(&worktree).unwrap_err(), "failed to parse .git metadata");

    std::fs::write(worktree.join(".git"), "gitdir: ../metadata\n").expect("relative git metadata");
    assert_eq!(
        super::worktree_git_dir(&worktree).expect("relative gitdir"),
        worktree.join("../metadata")
    );
}

mod task_worktree_pool_snapshot {
    use super::*;

    #[tokio::test]
    async fn task_worktree_pool_snapshot_counts_states() {
        let fixture = GitPoolFixture::new();
        fixture.add_slot("slot-idle");
        fixture.add_slot("slot-busy");
        fixture.add_slot("slot-tainted");
        fixture.add_slot("slot-recycling");
        fixture.add_worktree("manual-worktree", "manual-worktree", &fixture.workspace);
        fixture.add_worktree(
            "vw/task/manual-inside",
            "manual-inside",
            &fixture.pool_root.join("manual-inside"),
        );

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();

        service.snapshot(&project).await.expect("initial sync");
        service.set_slot_state_for_tests(
            "project-1",
            "slot-busy",
            TaskPoolWorktreeSlotStateDto::Busy,
            Some("task-busy"),
            None,
        );
        service.set_slot_state_for_tests(
            "project-1",
            "slot-tainted",
            TaskPoolWorktreeSlotStateDto::Tainted,
            None,
            Some("dirty"),
        );
        service.set_slot_state_for_tests(
            "project-1",
            "slot-recycling",
            TaskPoolWorktreeSlotStateDto::Recycling,
            Some("task-recycling"),
            None,
        );
        service.set_merge_lock_for_tests("project-1", "main", "task-merge", now_epoch_ms());

        let snapshot = service.snapshot(&project).await.expect("snapshot");

        assert_eq!(snapshot.project_id.0, "project-1");
        assert_eq!(
            snapshot.repo_root,
            std::fs::canonicalize(&fixture.repo).expect("canonical repo").to_string_lossy()
        );
        assert_eq!(snapshot.pool_root, fixture.pool_root.to_string_lossy());
        assert_eq!(snapshot.base_branch, "main");
        assert_eq!(snapshot.idle_count, 1);
        assert_eq!(snapshot.busy_count, 1);
        assert_eq!(snapshot.tainted_count, 1);
        assert_eq!(snapshot.recycling_count, 1);
        assert_eq!(snapshot.dead_count, 0);
        assert_eq!(snapshot.slots.len(), 4);
        assert_eq!(
            snapshot.slots.iter().map(|slot| slot.id.as_str()).collect::<Vec<_>>(),
            vec!["slot-busy", "slot-idle", "slot-recycling", "slot-tainted"]
        );
        assert_eq!(snapshot.merge_target_locks.len(), 1);
        assert_eq!(snapshot.merge_target_locks[0].target_branch, "main");
        assert_eq!(snapshot.merge_target_locks[0].task_id.0, "task-merge");

        let busy = snapshot.slots.iter().find(|slot| slot.id == "slot-busy").expect("busy slot");
        assert_eq!(busy.state, TaskPoolWorktreeSlotStateDto::Busy);
        assert_eq!(
            busy.leased_task_id.as_ref().map(|task_id| task_id.0.as_str()),
            Some("task-busy")
        );

        let tainted =
            snapshot.slots.iter().find(|slot| slot.id == "slot-tainted").expect("tainted slot");
        assert_eq!(tainted.state, TaskPoolWorktreeSlotStateDto::Tainted);
        assert_eq!(tainted.taint_reason.as_deref(), Some("dirty"));
    }

    #[tokio::test]
    async fn merge_lock_snapshot_reports_age_ms() {
        let fixture = GitPoolFixture::new();
        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();

        service.snapshot(&project).await.expect("initial sync");

        let locked_at_ms = now_epoch_ms().saturating_sub(MERGE_LOCK_STALE_AFTER_MS);
        service.set_merge_lock_for_tests("project-1", "main", "task-merge", locked_at_ms);

        let snapshot = service.snapshot(&project).await.expect("snapshot");

        assert_eq!(snapshot.merge_target_locks.len(), 1);
        let lock = &snapshot.merge_target_locks[0];
        assert_eq!(lock.target_branch, "main");
        assert_eq!(lock.task_id.0, "task-merge");
        assert!(
            lock.age_ms >= MERGE_LOCK_STALE_AFTER_MS,
            "expected age_ms >= {MERGE_LOCK_STALE_AFTER_MS}, got {}",
            lock.age_ms
        );
    }

    #[tokio::test]
    async fn task_worktree_pool_maintenance_ignores_dead_capacity() {
        let fixture = GitPoolFixture::new();
        fixture.add_slot("slot-dead");
        fixture.add_slot("slot-tainted");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();

        service.snapshot(&project).await.expect("initial sync");
        service.set_slot_state_for_tests(
            "project-1",
            "slot-dead",
            TaskPoolWorktreeSlotStateDto::Dead,
            None,
            Some("missing gitdir"),
        );
        service.set_slot_state_for_tests(
            "project-1",
            "slot-tainted",
            TaskPoolWorktreeSlotStateDto::Tainted,
            None,
            Some("dirty"),
        );

        let response = service.maintain(&project, 1).await.expect("maintain");
        let snapshot = response.snapshot.expect("maintenance snapshot");

        assert_eq!(response.idle_count, 1);
        assert_eq!(response.target_idle_count, 1);
        assert_eq!(snapshot.dead_count, 1);
        assert_eq!(snapshot.tainted_count, 1);
        assert_eq!(snapshot.idle_count, 1);
        assert_eq!(snapshot.slots.len(), 3);
        assert!(
            snapshot.slots.iter().any(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Idle
                && slot.id.starts_with("slot-"))
        );
    }

    #[tokio::test]
    async fn task_worktree_pool_health_marks_invalid_slot_dead() {
        let fixture = GitPoolFixture::new();
        let slot_path = fixture.add_slot("slot-broken");
        let git_dir = worktree_git_dir(&slot_path);
        std::fs::write(git_dir.join("index.lock"), "").expect("write index lock");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let snapshot = service.snapshot(&fixture.project()).await.expect("snapshot");

        assert_eq!(snapshot.idle_count, 0);
        assert_eq!(snapshot.dead_count, 1);
        let slot =
            snapshot.slots.iter().find(|slot| slot.id == "slot-broken").expect("broken slot");
        assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Dead);
        assert!(slot.taint_reason.as_deref().is_some_and(|reason| reason.contains("index.lock")));
    }
}

mod task_worktree_pool_lifecycle {
    use super::*;

    #[tokio::test]
    async fn task_worktree_pool_acquire_prepares_branch() {
        let fixture = GitPoolFixture::new();
        fixture.add_slot("slot-ready");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-alpha".to_string());

        let first =
            service.acquire_and_prepare(&project, &task_id).await.expect("acquire first slot");
        let slot = first.slot.expect("leased slot");

        assert!(first.leased);
        assert_eq!(slot.id, "slot-ready");
        assert_eq!(slot.branch, "vw/task/task-alpha");
        assert_eq!(current_branch(Path::new(&slot.path)), "vw/task/task-alpha");

        let second =
            service.acquire_and_prepare(&project, &task_id).await.expect("reuse assigned slot");
        let reused = second.slot.expect("reused slot");

        assert_eq!(reused.id, slot.id);
        assert_eq!(reused.state, TaskPoolWorktreeSlotStateDto::Busy);
        assert_eq!(
            reused.leased_task_id.as_ref().map(|task_id| task_id.0.as_str()),
            Some("task-alpha")
        );
    }

    #[tokio::test]
    async fn task_worktree_pool_concurrent_acquire_same_task_reuses_one_slot() {
        // 复现 "branch is already used by worktree"：同一任务被重复触发时并发 acquire，
        // 若未串行化会各自租到不同空闲槽位并对同一分支 `checkout -B`，git 拒绝第二个。
        let fixture = GitPoolFixture::new();
        fixture.add_slot("slot-a");
        fixture.add_slot("slot-b");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-concurrent".to_string());

        let first = {
            let service = service.clone();
            let project = project.clone();
            let task_id = task_id.clone();
            tokio::spawn(async move { service.acquire_and_prepare(&project, &task_id).await })
        };
        let second = {
            let service = service.clone();
            let project = project.clone();
            let task_id = task_id.clone();
            tokio::spawn(async move { service.acquire_and_prepare(&project, &task_id).await })
        };

        let first = first.await.expect("join first").expect("acquire first");
        let second = second.await.expect("join second").expect("acquire second");

        let first_slot = first.slot.expect("first leased slot");
        let second_slot = second.slot.expect("second leased slot");

        // 两次 acquire 必须复用同一槽位与分支，且不留下第二个被占用的空闲槽位。
        assert_eq!(first_slot.id, second_slot.id);
        assert_eq!(first_slot.branch, "vw/task/task-concurrent");
        assert_eq!(second_slot.branch, "vw/task/task-concurrent");

        let snapshot = service.snapshot(&project).await.expect("snapshot");
        let busy = snapshot
            .slots
            .iter()
            .filter(|slot| slot.state == TaskPoolWorktreeSlotStateDto::Busy)
            .count();
        assert_eq!(busy, 1, "only one slot should be leased to the task");
        assert_eq!(snapshot.idle_count, 1);
    }

    #[tokio::test]
    async fn task_worktree_pool_release_returns_idle() {
        let fixture = GitPoolFixture::new();
        fixture.add_slot("slot-ready");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-release".to_string());

        service.acquire_and_prepare(&project, &task_id).await.expect("acquire slot");
        let released = service.release_task_slot(&project, &task_id).await.expect("release slot");
        let snapshot = service.snapshot(&project).await.expect("snapshot");
        let slot = snapshot.slots.iter().find(|slot| slot.id == "slot-ready").expect("slot");

        assert!(released.released);
        assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Idle);
        assert_eq!(slot.leased_task_id, None);
        assert_eq!(snapshot.idle_count, 1);
        assert_eq!(snapshot.busy_count, 0);
    }

    #[tokio::test]
    async fn task_worktree_pool_recycle_marks_dead_on_reset_failure() {
        let fixture = GitPoolFixture::new();
        let slot_path = fixture.add_slot("slot-broken-reset");
        let git_dir = worktree_git_dir(&slot_path);

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-recycle".to_string());

        service.acquire_and_prepare(&project, &task_id).await.expect("acquire slot");
        std::fs::write(git_dir.join("index.lock"), "").expect("write index lock");

        let error = service
            .recycle_task_slot(&project, &task_id, Some("execution failed".to_string()))
            .await
            .expect_err("recycle should fail");
        let snapshot = service.snapshot(&project).await.expect("snapshot");
        let slot = snapshot.slots.iter().find(|slot| slot.id == "slot-broken-reset").expect("slot");

        assert!(error.to_string().contains("failed to reset task worktree slot"));
        assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Dead);
        assert_eq!(slot.leased_task_id, None);
        assert!(slot.taint_reason.as_deref().is_some_and(|reason| reason.contains("index.lock")));
    }

    #[tokio::test]
    async fn task_worktree_pool_acquire_replaces_slot_after_prepare_failure() {
        let fixture = GitPoolFixture::new();
        let slot_path = fixture.add_slot("slot-corrupt-index");
        let git_dir = worktree_git_dir(&slot_path);
        std::fs::write(git_dir.join("index"), "bad").expect("corrupt worktree index");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-replacement".to_string());

        let response = service
            .acquire_and_prepare(&project, &task_id)
            .await
            .expect("acquire replacement slot");
        let slot = response.slot.expect("leased replacement slot");

        assert!(response.leased);
        assert_ne!(slot.id, "slot-corrupt-index");
        assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Busy);
        assert_eq!(slot.branch, "vw/task/task-replacement");

        let snapshot = service.snapshot(&project).await.expect("snapshot");
        let corrupt = snapshot
            .slots
            .iter()
            .find(|slot| slot.id == "slot-corrupt-index")
            .expect("corrupt slot");
        assert_eq!(corrupt.state, TaskPoolWorktreeSlotStateDto::Tainted);
        assert_eq!(corrupt.leased_task_id, None);
        assert!(
            corrupt
                .taint_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("failed to reset task worktree slot"))
        );
    }

    #[tokio::test]
    async fn task_worktree_pool_merge_lock_excludes_other_tasks() {
        let fixture = GitPoolFixture::new();
        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_a = TaskId("task-a".to_string());
        let task_b = TaskId("task-b".to_string());

        let locked =
            service.try_lock_merge_target(&project, &task_a, "main").await.expect("lock target");
        let relocked = service
            .try_lock_merge_target(&project, &task_a, "main")
            .await
            .expect("same task relock");
        let denied = service
            .try_lock_merge_target(&project, &task_b, "main")
            .await
            .expect("other task denied");

        assert!(locked.locked);
        assert!(relocked.locked);
        assert!(!denied.locked);
        assert_eq!(
            denied.holder_task_id.as_ref().map(|task_id| task_id.0.as_str()),
            Some("task-a")
        );
        assert_eq!(
            service.merge_lock_holder(&project, "main").await.expect("holder").as_ref(),
            Some(&task_a)
        );

        service.unlock_merge_target(&project, &task_b, "main").await.expect("unlock non-holder");
        assert_eq!(
            service.merge_lock_holder(&project, "main").await.expect("holder").as_ref(),
            Some(&task_a)
        );

        service.unlock_merge_target(&project, &task_a, "main").await.expect("unlock holder");
        assert_eq!(service.merge_lock_holder(&project, "main").await.expect("cleared"), None);
    }

    #[tokio::test]
    async fn merge_lock_stale_holder_can_be_preempted() {
        let fixture = GitPoolFixture::new();
        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_b = TaskId("task-b".to_string());

        // task-a 持有 main 的合并锁，但 locked_at_ms 久远到已超 TTL（崩溃后未释放）。
        service.snapshot(&project).await.expect("initial sync");
        let stale_locked_at = now_epoch_ms() - MERGE_LOCK_STALE_AFTER_MS - 1;
        service.set_merge_lock_for_tests("project-1", "main", "task-a", stale_locked_at);

        let preempted = service
            .try_lock_merge_target(&project, &task_b, "main")
            .await
            .expect("preempt stale lock");

        assert!(preempted.locked, "陈旧锁应被新任务抢占");
        assert_eq!(
            preempted.holder_task_id.as_ref().map(|task_id| task_id.0.as_str()),
            Some("task-b")
        );
        assert_eq!(
            service.merge_lock_holder(&project, "main").await.expect("holder").as_ref(),
            Some(&task_b)
        );
    }

    #[tokio::test]
    async fn merge_lock_fresh_holder_blocks_other_task() {
        let fixture = GitPoolFixture::new();
        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_a = TaskId("task-a".to_string());
        let task_b = TaskId("task-b".to_string());

        // task-a 刚刚持锁（未过期），task-b 不得抢占。
        service.snapshot(&project).await.expect("initial sync");
        service.set_merge_lock_for_tests("project-1", "main", "task-a", now_epoch_ms());

        let denied = service
            .try_lock_merge_target(&project, &task_b, "main")
            .await
            .expect("fresh lock blocks other task");

        assert!(!denied.locked, "未过期锁对其他任务仍互斥");
        assert_eq!(
            denied.holder_task_id.as_ref().map(|task_id| task_id.0.as_str()),
            Some("task-a")
        );
        assert_eq!(
            service.merge_lock_holder(&project, "main").await.expect("holder").as_ref(),
            Some(&task_a)
        );
    }

    #[tokio::test]
    async fn task_worktree_pool_acquire_beyond_prewarm_cap() {
        // 复现「执行网关前预分配失败：gateway 未分配任务 worktree 槽位」。
        // maintain(1) 的 max_worktrees=2 上限只用于预热，按需分配不应受其限制：
        // 即使已有 2 个 busy 槽位，第三个任务仍必须拿到属于自己的槽位。
        let fixture = GitPoolFixture::new();
        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();

        let mut leased_slot_ids = Vec::new();
        for index in 0..3 {
            let task_id = TaskId(format!("task-concurrent-{index}"));
            let response = service
                .acquire_and_prepare(&project, &task_id)
                .await
                .expect("acquire concurrent slot");
            assert!(response.leased, "task {index} 必须分配到 worktree 槽位");
            let slot = response.slot.expect("leased slot");
            assert_eq!(slot.state, TaskPoolWorktreeSlotStateDto::Busy);
            assert_eq!(
                slot.leased_task_id.as_ref().map(|task_id| task_id.0.as_str()),
                Some(format!("task-concurrent-{index}").as_str())
            );
            leased_slot_ids.push(slot.id);
        }

        leased_slot_ids.sort();
        leased_slot_ids.dedup();
        assert_eq!(leased_slot_ids.len(), 3, "三个任务应各自占用独立槽位");

        let snapshot = service.snapshot(&project).await.expect("snapshot");
        assert_eq!(snapshot.busy_count, 3);
    }

    #[tokio::test]
    async fn task_worktree_pool_git2_lifecycle_works_without_git_executable() {
        let _env_lock =
            crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let _async_env_lock = crate::test_support::global_async_env_lock().lock().await;

        let fixture = Git2PoolFixture::new();
        fixture.add_slot("slot-ready");
        std::fs::write(fixture.repo.join(".gitignore"), "ignored.txt\n").expect("gitignore");
        // 重新提交 ignore，便于 recycle 时验证忽略文件被保留。
        let repo = Repository::open(&fixture.repo).expect("reopen repo");
        commit_tree(&repo, "add gitignore");
        drop(repo);

        let service = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
        let project = fixture.project();
        let task_id = TaskId("task-git2".to_string());

        let _path = PathGuard::without_git();

        let snapshot = service.snapshot(&project).await.expect("snapshot without git CLI");
        assert_eq!(snapshot.base_branch, "main");
        assert_eq!(snapshot.idle_count, 1);
        assert_eq!(snapshot.slots.len(), 1);
        assert_eq!(snapshot.slots[0].id, "slot-ready");

        let maintained = service.maintain(&project, 2).await.expect("maintain without git CLI");
        assert!(maintained.idle_count >= 2);
        assert_eq!(maintained.target_idle_count, 2);

        let acquired =
            service.acquire_and_prepare(&project, &task_id).await.expect("acquire without git CLI");
        assert!(acquired.leased);
        let slot = acquired.slot.expect("leased slot");
        assert_eq!(slot.branch, "vw/task/task-git2");
        assert_eq!(git2_current_branch(Path::new(&slot.path)), "vw/task/task-git2");

        let slot_path = PathBuf::from(&slot.path);
        std::fs::write(slot_path.join("scratch.txt"), "dirty").expect("untracked file");
        std::fs::write(slot_path.join("ignored.txt"), "keep").expect("ignored file");

        let reused = service
            .acquire_and_prepare(&project, &task_id)
            .await
            .expect("re-acquire without git CLI");
        assert_eq!(reused.slot.expect("reused").id, slot.id);

        // 另一任务不得占用已在本槽检出的分支（跨 worktree 冲突由 git2 拒绝）。
        let other = TaskId("task-other".to_string());
        let other_acquired = service
            .acquire_and_prepare(&project, &other)
            .await
            .expect("other task acquires different slot");
        let other_slot = other_acquired.slot.expect("other slot");
        assert_ne!(other_slot.id, slot.id);
        assert_eq!(other_slot.branch, "vw/task/task-other");

        service
            .recycle_task_slot(&project, &task_id, None)
            .await
            .expect("recycle without git CLI");
        assert!(!slot_path.join("scratch.txt").exists(), "untracked file should be cleaned");
        assert!(slot_path.join("ignored.txt").exists(), "ignored file should be kept");

        let after = service.snapshot(&project).await.expect("snapshot after recycle");
        let recycled =
            after.slots.iter().find(|entry| entry.id == slot.id).expect("recycled slot");
        assert_eq!(recycled.state, TaskPoolWorktreeSlotStateDto::Idle);
        assert_eq!(recycled.leased_task_id, None);
    }

    #[tokio::test]
    async fn task_worktree_pool_requires_initial_commit_for_unborn_repo() {
        let _env_lock =
            crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let _async_env_lock = crate::test_support::global_async_env_lock().lock().await;

        let temp = TempDir::new().expect("tempdir");
        let repo = temp.path().join("unborn");
        let pool_root = temp.path().join("task-worktrees");
        std::fs::create_dir_all(&pool_root).expect("pool root");
        Repository::init(&repo).expect("unborn repo");

        let service = TaskWorktreePoolService::with_pool_root_for_tests(pool_root);
        let project = project::Info {
            id: "project-unborn".to_string(),
            worktree: repo.to_string_lossy().to_string(),
            vcs: Some(project::Vcs::Git),
            name: None,
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        };

        let _path = PathGuard::without_git();

        let error = service.snapshot(&project).await.expect_err("unborn repo should fail");
        let message = error.to_string();
        assert!(
            message.contains("需要初始提交") || message.contains("requires initial commit"),
            "expected initial-commit error, got: {message}"
        );
        assert!(
            !message.to_lowercase().contains("non-git")
                && !message.contains("requires a git project"),
            "unborn repo must not be reported as non-git: {message}"
        );
    }
}

struct GitPoolFixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    repo: PathBuf,
    pool_root: PathBuf,
}

type Git2PoolFixture = GitPoolFixture;

impl GitPoolFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let workspace = temp.path().join("workspace");
        let repo = temp.path().join("repo");
        let pool_root = temp.path().join("task-worktrees");
        std::fs::create_dir_all(&pool_root).expect("pool root");
        init_repo_git2(&repo);
        Self { _temp: temp, workspace, repo, pool_root }
    }

    fn project(&self) -> project::Info {
        project::Info {
            id: "project-1".to_string(),
            worktree: self.repo.to_string_lossy().to_string(),
            vcs: Some(project::Vcs::Git),
            name: None,
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        }
    }

    fn add_slot(&self, slot_id: &str) -> PathBuf {
        let path = self.pool_root.join(slot_id);
        self.add_worktree(&format!("vw/task/{slot_id}"), slot_id, &path);
        path
    }

    fn add_worktree(&self, branch: &str, branch_token: &str, path: &Path) {
        if path.exists() {
            std::fs::remove_dir_all(path).expect("clear worktree destination");
        }
        let repo = Repository::open(&self.repo).expect("open repo");
        let commit = repo
            .revparse_single("main")
            .expect("main")
            .peel_to_commit()
            .expect("main commit");
        let git_branch = repo.branch(branch, &commit, false).expect("create branch");
        let reference = git_branch.into_reference();
        let mut opts = WorktreeAddOptions::new();
        opts.reference(Some(&reference));
        let name = path.file_name().and_then(|value| value.to_str()).expect("worktree name");
        repo.worktree(name, path, Some(&mut opts)).expect("add worktree");
        std::fs::write(path.join(format!("{branch_token}.txt")), branch_token)
            .expect("write worktree marker");
    }
}

fn init_repo_git2(path: &Path) {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(path, &opts).expect("git2 init");
    std::fs::write(path.join("README.md"), "demo").expect("write readme");
    commit_tree(&repo, "init");
}

fn worktree_git_dir(path: &Path) -> PathBuf {
    let dot_git = path.join(".git");
    if dot_git.is_dir() {
        return dot_git;
    }
    let content = std::fs::read_to_string(&dot_git).expect("read .git file");
    let gitdir = content
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))
        .expect("gitdir entry");
    let path = PathBuf::from(gitdir);
    if path.is_absolute() { path } else { dot_git.parent().expect("worktree path").join(path) }
}

fn current_branch(path: &Path) -> String {
    git2_current_branch(path)
}
