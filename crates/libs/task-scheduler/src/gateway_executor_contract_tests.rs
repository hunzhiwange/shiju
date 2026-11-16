//! 执行器领域契约与脚本流夹具；宿主用真实 GatewayExecutorHost 实现调用。
//! 仅 test-support 开启，保留原断言及真实上下文注入，不提供模拟权限宿主。
use std::{path::PathBuf, sync::{Arc, Mutex}, collections::VecDeque};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use vw_api_transport::common::TimestampMs;
use vw_api_chat::chat::GatewayChatStreamRequest;
use session_port::chat_stream::GatewayChatStreamCollected;
use crate::runner::{gateway_executor::*, local::*};
use crate::domain::{RunId, TaskId};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::store::{AgentSchedulerStore, runs::ClaimTaskRunRequest, tasks::NewAgentTask};

pub fn execution_request(
    agent: Option<&str>,
    acp_agent: Option<&str>,
) -> Result<LocalAgentExecutionRequest> {
    Ok(LocalAgentExecutionRequest {
        project_path: PathBuf::from("/repo"),
        task_id: TaskId::new("task-1")?,
        run_id: RunId::new("run-1")?,
        title: "Task one".to_string(),
        prompt: "Do the task".to_string(),
        model: Some("gpt-5".to_string()),
        agent: agent.map(str::to_string),
        acp_agent: acp_agent.map(str::to_string),
        reasoning_effort: None,
        workspace: LocalAgentWorkspace {
            slot_id: Some("slot-1".to_string()),
            path: "/repo/.vw/worktrees/slot-1".to_string(),
            branch: Some("codex/task-1".to_string()),
        },
    })
}

pub fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Gateway stream log task".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Assigned,
        spec_uri: "memory://spec".to_string(),
        meta_uri: "memory://meta".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({}),
        created_at_ms: TimestampMs(1),
    })
}

#[derive(Debug, Default)]
struct FakeChatStream {
    results: Mutex<VecDeque<Result<GatewayChatStreamCollected, String>>>,
    payloads: Mutex<VecDeque<Vec<Value>>>,
    calls: Mutex<Vec<(String, Option<String>, Vec<Value>)>>,
}

impl GatewayChatStreamClient for FakeChatStream {
    fn collect_with_observer<'a>(
        &'a self,
        workspace_path: String,
        body: GatewayChatStreamRequest,
        _cancellation: tokio_util::sync::CancellationToken,
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        self.calls.lock().expect("scripted stream mutex").push((
            workspace_path,
            body.session_id.as_ref().map(|session_id| session_id.as_ref().to_string()),
            body.messages,
        ));
        let payloads = self.payloads.lock().expect("scripted stream mutex").pop_front().unwrap_or_default();
        let result = pop_stream_result(&self.results, "collect");
        Box::pin(async move {
            let mut observe_payload = observe_payload;
            for payload in &payloads {
                observe_payload(payload);
            }
            result
        })
    }
}

fn pop_stream_result(
    queue: &Mutex<VecDeque<Result<GatewayChatStreamCollected, String>>>,
    operation: &str,
) -> Result<GatewayChatStreamCollected> {
    match queue.lock().expect("scripted stream mutex").pop_front() {
        Some(Ok(value)) => Ok(value),
        Some(Err(message)) => bail!(message),
        None => bail!("unexpected {operation} call"),
    }
}

pub fn gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project<H: GatewayExecutorHost>() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project_path = temp.path().join("unregistered-project");
    std::fs::create_dir_all(&project_path)?;
    let executor = GatewayLocalAgentExecutor::<H>::default();
    let task_id = TaskId::new("task-unregistered")?;
    let run_id = RunId::new("run-unregistered")?;
    let workspace = LocalAgentWorkspace {
        slot_id: Some("slot".to_string()),
        path: project_path.join("slot").to_string_lossy().to_string(),
        branch: Some("codex/task-unregistered".to_string()),
    };

    let lease_error = executor
        .lease_workspace(LocalAgentWorkspaceRequest {
            agent: None,
            project_path: project_path.clone(),
            task_id: task_id.clone(),
            run_id: run_id.clone(),
        })
        .expect_err("lease should fail before pool IO")
        .to_string();
    assert!(lease_error.contains("registered git project not found"));

    let release_error = executor
        .release_workspace(LocalAgentWorkspaceReleaseRequest {
            project_path: project_path.clone(),
            task_id: task_id.clone(),
            run_id: run_id.clone(),
            workspace: workspace.clone(),
        })
        .expect_err("release should fail before pool IO")
        .to_string();
    assert!(release_error.contains("registered git project not found"));

    let recycle_error = executor
        .recycle_workspace(LocalAgentWorkspaceRecycleRequest {
            project_path,
            task_id,
            run_id,
            workspace,
            taint_reason: "failed execution".to_string(),
        })
        .expect_err("recycle should fail before pool IO")
        .to_string();
    assert!(recycle_error.contains("registered git project not found"));
    Ok(())
}

