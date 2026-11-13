use super::workspace_claim::WorkspaceClaim;
use anyhow::Result;

#[test]
fn task_pool_claim_same_directory_serializes_runs() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let mut claim = WorkspaceClaim::acquire(root.path(), project.path(), "one".into())?.unwrap();
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "two".into())?.is_none());
    claim.release("one")?;
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "two".into())?.is_some());
    Ok(())
}

#[test]
fn task_pool_claim_different_directories_can_overlap() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let a = project.path().join("a");
    let b = project.path().join("b");
    std::fs::create_dir(&a)?;
    std::fs::create_dir(&b)?;
    let _first = WorkspaceClaim::acquire(root.path(), &a, "one".into())?.unwrap();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| -> Result<()> {
        let worker = scope.spawn(|| -> Result<()> {
            let _second = WorkspaceClaim::acquire(root.path(), &b, "two".into())?.unwrap();
            barrier.wait();
            barrier.wait();
            Ok(())
        });
        barrier.wait();
        assert!(WorkspaceClaim::acquire(root.path(), &a, "other".into())?.is_none());
        barrier.wait();
        worker.join().unwrap()?;
        Ok(())
    })?;
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "parent".into())?.is_none());
    Ok(())
}

#[test]
fn task_pool_claim_canonical_aliases_share_ownership() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let alias = project.path().join("alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(project.path(), &alias)?;
    #[cfg(not(unix))]
    let alias = project.path().join(".");
    let _claim = WorkspaceClaim::acquire(root.path(), project.path(), "one".into())?.unwrap();
    assert!(WorkspaceClaim::acquire(root.path(), &alias, "two".into())?.is_none());
    Ok(())
}

#[test]
fn task_pool_claim_foreign_release_is_rejected() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let mut claim = WorkspaceClaim::acquire(root.path(), project.path(), "one".into())?.unwrap();
    assert!(claim.release("foreign").is_err());
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "two".into())?.is_none());
    Ok(())
}

#[test]
fn task_pool_claim_cross_process_conflict() -> Result<()> {
    const CHILD: &str = "SHIJU_TASK4_LOCK_PROBE";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        assert!(
            WorkspaceClaim::acquire(&root.join("locks"), &root.join("project"), "child".into())?
                .is_none()
        );
        return Ok(());
    }
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("project"))?;
    let _claim = WorkspaceClaim::acquire(
        &root.path().join("locks"),
        &root.path().join("project"),
        "parent".into(),
    )?
    .unwrap();
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "agent_scheduler::runner::workspace_claim_tests::task_pool_claim_cross_process_conflict", "--nocapture"])
        .env(CHILD, root.path()).status()?;
    assert!(status.success());
    Ok(())
}

