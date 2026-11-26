//! 保存、项目应用、自动执行与显式步骤操作共用真实存储。
use super::*;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use task_scheduler::domain::AgentRunStatus;
use task_scheduler::runner::worktree_owner::SchedulerWorktreeOwner;
use tool_context::run_worktree::RunWorktreePort;
use vw_api_agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerMoveTaskPayloadDto, AgentSchedulerSelectWorkflowRequest,
    AgentSchedulerTaskMetadataPatchDto,
};
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusCategoryDto};
use vw_api_task_lifecycle::{TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody};
use vw_api_transport::TimestampMs;

struct StepExecutor {
    store: AgentSchedulerStore,
    started: tokio::sync::mpsc::UnboundedSender<LocalAgentExecutionRequest>,
    actions: Mutex<mpsc::Receiver<Option<bool>>>,
}
impl LocalAgentExecutor for StepExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            slot_id: None,
            branch: None,
            path: request.project_path.display().to_string(),
        })
    }
    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        self.started.send(request.clone())?;
        if let Some(return_step) =
            self.actions.lock().unwrap().recv_timeout(Duration::from_secs(10))?
        {
            owner(&self.store, &request).move_workflow_step(return_step)?;
        }
        Ok(LocalAgentExecutionOutput { raw_output: "execution result".into() })
    }
}
fn owner(
    store: &AgentSchedulerStore,
    request: &LocalAgentExecutionRequest,
) -> SchedulerWorktreeOwner {
    SchedulerWorktreeOwner {
        store: store.clone(),
        project_path: request.project_path.clone(),
        context: request.scheduler_context.clone(),
        agent_exists: |agent| Ok(agent == "main"),
        task_id: request.task_id.clone(),
        run_id: request.run_id.clone(),
        claims: Arc::new(Mutex::new(Vec::new())),
    }
}
async fn move_step(
    gateway: &StoreBackedAgentSchedulerGateway,
    context: &AgentSchedulerContext,
    key: &str,
) -> Result<()> {
    gateway
        .command(
            "project".into(),
            context.clone(),
            AgentSchedulerCommandRequest {
                task_id: "handoff".into(),
                command: AgentSchedulerCommandKindDto::Move,
                max_concurrent: None,
                reason: None,
                payload: AgentSchedulerCommandPayloadDto {
                    move_task: Some(AgentSchedulerMoveTaskPayloadDto {
                        from_status_key: None,
                        to_status_key: key.into(),
                        to_status: None,
                        order_no: Some(4),
                    }),
                    ..Default::default()
                },
            },
        )
        .await?;
    Ok(())
}
async fn finished(
    gateway: &StoreBackedAgentSchedulerGateway,
    context: &AgentSchedulerContext,
    run: &RunId,
) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if gateway
                .store
                .load_run(context, run)?
                .is_some_and(|run| run.status == AgentRunStatus::Succeeded)
            {
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    Ok(())
}
async fn started(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<LocalAgentExecutionRequest>,
) -> Result<LocalAgentExecutionRequest> {
    Ok(tokio::time::timeout(Duration::from_secs(10), receiver.recv())
        .await?
        .expect("execution started"))
}

