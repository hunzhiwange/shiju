#![cfg(not(target_arch = "wasm32"))]

use super::{
    GitOutput, MERGE_LOCK_STALE_AFTER_MS, TaskWorktreePoolService, git_error_text,
    is_managed_slot_path, normalize_merge_target, normalize_output, now_epoch_ms,
    parse_worktree_list, sanitize_branch_token, slot_id_from_path,
};
use crate::app::agent::project;
use std::path::{Path, PathBuf};
use std::process::Command;
use vw_api_types::id::TaskId;
use vw_api_types::task::TaskPoolWorktreeSlotStateDto;

#[test]
fn worktree_pool_parsers_normalize_git_output_and_porcelain_entries() {
    assert_eq!(normalize_merge_target("  release/v1  "), "release/v1");
    assert_eq!(normalize_output(b"one\r\ntwo\rthree\xff"), "one\ntwo\nthree\u{fffd}");

    let output = GitOutput {
        success: false,
        stdout: " stdout detail \n".to_string(),
        stderr: " stderr detail \n".to_string(),
    };
    assert_eq!(git_error_text(&output, "fallback"), "stderr detail\nstdout detail");
    let empty = GitOutput { success: false, stdout: String::new(), stderr: String::new() };
    assert_eq!(git_error_text(&empty, "fallback"), "fallback");

    let entries = parse_worktree_list(
        "worktree /repo\nbranch refs/heads/main\n\n\
         worktree /pool/slot-1\nbranch vw/task/slot-1\n\n\
         worktree /detached\ndetached\n",
    );
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].path, PathBuf::from("/repo"));
    assert_eq!(entries[0].branch.as_deref(), Some("main"));
    assert_eq!(entries[1].branch.as_deref(), Some("vw/task/slot-1"));
    assert_eq!(entries[2].branch, None);
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
}

struct GitPoolFixture {
    _temp: tempfile::TempDir,
    workspace: PathBuf,
    repo: PathBuf,
    pool_root: PathBuf,
}

impl GitPoolFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let workspace = temp.path().join("workspace");
        let repo = temp.path().join("repo");
        let pool_root = temp.path().join("task-worktrees");
        std::fs::create_dir_all(&workspace).expect("workspace dir");
        std::fs::create_dir_all(&pool_root).expect("pool root");
        init_repo(&repo);
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
        run(Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch)
            .arg(path)
            .arg("main"));
        std::fs::write(path.join(format!("{branch_token}.txt")), branch_token)
            .expect("write worktree marker");
    }
}

fn init_repo(path: &Path) {
    run(Command::new("git").arg("init").arg("-b").arg("main").arg(path));
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com"));
    run(Command::new("git").arg("-C").arg(path).arg("config").arg("user.name").arg("Test User"));
    std::fs::write(path.join("README.md"), "demo").expect("write readme");
    run(Command::new("git").arg("-C").arg(path).arg("add").arg("README.md"));
    run(Command::new("git").arg("-C").arg(path).arg("commit").arg("-m").arg("init"));
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
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .expect("git branch");
    assert!(
        output.status.success(),
        "command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run(command: &mut Command) {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