#[test]
fn task_pool_claim_runner_waits_without_creating_run() -> Result<()> {
    use super::local::*;
    use crate::agent_scheduler::{
        domain::TaskId,
        state_machine::{AgentTaskKind, AgentTaskStatus},
        store::{AgentSchedulerStore, tasks::NewAgentTask},
    };
    use vw_api_types::common::TimestampMs;
    struct Executor;
    impl LocalAgentExecutor for Executor {
        fn lease_workspace(&self, r: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
            Ok(LocalAgentWorkspace {
                slot_id: None,
                path: r.project_path.to_string_lossy().into_owned(),
                branch: None,
            })
        }
        fn execute(&self, _: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
            Ok(LocalAgentExecutionOutput { raw_output: "done".into() })
        }
    }
    let project = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(project.path())?;
    let first = TaskId::new("claim-first")?;
    let second = TaskId::new("claim-second")?;
    for id in [&first, &second] {
        store.create_task(
            project.path(),
            NewAgentTask {
                id: id.clone(),
                title: "directory claim".into(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "spec.md".into(),
                meta_uri: "meta.json".into(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: vec![],
                metadata: serde_json::json!({"prompt":"done"}),
                created_at_ms: TimestampMs(1000),
            },
        )?;
    }
    let runner = LocalAgentRunner::new(&store, &Executor);
    let request =
        |id: &TaskId| LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(2000) };
    let started = runner.start_once(project.path(), request(&first))?.unwrap();
    assert!(runner.start_once(project.path(), request(&second))?.is_none());
    assert!(runner.start_once(project.path(), request(&first))?.is_none());
    assert_eq!(store.load_task(project.path(), &second)?.unwrap().status, AgentTaskStatus::Queued);
    assert!(
        store
            .load_run(
                project.path(),
                &crate::agent_scheduler::domain::RunId::new("local-run-claim-second-2000")?
            )?
            .is_none()
    );
    runner.finish_started_run(project.path(), &first, &started.run.id, request(&first))?.unwrap();
    assert!(runner.run_once(project.path(), request(&second))?.is_some());
    Ok(())
}

fn recovery_fixture() -> Result<(
    tempfile::TempDir,
    crate::agent_scheduler::store::AgentSchedulerStore,
    crate::agent_scheduler::domain::TaskId,
)> {
    use crate::agent_scheduler::{
        domain::TaskId,
        state_machine::{AgentTaskKind, AgentTaskStatus},
        store::{AgentSchedulerStore, tasks::NewAgentTask},
    };
    let project = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(project.path())?;
    let id = TaskId::new("recovery-task")?;
    store.create_task(
        project.path(),
        NewAgentTask {
            id: id.clone(),
            title: "recovery".into(),
            kind: AgentTaskKind::Finite,
            status: AgentTaskStatus::Queued,
            spec_uri: "spec.md".into(),
            meta_uri: "meta.json".into(),
            assigned_to: None,
            team_id: None,
            room_id: None,
            project_room_id: None,
            delegated_to_team: None,
            schedule: None,
            timezone: None,
            next_scheduled_at: None,
            last_executed_at: None,
            dag: vec![],
            metadata: serde_json::json!({"prompt":"test"}),
            created_at_ms: vw_api_types::common::TimestampMs(1000),
        },
    )?;
    Ok((project, store, id))
}

struct RecoveryExecutor<F>(F);
impl<F> super::local::LocalAgentExecutor for RecoveryExecutor<F>
where
    F: Fn() -> Result<super::local::LocalAgentExecutionOutput>,
{
    fn lease_workspace(
        &self,
        r: super::local::LocalAgentWorkspaceRequest,
    ) -> Result<super::local::LocalAgentWorkspace> {
        Ok(super::local::LocalAgentWorkspace {
            slot_id: None,
            path: r.project_path.to_string_lossy().into_owned(),
            branch: None,
        })
    }
    fn execute(
        &self,
        _: super::local::LocalAgentExecutionRequest,
    ) -> Result<super::local::LocalAgentExecutionOutput> {
        (self.0)()
    }
}

#[test]
fn task_pool_recovery_cancel_waits_for_execution_shutdown() -> Result<()> {
    recovery_waits_for_writer(false)
}

fn recovery_waits_for_writer(expire: bool) -> Result<()> {
    use super::local::*;
    use crate::agent_scheduler::state_machine::AgentTaskStatus;
    use vw_api_types::common::TimestampMs;
    let (project, store, id) = recovery_fixture()?;
    let entered = std::sync::Barrier::new(2);
    let stopped = std::sync::Barrier::new(2);
    let executor = RecoveryExecutor(|| {
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "agent_scheduler::runner::workspace_claim_tests::workspace_claim_writer_process_fixture"])
            .env("SHIJU_TASK5_WRITER_CHILD", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()?;
        entered.wait();
        stopped.wait();
        child.kill()?;
        child.wait()?;
        Ok(LocalAgentExecutionOutput { raw_output: "late result".into() })
    });
    let runner = LocalAgentRunner::new(&store, &executor);
    let request = LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(2000) };
    let started = runner.start_once(project.path(), request.clone())?.unwrap();
    std::thread::scope(|scope| -> Result<()> {
        let worker = scope
            .spawn(|| runner.finish_started_run(project.path(), &id, &started.run.id, request));
        entered.wait();
        let expected = if expire {
            use crate::agent_scheduler::engine::recovery::{RecoveryEngine, RecoveryPolicy};
            let recovery = RecoveryEngine::with_policy(
                &store,
                RecoveryPolicy { stale_run_grace_ms: 1, ..RecoveryPolicy::default() },
            );
            tokio::runtime::Runtime::new()?.block_on(async {
                recovery.reconcile_once(project.path(), TimestampMs(2_000_000)).await?;
                recovery.reconcile_once(project.path(), TimestampMs(2_000_002)).await?;
                anyhow::Ok(())
            })?;
            AgentTaskStatus::Failed
        } else {
            store.transition_task_status(
                project.path(),
                &id,
                AgentTaskStatus::Cancelled,
                TimestampMs(3000),
            )?;
            AgentTaskStatus::Cancelled
        };
        // The production namespace must remain locked until execute returns.
        let root =
            directories::BaseDirs::new().unwrap().data_local_dir().join("shiju/workspace-claims");
        assert!(WorkspaceClaim::acquire(&root, project.path(), "second".into())?.is_none());
        stopped.wait();
        worker.join().unwrap()?;
        assert_eq!(store.load_task(project.path(), &id)?.unwrap().status, expected);
        assert!(WorkspaceClaim::acquire(&root, project.path(), "second".into())?.is_some());
        Ok(())
    })?;
    Ok(())
}

#[test]
fn task_pool_recovery_expiry_does_not_steal_live_workspace() -> Result<()> {
    recovery_waits_for_writer(true)
}