#[tokio::test]
async fn workflow_handoff_saved_workflow_runs_returns_reenters_and_completes() -> Result<()> {
    for terminal_category in ["done", "closed"] {
        let directory = tempfile::tempdir()?;
        let context = owned_scheduler_context(directory.path())?;
        let store = AgentSchedulerStore::new();
        let database = agent_database::GlobalDatabase::new(store.database_path())?;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let (actions, commands) = mpsc::channel();
        let mut gateway = StoreBackedAgentSchedulerGateway::with_executor(StepExecutor {
            store: store.clone(),
            started: tx,
            actions: Mutex::new(commands),
        });
        gateway.store = store;
        let tenant = context.scope().project_scope().tenant_scope().clone();
        let login_user = context.scope().project_scope().user_id().as_ref();
        let workflow: TaskLifecycleWorkflowDto = serde_json::from_value(json!({
            "id":format!("handoff-flow-{}", uuid::Uuid::new_v4()), "label":"Handoff", "initial_status_key":"human", "built_in":false,
            "statuses":[{"key":"human","label":"Human","category":"unstarted"},
                {"key":"agent","label":"Agent","category":"started"},
                {"key":"done","label":"Done","category":terminal_category}],
            "steps":[{"status_key":"human","handoff":{"kind":"member","member_id":login_user}},
                {"status_key":"agent","handoff":{"kind":"agent","agent_id":"main","instructions":"Inspect evidence", "return_status_key":"human", "completion_status_key":"done"}},
                {"status_key":"done","handoff":{"kind":"keep_assignee"}}]
        }))?;
        let saved = task_lifecycle::store::create_workflow(
            database,
            tenant,
            TaskLifecycleWorkflowUpsertBody { workflow },
        )
        .await?;
        let selected = gateway
            .select_task_board_workflow(
                "project".into(),
                context.clone(),
                AgentSchedulerSelectWorkflowRequest {
                    workflow_id: saved.id.clone(),
                    expected_workflow: None,
                },
            )
            .await?
            .workflow
            .unwrap();
        assert_eq!(selected, TaskPoolWorkflowSnapshotDto::from(saved));
        let created = gateway
            .create_task_scoped(context.clone(), super::cases::create_task_request("handoff"))
            .await?;
        assert_eq!(created.task.status, AgentTaskStatusDto::Draft);
        assert_eq!(created.task.metadata["assignee"], login_user);

        move_step(&gateway, &context, "agent").await?;
        let first = started(&mut rx).await?;
        assert_eq!(first.agent.as_deref(), Some("main"));
        assert!(first.prompt.contains("Inspect evidence"));
        // Sorting/repeated moves must not create another entry or consume ownership.
        move_step(&gateway, &context, "agent").await?;
        let before = gateway.store.load_task(&context, &first.task_id)?.unwrap();
        assert_eq!(before.metadata["task_board.entry_revision"], 2);
        actions.send(None)?;
        finished(&gateway, &context, &first.run_id).await?;
        let snapshot = gateway.task_board_status("project".into(), context.clone()).await?;
        assert_eq!(snapshot.tasks[0].status_key, "agent");
        assert_eq!(snapshot.tasks[0].status, Some(AgentTaskStatusDto::Completed));
        assert_eq!(
            snapshot.tasks[0].read_model.status_category,
            Some(TaskPoolWorkflowStatusCategoryDto::Started)
        );
        assert!(owner(&gateway.store, &first).move_workflow_step(false).is_err());

        move_step(&gateway, &context, "human").await?;
        move_step(&gateway, &context, "agent").await?;
        let second = started(&mut rx).await?;
        assert_ne!(first.run_id, second.run_id);
        actions.send(Some(true))?;
        finished(&gateway, &context, &second.run_id).await?;
        let returned = gateway.task_board_status("project".into(), context.clone()).await?;
        assert_eq!(returned.tasks[0].status_key, "human");
        assert_eq!(returned.tasks[0].read_model.assignee.as_deref(), Some(login_user));
        assert_eq!(returned.tasks[0].status, Some(AgentTaskStatusDto::Draft));

        move_step(&gateway, &context, "agent").await?;
        let third = started(&mut rx).await?;
        assert!(owner(&gateway.store, &second).move_workflow_step(false).is_err());
        actions.send(Some(false))?;
        finished(&gateway, &context, &third.run_id).await?;
        let completed = gateway.task_board_status("project".into(), context.clone()).await?;
        assert_eq!(completed.tasks[0].status_key, "done");
        assert_eq!(
            completed.tasks[0].read_model.status_category,
            Some(if terminal_category == "done" {
                TaskPoolWorkflowStatusCategoryDto::Done
            } else {
                TaskPoolWorkflowStatusCategoryDto::Closed
            })
        );
        assert_eq!(
            completed.tasks[0].status,
            Some(if terminal_category == "done" {
                AgentTaskStatusDto::Completed
            } else {
                AgentTaskStatusDto::Cancelled
            })
        );
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}

#[tokio::test]
async fn workflow_handoff_manual_takeover_rejects_old_run_and_isolates_scopes() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let context = owned_scheduler_context(directory.path())?;
    let store = AgentSchedulerStore::new();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (actions, commands) = mpsc::channel();
    let mut gateway = StoreBackedAgentSchedulerGateway::with_executor(StepExecutor {
        store: store.clone(),
        started: tx,
        actions: Mutex::new(commands),
    });
    gateway.store = store;
    let workflow: TaskPoolWorkflowSnapshotDto = serde_json::from_value(json!({
        "id":"takeover", "label":"Takeover", "initial_status_key":"human",
        "statuses":[{"key":"human","label":"Human","category":"unstarted"}, {"key":"agent","label":"Agent","category":"started"}],
        "steps":[{"status_key":"human","handoff":{"kind":"member","member_id":context.scope().project_scope().user_id().as_ref()}},
            {"status_key":"agent","handoff":{"kind":"agent","agent_id":"main","completion_status_key":"human"}}]
    }))?;
    gateway.store.save_task_board_workflow(&context, &workflow)?;
    gateway
        .create_task_scoped(context.clone(), super::cases::create_task_request("handoff"))
        .await?;
    move_step(&gateway, &context, "agent").await?;
    let running = started(&mut rx).await?;
    move_step(&gateway, &context, "human").await?;
    assert!(owner(&gateway.store, &running).move_workflow_step(false).is_err());
    actions.send(None)?;
    finished(&gateway, &context, &running.run_id).await?;
    let snapshot = gateway.task_board_status("project".into(), context.clone()).await?;
    assert_eq!(snapshot.tasks[0].status_key, "human");
    assert_eq!(snapshot.tasks[0].status, Some(AgentTaskStatusDto::Draft));
    for scope in [
        ProjectScope::new(
            context.scope().project_scope().tenant_scope().clone(),
            "another-project".into(),
        )?,
        ProjectScope::new(
            TenantScope::new("other-platform", "other-company")?,
            context.scope().project_scope().project_id().clone(),
        )?,
    ] {
        let other = AgentSchedulerContext::new(
            vw_api_agent_scheduler::AgentSchedulerScope::new(scope),
            directory.path(),
        )?;
        assert!(gateway.store.load_task(&other, &running.task_id)?.is_none());
        assert!(gateway.store.load_run(&other, &running.run_id)?.is_none());
        assert!(gateway.store.load_task_board_workflow(&other)?.is_none());
        assert!(move_step(&gateway, &other, "human").await.is_err());
    }
    // Generic metadata edits cannot remove the fence or execution routing.
    for key in ["task_board.active_run_id", "agent", "assignee"] {
        assert!(
            task_scheduler::task_board::AgentTaskBoardService::new(&gateway.store)
                .update_metadata(
                    &context,
                    &running.task_id,
                    AgentSchedulerTaskMetadataPatchDto {
                        metadata_remove_keys: vec![key.into()],
                        ..Default::default()
                    },
                    None,
                    TimestampMs(200),
                )
                .is_err()
        );
    }
    Ok(())
}