pub fn gateway_executor_execute_collects_stream_output_and_appends_observed_logs<H: GatewayExecutorHost>() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project_path = temp.path().join("gateway-execute-project");
    let workspace_path = project_path.join(".vw/worktrees/slot-1");
    std::fs::create_dir_all(&workspace_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-execute")?;
    let run_id = RunId::new("run-execute")?;
    store.create_task(&project_path, finite_task(&task_id)?)?;
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                lease_acquired_at_ms: TimestampMs(100),
                lease_expires_at_ms: Some(TimestampMs(200)),
            },
        )?
        .expect("run claim succeeds for assigned task");

    let stream = Arc::new(FakeChatStream::default());
    stream.payloads.lock().expect("scripted stream mutex").push_back(vec![
        json!({"type": "chat.step_start", "step_index": 0, "model": "gpt-5"}),
        json!({"type": "chat.delta", "delta": "hello\nworld"}),
        json!({"type": "chat.done"}),
    ]);
    stream.results.lock().expect("scripted stream mutex").push_back(Ok(GatewayChatStreamCollected {
        output: "hello world".to_string(),
        message_id: Some("assistant-1".to_string()),
        parent_message_id: Some("user-1".to_string()),
    }));
    let executor = GatewayLocalAgentExecutor::<H>::new_with_chat_stream(store.clone(), stream.clone());
    let mut request = execution_request(Some("reviewer"), Some("codex"))?;
    request.project_path = project_path.clone();
    request.workspace.path = workspace_path.to_string_lossy().to_string();
    request.task_id = task_id.clone();
    request.run_id = run_id.clone();

    let output = executor.execute(request)?;

    assert_eq!(output.raw_output, "hello world");
    assert_eq!(stream.calls.lock().expect("scripted stream mutex").len(), 1);
    let call = stream.calls.lock().expect("scripted stream mutex")[0].clone();
    assert_eq!(call.0, workspace_path.to_string_lossy());
    assert_eq!(call.1.as_deref(), Some("agent-scheduler-task-execute-run-execute"));
    assert_eq!(call.2, vec![json!({"role": "user", "content": "Do the task"})]);
    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert_eq!(logs.len(), 4);
    assert_eq!(logs[0].message, "gateway stream step 0 started model=gpt-5");
    assert_eq!(logs[1].message, "gateway stream output: hello");
    assert_eq!(logs[2].message, "gateway stream output: world");
    assert_eq!(logs[3].message, "gateway stream completed");
    assert!(logs.iter().all(|log| log.run_id.as_ref() == Some(&run_id)));
    Ok(())
}

pub fn gateway_executor_execute_propagates_stream_collection_error<H: GatewayExecutorHost>() -> Result<()> {
    let store = AgentSchedulerStore::new();
    let stream = Arc::new(FakeChatStream::default());
    stream.payloads.lock().expect("scripted stream mutex").push_back(vec![json!({
        "type": "chat.delta",
        "delta": "will fail to append because project is missing"
    })]);
    stream.results.lock().expect("scripted stream mutex").push_back(Err("stream unavailable".to_string()));
    let executor = GatewayLocalAgentExecutor::<H>::new_with_chat_stream(store, stream);

    let error = executor
        .execute(execution_request(None, None)?)
        .expect_err("stream error should propagate");

    assert_eq!(error.to_string(), "stream unavailable");
    Ok(())
}

pub fn task_pool_recovery_gateway_passes_stop_to_stream<H: GatewayExecutorHost>() -> Result<()> {
    #[derive(Debug)]
    struct CancelStream;
    impl GatewayChatStreamClient for CancelStream {
        fn collect_with_observer<'a>(
            &'a self,
            _: String,
            _: GatewayChatStreamRequest,
            cancel: tokio_util::sync::CancellationToken,
            _: GatewayPayloadObserver<'a>,
        ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
            Box::pin(async move {
                tokio::time::timeout(std::time::Duration::from_secs(2), cancel.cancelled())
                    .await
                    .context("scheduler did not propagate cancellation")?;
                bail!("stream acknowledged cancellation and shutdown")
            })
        }
    }
    let project = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(project.path())?;
    let mut request = execution_request(None, None)?;
    request.project_path = project.path().to_path_buf();
    // A missing/cancelled run must stop the transport, and still await its result.
    let executor = GatewayLocalAgentExecutor::<H>::new_with_chat_stream(store, Arc::new(CancelStream));
    assert!(
        executor.execute(request).unwrap_err().to_string().contains("acknowledged cancellation")
    );
    Ok(())
}