#[test]
fn task_pool_recovery_old_run_cannot_release_new_owner() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let mut old = WorkspaceClaim::acquire(root.path(), project.path(), "old".into())?.unwrap();
    old.release("old")?;
    let new = WorkspaceClaim::acquire(root.path(), project.path(), "new".into())?.unwrap();
    new.retain(project.path())?;
    assert!(WorkspaceClaim::take(project.path(), "old")?.is_none());
    old.release("old")?;
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "other".into())?.is_none());
    WorkspaceClaim::take(project.path(), "new")?.unwrap().release("new")?;
    assert!(WorkspaceClaim::acquire(root.path(), project.path(), "other".into())?.is_some());
    Ok(())
}

#[test]
fn task_pool_recovery_failure_preserves_user_directory() -> Result<()> {
    use super::local::*;
    use vw_api_types::common::TimestampMs;
    let (project, store, id) = recovery_fixture()?;
    let user_file = project.path().join("uncommitted.txt");
    std::fs::write(&user_file, "user content")?;
    let executor = RecoveryExecutor(|| anyhow::bail!("execution failed"));
    let runner = LocalAgentRunner::new(&store, &executor);
    runner.run_once(
        project.path(),
        LocalAgentRunnerRequest { task_id: id, now_ms: TimestampMs(2000) },
    )?;
    assert_eq!(std::fs::read_to_string(user_file)?, "user content");
    let root =
        directories::BaseDirs::new().unwrap().data_local_dir().join("shiju/workspace-claims");
    assert!(WorkspaceClaim::acquire(&root, project.path(), "second".into())?.is_some());
    Ok(())
}

#[test]
fn task_pool_recovery_unconfirmed_shutdown_cannot_be_retried() -> Result<()> {
    use super::local::*;
    use vw_api_types::common::TimestampMs;
    let (project, store, id) = recovery_fixture()?;
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let executor = RecoveryExecutor(|| {
        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(ExecutionShutdownUnconfirmed("test writer did not acknowledge stop".into()).into())
    });
    let runner = LocalAgentRunner::new(&store, &executor);
    let request = LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(2000) };
    let started = runner.start_once(project.path(), request.clone())?.unwrap();
    assert!(
        runner.finish_started_run(project.path(), &id, &started.run.id, request.clone()).is_err()
    );
    assert!(runner.finish_started_run(project.path(), &id, &started.run.id, request)?.is_none());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(
        WorkspaceClaim::acquire(
            &super::workspace_claim::lock_root()?,
            project.path(),
            "second".into()
        )?
        .is_none()
    );
    // This fixture has no real writer; remove only its own durable test record.
    use sha2::{Digest, Sha256};
    let canonical = project.path().canonicalize()?;
    let key = hex::encode(Sha256::digest(canonical.as_os_str().as_encoded_bytes()));
    std::fs::remove_file(super::workspace_claim::lock_root()?.join(format!("active-{key}")))?;
    Ok(())
}

#[test]
fn workspace_claim_writer_process_fixture() {
    if std::env::var_os("SHIJU_TASK5_WRITER_CHILD").is_some() {
        let _ = std::io::read_to_string(std::io::stdin()).unwrap();
    }
}

#[test]
fn task_pool_recovery_restart_retains_unconfirmed_writer() -> Result<()> {
    const CHILD: &str = "SHIJU_TASK5_CRASH_ROOT";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = std::path::PathBuf::from(root);
        WorkspaceClaim::acquire(&root.join("locks"), &root.join("project"), "crashed-run".into())?
            .unwrap()
            .retain(&root.join("project"))?;
        // Simulate process death: destructors must not remove the ownership record.
        std::process::exit(0);
    }
    let root = tempfile::tempdir()?;
    let project = root.path().join("project");
    let child = project.join("child");
    std::fs::create_dir_all(&child)?;
    let sibling = root.path().join("sibling");
    std::fs::create_dir(&sibling)?;
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "agent_scheduler::runner::workspace_claim_tests::task_pool_recovery_restart_retains_unconfirmed_writer"])
        .env(CHILD, root.path()).status()?;
    assert!(status.success());
    for path in [&project, &child, &root.path().to_path_buf()] {
        let error =
            WorkspaceClaim::acquire(&root.path().join("locks"), path, "new".into()).err().unwrap();
        assert!(error.to_string().contains("crashed-run"));
    }
    assert!(
        WorkspaceClaim::acquire(&root.path().join("locks"), &sibling, "sibling".into())?.is_some()
    );
    Ok(())
}

#[test]
fn task_pool_worktree_removed_directory_releases_recorded_claim() -> Result<()> {
    let locks = tempfile::tempdir()?;
    let checkout = tempfile::tempdir()?;
    let canonical = checkout.path().canonicalize()?;
    WorkspaceClaim::acquire(locks.path(), &canonical, "removed-run".into())?
        .expect("claim")
        .retain(&canonical)?;
    checkout.close()?;
    WorkspaceClaim::take_canonical(&canonical, "removed-run")?
        .expect("retained claim")
        .release("removed-run")?;
    assert!(
        !std::fs::read_dir(locks.path())?.any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("active-"))
    );
    Ok(())
}
