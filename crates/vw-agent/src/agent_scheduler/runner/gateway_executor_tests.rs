//! 测试网关执行器的请求转换、流事件处理、取消与结果归档。
//! 脚本化事件流覆盖完成、失败与取消，确保最终归档不依赖用户可见文本解析。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use parking_lot::Mutex;
use serde_json::json;
use std::collections::VecDeque;
use vw_api_types::task::{
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto,
};

use super::*;
use crate::agent_scheduler::domain::{RunId, TaskId, AssigneeId};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;
use crate::agent_scheduler::store::tasks::NewAgentTask;
use crate::app::agent::gateway::api::handlers::session::stream::GatewayChatStreamCollected;

#[test]
fn gateway_project_path_normalization_handles_platform_separators_and_edges() {
    assert_eq!(normalize_project_path(Path::new("/repo/project/")), "repo/project");
    assert_eq!(normalize_project_path(Path::new("./repo/project")), "repo/project");
    assert_eq!(normalize_project_path(Path::new(r"\repo\project\")), "repo/project");
}

#[test]
fn gateway_delta_logs_trim_limit_and_truncate_output() {
    let long_line = "界".repeat(GATEWAY_STREAM_DELTA_LOG_LIMIT + 5);
    let delta = format!("  first  \n\n{long_line}\nthird\nfourth");

    let messages = gateway_delta_log_messages(&delta);

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "gateway stream output: first");
    assert!(messages[1].ends_with("..."));
    assert_eq!(messages[1].chars().count(), "gateway stream output: ".chars().count() + 280);
    assert_eq!(messages[2], "gateway stream output: third");
}

#[test]
fn gateway_stream_payload_logs_cover_lifecycle_and_error_events() {
    assert_eq!(
        gateway_stream_payload_log_messages(&json!({
            "type": "chat.step_start",
            "step_index": 2,
            "model": "  gpt-5  "
        })),
        vec!["gateway stream step 2 started model=gpt-5"]
    );
    assert_eq!(
        gateway_stream_payload_log_messages(&json!({
            "type": "chat.step_finish",
            "finish_reason": "   "
        })),
        vec!["gateway stream step ? finished"]
    );
    assert_eq!(
        gateway_stream_payload_log_messages(&json!({
            "type": "chat.post_tool_round",
            "step_index": -1
        })),
        vec!["gateway stream step -1 completed tool round"]
    );
    assert_eq!(
        gateway_stream_payload_log_messages(&json!({
            "type": "chat.error",
            "error": "  gateway unavailable  "
        })),
        vec!["gateway stream error: gateway unavailable"]
    );
    assert_eq!(
        gateway_stream_payload_log_messages(&json!({"type": "chat.done"})),
        vec!["gateway stream completed"]
    );
    assert!(gateway_stream_payload_log_messages(&json!({"type": "unknown"})).is_empty());
    assert!(
        gateway_stream_payload_log_messages(&json!({"type": "chat.error", "error": " "}))
            .is_empty()
    );
}

#[test]
fn gateway_stream_payload_logs_ignore_empty_delta_and_truncate_step_fields() {
    assert!(gateway_stream_payload_log_messages(&json!({"type": "chat.delta"})).is_empty());
    assert!(
        gateway_stream_payload_log_messages(&json!({
            "type": "chat.delta",
            "delta": "   \n\t"
        }))
        .is_empty()
    );

    let long_model = "m".repeat(GATEWAY_STREAM_FIELD_LOG_LIMIT + 5);
    let start = gateway_stream_payload_log_messages(&json!({
        "type": "chat.step_start",
        "step_index": 4,
        "model": long_model,
    }));
    assert_eq!(start.len(), 1);
    assert!(start[0].starts_with("gateway stream step 4 started model="));
    assert!(start[0].ends_with("..."));

    let long_reason = "reason".repeat(GATEWAY_STREAM_FIELD_LOG_LIMIT);
    let finish = gateway_stream_payload_log_messages(&json!({
        "type": "chat.step_finish",
        "step_index": 5,
        "finish_reason": long_reason,
    }));
    assert_eq!(finish.len(), 1);
    assert!(finish[0].starts_with("gateway stream step 5 finished finish_reason="));
    assert!(finish[0].ends_with("..."));
}

#[test]
fn gateway_stream_request_adds_acp_session_isolation_options() -> Result<()> {
    let request = execution_request(Some("reviewer"), Some("codex"))?;

    let stream = gateway_local_agent_stream_request(&request);
    let options = stream.options.expect("gateway options");

    assert_eq!(
        stream.session_id.as_ref().map(|session_id| session_id.as_ref()),
        Some("agent-scheduler-task-1-run-1")
    );
    assert_eq!(stream.messages, vec![json!({"role": "user", "content": "Do the task"})]);
    assert_eq!(options["cwd"], "/repo/.vw/worktrees/slot-1");
    assert_eq!(options["full_access"], true);
    assert_eq!(options["agent"], "reviewer");
    assert_eq!(options["acp_agent"], "codex");
    assert_eq!(options["acp_force_new_session"], true);
    assert_eq!(options["acp_close_client_after_request"], true);
    assert_eq!(options["acp_history_strategy"], "discard");
    assert_eq!(options["acp_history_recent_count"], 1);
    assert_eq!(options["acp_permission_mode"], "approve-all");
    Ok(())
}

#[test]
fn gateway_stream_request_without_agent_or_acp_keeps_minimal_options() -> Result<()> {
    let request = execution_request(None, None)?;

    let stream = gateway_local_agent_stream_request(&request);
    let options = stream.options.expect("gateway options");
    let options = options.as_object().expect("options object");

    assert_eq!(stream.agent, None);
    assert_eq!(stream.acp_agent, None);
    assert_eq!(stream.reasoning_effort, None);
    assert_eq!(options.len(), 2);
    assert_eq!(options["cwd"], "/repo/.vw/worktrees/slot-1");
    assert_eq!(options["full_access"], true);
    Ok(())
}

#[test]
fn gateway_stream_payload_task_log_appends_each_message_with_run_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project_path = temp.path().join("gateway-stream-log-project");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path)?;
    let task_id = TaskId::new("task-stream-log")?;
    let run_id = RunId::new("run-stream-log")?;
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

    append_gateway_stream_payload_task_log(
        &store,
        &project_path,
        &task_id,
        &run_id,
        TimestampMs(123),
        &json!({
            "type": "chat.delta",
            "delta": " first line \n second line \n third line \n fourth line "
        }),
    )?;

    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert_eq!(logs.len(), 3);
    assert!(logs.iter().all(|log| log.run_id.as_ref() == Some(&run_id)));
    assert!(logs.iter().all(|log| log.timestamp_ms == TimestampMs(123)));
    assert_eq!(logs[0].message, "gateway stream output: first line");
    assert_eq!(logs[1].message, "gateway stream output: second line");
    assert_eq!(logs[2].message, "gateway stream output: third line");
    Ok(())
}

