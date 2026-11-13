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
