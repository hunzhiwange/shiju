//! Real storage coverage for handoff idempotency and stale execution fencing.
use crate::{
    domain::{AgentRunStatus, RunId, TaskId},
    runner::local::*,
    state_machine::AgentTaskStatus,
    store::AgentSchedulerStore,
    task_board::AgentTaskBoardService,
};
use anyhow::Result;
use serde_json::json;
use vw_api_agent_scheduler::AgentSchedulerMoveTaskPayloadDto;
use vw_api_transport::common::TimestampMs;

fn move_to(
    store: &AgentSchedulerStore,
    path: &std::path::Path,
    id: &TaskId,
    key: &str,
) -> Result<()> {
    AgentTaskBoardService::new(store)
        .move_task(
            path,
            id,
            AgentSchedulerMoveTaskPayloadDto {
                from_status_key: None,
                to_status_key: key.into(),
                to_status: None,
                order_no: None,
            },
            None,
            TimestampMs(100),
        )?
        .unwrap();
    Ok(())
}
fn setup(path: &std::path::Path) -> Result<(AgentSchedulerStore, TaskId)> {
    let store = AgentSchedulerStore::new();
    store.initialize_project(path)?;
    store.save_task_board_workflow(path, &crate::lifecycle_bridge_tests::workflow())?;
    let id = TaskId::new("handoff-task")?;
    let mut task = crate::local_runner_tests::finite_task(&id)?;
    task.status = AgentTaskStatus::Draft;
    task.metadata = json!({"task_board.status_key":"human", "assignee":"login-user"});
    store.create_task(path, task)?;
    Ok((store, id))
}
struct Executor {
    fail: bool,
    move_to_human: Option<AgentSchedulerStore>,
}
impl LocalAgentExecutor for Executor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            slot_id: None,
            path: request.project_path.to_string_lossy().into_owned(),
            branch: None,
        })
    }
    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        assert!(request.prompt.contains("Review the evidence"));
        assert_eq!(request.agent.as_deref(), Some("main"));
        if let Some(store) = &self.move_to_human {
            move_to(store, &request.project_path, &request.task_id, "human")?;
        }
        if self.fail {
            anyhow::bail!("execution failure");
        }
        Ok(LocalAgentExecutionOutput { raw_output: "finished".into() })
    }
}
#[test]
fn local_runner_handoff_once_and_results_keep_step() -> Result<()> {
    for fail in [false, true] {
        let dir = tempfile::tempdir()?;
        let (store, id) = setup(dir.path())?;
        move_to(&store, dir.path(), &id, "agent")?;
        let executor = Executor { fail, move_to_human: None };
        let runner = LocalAgentRunner::new(&store, &executor);
        let result = runner
            .run_once(
                dir.path(),
                LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(1000) },
            )?
            .unwrap();
        assert_eq!(
            result.run.status,
            if fail { AgentRunStatus::Failed } else { AgentRunStatus::Succeeded }
        );
        move_to(&store, dir.path(), &id, "agent")?;
        assert!(
            runner
                .start_once(
                    dir.path(),
                    LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(2000) }
                )?
                .is_none()
        );
        let task = store.load_task(dir.path(), &id)?.unwrap();
        assert_eq!(task.metadata["task_board.status_key"], "agent");
        assert_eq!(task.metadata["task_board.entry_revision"], 1);
        assert_eq!(store.load_latest_run_for_task(dir.path(), &id)?.unwrap().attempt, 1);
        move_to(&store, dir.path(), &id, "human")?;
        assert!(
            runner
                .start_once(
                    dir.path(),
                    LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(3000) }
                )?
                .is_none()
        );
    }
    Ok(())
}
#[test]
fn local_runner_late_result_cannot_overwrite_manual_step() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (store, id) = setup(dir.path())?;
    move_to(&store, dir.path(), &id, "agent")?;
    let executor = Executor { fail: false, move_to_human: Some(store.clone()) };
    let result = LocalAgentRunner::new(&store, &executor)
        .run_once(
            dir.path(),
            LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(1000) },
        )?
        .unwrap();
    assert_eq!(result.run.status, AgentRunStatus::Succeeded);
    let task = store.load_task(dir.path(), &id)?.unwrap();
    assert_eq!(task.metadata["task_board.status_key"], "human");
    assert_eq!(task.status, AgentTaskStatus::Draft);
    assert!(task.metadata.get("task_board.active_run_id").is_none());
    Ok(())
}
#[test]
fn local_runner_explicit_step_action_is_fenced_and_uses_configured_target() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (store, id) = setup(dir.path())?;
    move_to(&store, dir.path(), &id, "agent")?;
    // Claim a real run so runtime ownership and project storage are exercised.
    let run_id = RunId::new("explicit-action")?;
    store
        .claim_ready_task_run(
            dir.path(),
            crate::store::runs::ClaimReadyTaskRunRequest {
                run_id: run_id.clone(),
                task_id: id.clone(),
                lease_acquired_at_ms: TimestampMs(1000),
                lease_expires_at_ms: Some(TimestampMs(2000)),
            },
        )?
        .unwrap();
    store.update_task_for_run(
        dir.path(),
        &id,
        &run_id,
        Some(AgentTaskStatus::Running),
        Default::default(),
        TimestampMs(1001),
    )?;
    let task = AgentTaskBoardService::new(&store).move_running_task(
        dir.path(),
        &id,
        &run_id,
        true,
        TimestampMs(1002),
        |_| Ok(()),
    )?;
    assert_eq!(task.metadata["task_board.status_key"], "human");
    move_to(&store, dir.path(), &id, "agent")?;
    assert!(
        AgentTaskBoardService::new(&store)
            .move_running_task(dir.path(), &id, &run_id, false, TimestampMs(1003), |_| Ok(()))
            .is_err()
    );
    Ok(())
}

#[test]
fn workflow_metadata_removal_cannot_bypass_step_ownership() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (store, id) = setup(dir.path())?;
    move_to(&store, dir.path(), &id, "agent")?;
    let before = store.load_task(dir.path(), &id)?.unwrap();
    for key in ["task_board.status_key", "task_board.entry_revision", "task_board.active_run_id", "agent", "acp_agent", "assignee"] {
        let patch = vw_api_agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
            metadata_remove_keys: vec![key.into()], ..Default::default()
        };
        assert!(AgentTaskBoardService::new(&store).update_metadata(dir.path(), &id, patch, None, TimestampMs(200)).is_err());
        assert_eq!(store.load_task(dir.path(), &id)?.unwrap().metadata, before.metadata);
    }
    move_to(&store, dir.path(), &id, "human")?;
    let human = store.load_task(dir.path(), &id)?.unwrap();
    assert!(human.metadata.get("task_board.step_instructions").is_none());
    Ok(())
}