#[test]
fn gateway_stream_payload_task_log_noops_when_payload_has_no_messages() -> Result<()> {
    let store = AgentSchedulerStore::new();
    append_gateway_stream_payload_task_log(
        &store,
        Path::new("/missing"),
        &TaskId::new("task-noop")?,
        &RunId::new("run-noop")?,
        TimestampMs(1),
        &json!({"type": "chat.error", "error": "   "}),
    )?;
    Ok(())
}

#[test]
fn gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let project_path = temp.path().join("unregistered-project");
    std::fs::create_dir_all(&project_path)?;
    let executor = GatewayLocalAgentExecutor::default();
    let task_id = TaskId::new("task-unregistered")?;
    let run_id = RunId::new("run-unregistered")?;
    let workspace = LocalAgentWorkspace {
        slot_id: "slot".to_string(),
        path: project_path.join("slot").to_string_lossy().to_string(),
        branch: "codex/task-unregistered".to_string(),
    };

    let lease_error = executor
        .lease_workspace(LocalAgentWorkspaceRequest {
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

#[test]
fn gateway_executor_execute_collects_stream_output_and_appends_observed_logs() -> Result<()> {
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
    stream.payloads.lock().push_back(vec![
        json!({"type": "chat.step_start", "step_index": 0, "model": "gpt-5"}),
        json!({"type": "chat.delta", "delta": "hello\nworld"}),
        json!({"type": "chat.done"}),
    ]);
    stream.results.lock().push_back(Ok(GatewayChatStreamCollected {
        output: "hello world".to_string(),
        message_id: Some("assistant-1".to_string()),
        parent_message_id: Some("user-1".to_string()),
    }));
    let executor = GatewayLocalAgentExecutor::new_with_chat_stream(store.clone(), stream.clone());
    let mut request = execution_request(Some("reviewer"), Some("codex"))?;
    request.project_path = project_path.clone();
    request.workspace.path = workspace_path.to_string_lossy().to_string();
    request.task_id = task_id.clone();
    request.run_id = run_id.clone();

    let output = executor.execute(request)?;

    assert_eq!(output.raw_output, "hello world");
    assert_eq!(stream.calls.lock().len(), 1);
    let call = stream.calls.lock()[0].clone();
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

#[test]
fn gateway_executor_execute_propagates_stream_collection_error() -> Result<()> {
    let store = AgentSchedulerStore::new();
    let stream = Arc::new(FakeChatStream::default());
    stream.payloads.lock().push_back(vec![json!({
        "type": "chat.delta",
        "delta": "will fail to append because project is missing"
    })]);
    stream.results.lock().push_back(Err("stream unavailable".to_string()));
    let executor = GatewayLocalAgentExecutor::new_with_chat_stream(store, stream);

    let error = executor
        .execute(execution_request(None, None)?)
        .expect_err("stream error should propagate");

    assert_eq!(error.to_string(), "stream unavailable");
    Ok(())
}

#[test]
fn gateway_project_lookup_matches_registered_worktree_and_sandbox_paths() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let worktree = temp.path().join("registered-worktree");
    let sandbox = temp.path().join("registered-sandbox");
    std::fs::create_dir_all(&worktree)?;
    std::fs::create_dir_all(&sandbox)?;
    let mut info = project_info(worktree.to_string_lossy().as_ref());
    info.id = format!("project-gateway-{}", std::process::id());
    info.sandboxes = vec![sandbox.to_string_lossy().to_string()];
    project::store_local_test_project(&info)?;

    let found_worktree = {
        let worktree = worktree.clone();
        run_gateway_future("find registered worktree", async move {
            project_for_local_workspace(&worktree).await
        })?
    };
    let found_sandbox = {
        let sandbox = sandbox.clone();
        run_gateway_future("find registered sandbox", async move {
            project_for_local_workspace(&sandbox).await
        })?
    };

    assert_eq!(found_worktree.id, info.id);
    assert_eq!(found_sandbox.id, info.id);
    project::remove_local_test_project(&info.id)?;
    Ok(())
}

#[tokio::test]
async fn gateway_worktree_pool_inner_helpers_map_successful_pool_responses() -> Result<()> {
    let pool = FakeWorktreePool::default();
    pool.acquire_results.lock().push_back(Ok(TaskPoolWorktreeAcquireResponse {
        leased: true,
        slot: Some(TaskPoolWorktreeSlotDto {
            id: "slot-7".to_string(),
            path: "/repo/.vw/worktrees/slot-7".to_string(),
            branch: "codex/task-worktree".to_string(),
            state: TaskPoolWorktreeSlotStateDto::Busy,
            leased_task_id: None,
            taint_reason: None,
        }),
    }));
    pool.release_results.lock().push_back(Ok(TaskPoolWorktreeReleaseResponse { released: true }));
    pool.recycle_results.lock().push_back(Ok(TaskPoolWorktreeRecycleResponse { recycled: true }));
    let request = workspace_request("task-worktree")?;
    let release = release_request("task-worktree")?;
    let recycle = recycle_request("task-worktree", "needs clean checkout")?;
    let project = project_info("/repo");

    let workspace =
        lease_gateway_local_agent_workspace_for_project(request, &project, &pool).await?;
    release_gateway_local_agent_workspace_for_project(release, &project, &pool).await?;
    recycle_gateway_local_agent_workspace_for_project(recycle, &project, &pool).await?;

    assert_eq!(
        workspace,
        LocalAgentWorkspace {
            slot_id: "slot-7".to_string(),
            path: "/repo/.vw/worktrees/slot-7".to_string(),
            branch: "codex/task-worktree".to_string(),
        }
    );
    assert_eq!(
        pool.acquire_calls.lock().as_slice(),
        &[("repo".to_string(), "task-worktree".to_string())]
    );
    assert_eq!(
        pool.release_calls.lock().as_slice(),
        &[("repo".to_string(), "task-worktree".to_string())]
    );
    assert_eq!(
        pool.recycle_calls.lock().as_slice(),
        &[(
            "repo".to_string(),
            "task-worktree".to_string(),
            Some("needs clean checkout".to_string())
        )]
    );
    Ok(())
}

#[tokio::test]
async fn gateway_worktree_pool_inner_helpers_report_unusable_pool_responses() -> Result<()> {
    let project = project_info("/repo");
    let pool = FakeWorktreePool::default();
    pool.acquire_results
        .lock()
        .push_back(Ok(TaskPoolWorktreeAcquireResponse { leased: false, slot: None }));
    let lease_error = lease_gateway_local_agent_workspace_for_project(
        workspace_request("task-no-lease")?,
        &project,
        &pool,
    )
    .await
    .expect_err("not leased");
    assert!(lease_error.to_string().contains("did not lease"));

    pool.acquire_results
        .lock()
        .push_back(Ok(TaskPoolWorktreeAcquireResponse { leased: true, slot: None }));
    let missing_slot = lease_gateway_local_agent_workspace_for_project(
        workspace_request("task-no-slot")?,
        &project,
        &pool,
    )
    .await
    .expect_err("leased without slot");
    assert!(missing_slot.to_string().contains("leased without slot metadata"));

    pool.release_results.lock().push_back(Ok(TaskPoolWorktreeReleaseResponse { released: false }));
    let release_error = release_gateway_local_agent_workspace_for_project(
        release_request("task-release")?,
        &project,
        &pool,
    )
    .await
    .expect_err("not released");
    assert!(release_error.to_string().contains("did not release"));

    pool.recycle_results.lock().push_back(Ok(TaskPoolWorktreeRecycleResponse { recycled: false }));
    let recycle_error = recycle_gateway_local_agent_workspace_for_project(
        recycle_request("task-recycle", "bad")?,
        &project,
        &pool,
    )
    .await
    .expect_err("not recycled");
    assert!(recycle_error.to_string().contains("did not recycle"));

    pool.acquire_results.lock().push_back(Err("pool offline".to_string()));
    let acquire_io_error = lease_gateway_local_agent_workspace_for_project(
        workspace_request("task-acquire-io")?,
        &project,
        &pool,
    )
    .await
    .expect_err("acquire IO failure");
    assert!(
        acquire_io_error.to_string().contains("acquire task worktree for local agent execution")
    );

    pool.release_results.lock().push_back(Err("pool offline".to_string()));
    let release_io_error = release_gateway_local_agent_workspace_for_project(
        release_request("task-release-io")?,
        &project,
        &pool,
    )
    .await
    .expect_err("release IO failure");
    assert!(release_io_error.to_string().contains("release successful local agent worktree"));

    pool.recycle_results.lock().push_back(Err("pool offline".to_string()));
    let recycle_io_error = recycle_gateway_local_agent_workspace_for_project(
        recycle_request("task-recycle-io", "bad")?,
        &project,
        &pool,
    )
    .await
    .expect_err("recycle IO failure");
    assert!(recycle_io_error.to_string().contains("recycle failed local agent worktree"));
    Ok(())
}

#[test]
fn gateway_real_chat_client_is_disabled_in_unit_tests() -> Result<()> {
    let chat = RealGatewayChatStreamClient;
    let chat_error = block_on_gateway_future(
        "real chat stub",
        chat.collect_with_observer(
            "/repo".to_string(),
            gateway_local_agent_stream_request(&execution_request(None, None)?),
            Box::new(|_| {}),
        ),
    )
    .expect_err("real chat stream is disabled");
    assert!(chat_error.to_string().contains("disabled in tests"));
    Ok(())
}

#[test]
fn gateway_future_runner_propagates_success_and_error_without_runtime() {
    assert_eq!(run_gateway_future("run success", async { Ok(7) }).expect("future succeeds"), 7);
    let error = run_gateway_future::<(), _>("run failure", async { bail!("future failed") })
        .expect_err("future error should propagate");
    assert_eq!(error.to_string(), "future failed");
}

#[tokio::test]
async fn gateway_future_runner_works_inside_existing_runtime() {
    let value = run_gateway_future("nested runtime", async { Ok(11) })
        .expect("nested runtime future succeeds");
    assert_eq!(value, 11);
}

fn execution_request(
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
            slot_id: "slot-1".to_string(),
            path: "/repo/.vw/worktrees/slot-1".to_string(),
            branch: "codex/task-1".to_string(),
        },
    })
}

fn workspace_request(task_id: &str) -> Result<LocalAgentWorkspaceRequest> {
    Ok(LocalAgentWorkspaceRequest {
        project_path: PathBuf::from("/repo"),
        task_id: TaskId::new(task_id)?,
        run_id: RunId::new("run-1")?,
    })
}

fn release_request(task_id: &str) -> Result<LocalAgentWorkspaceReleaseRequest> {
    Ok(LocalAgentWorkspaceReleaseRequest {
        project_path: PathBuf::from("/repo"),
        task_id: TaskId::new(task_id)?,
        run_id: RunId::new("run-1")?,
        workspace: LocalAgentWorkspace {
            slot_id: "slot-1".to_string(),
            path: "/repo/.vw/worktrees/slot-1".to_string(),
            branch: "codex/task".to_string(),
        },
    })
}

fn recycle_request(task_id: &str, taint_reason: &str) -> Result<LocalAgentWorkspaceRecycleRequest> {
    let release = release_request(task_id)?;
    Ok(LocalAgentWorkspaceRecycleRequest {
        project_path: release.project_path,
        task_id: release.task_id,
        run_id: release.run_id,
        workspace: release.workspace,
        taint_reason: taint_reason.to_string(),
    })
}

fn project_info(worktree: &str) -> project::Info {
    project::Info {
        id: "project-1".to_string(),
        worktree: worktree.to_string(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 1, updated: 1, initialized: None },
        sandboxes: Vec::new(),
    }
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
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
        observe_payload: GatewayPayloadObserver<'a>,
    ) -> GatewayFuture<'a, GatewayChatStreamCollected> {
        self.calls.lock().push((
            workspace_path,
            body.session_id.as_ref().map(|session_id| session_id.as_ref().to_string()),
            body.messages,
        ));
        let payloads = self.payloads.lock().pop_front().unwrap_or_default();
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
    match queue.lock().pop_front() {
        Some(Ok(value)) => Ok(value),
        Some(Err(message)) => bail!(message),
        None => bail!("unexpected {operation} call"),
    }
}

#[derive(Default)]
struct FakeWorktreePool {
    acquire_results: Mutex<VecDeque<Result<TaskPoolWorktreeAcquireResponse, String>>>,
    release_results: Mutex<VecDeque<Result<TaskPoolWorktreeReleaseResponse, String>>>,
    recycle_results: Mutex<VecDeque<Result<TaskPoolWorktreeRecycleResponse, String>>>,
    acquire_calls: Mutex<Vec<(String, String)>>,
    release_calls: Mutex<Vec<(String, String)>>,
    recycle_calls: Mutex<Vec<(String, String, Option<String>)>>,
}

impl GatewayWorktreePoolClient for FakeWorktreePool {
    fn acquire_and_prepare<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeAcquireResponse> {
        self.acquire_calls.lock().push((
            normalize_project_path(Path::new(&project.worktree)),
            task_id.as_ref().to_string(),
        ));
        let result = pop_pool_result(&self.acquire_results, "acquire");
        Box::pin(async move { result })
    }

    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
    ) -> GatewayFuture<'a, TaskPoolWorktreeReleaseResponse> {
        self.release_calls.lock().push((
            normalize_project_path(Path::new(&project.worktree)),
            task_id.as_ref().to_string(),
        ));
        let result = pop_pool_result(&self.release_results, "release");
        Box::pin(async move { result })
    }

    fn recycle_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_types::id::TaskId,
        taint_reason: Option<String>,
    ) -> GatewayFuture<'a, TaskPoolWorktreeRecycleResponse> {
        self.recycle_calls.lock().push((
            normalize_project_path(Path::new(&project.worktree)),
            task_id.as_ref().to_string(),
            taint_reason,
        ));
        let result = pop_pool_result(&self.recycle_results, "recycle");
        Box::pin(async move { result })
    }
}

fn pop_pool_result<T>(queue: &Mutex<VecDeque<Result<T, String>>>, operation: &str) -> Result<T> {
    match queue.lock().pop_front() {
        Some(Ok(value)) => Ok(value),
        Some(Err(message)) => bail!(message),
        None => bail!("unexpected {operation} call"),
    }
}
