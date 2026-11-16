//! 测试网关执行器的请求转换、流事件处理、取消与结果归档。
//! 脚本化事件流覆盖完成、失败与取消，确保最终归档不依赖用户可见文本解析。

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use parking_lot::Mutex;
use serde_json::{Value, json};
use vw_api_transport::common::TimestampMs;
use std::collections::VecDeque;
use vw_api_task::{
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto,
};

use crate::runner::gateway_executor::*;
use crate::runner::local::*;
use crate::store::AgentSchedulerStore;
use agent_project as project;
use crate::domain::{AssigneeId, RunId, TaskId};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::store::runs::ClaimTaskRunRequest;
use crate::store::tasks::NewAgentTask;

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
fn acp_background_approval_scheduler_keeps_session_isolation() -> Result<()> {
    let request = execution_request(Some("reviewer"), Some("codex"))?;

    let stream = gateway_local_agent_stream_request(&request);
    let options = stream.options.expect("gateway options");

    assert_eq!(
        stream.session_id.as_ref().map(|session_id| session_id.as_ref()),
        Some("agent-scheduler-task-1-run-1")
    );
    assert_eq!(stream.messages, vec![json!({"role": "user", "content": "Do the task"})]);
    assert_eq!(options["cwd"], "/repo/.vw/worktrees/slot-1");
    assert_eq!(options["full_access"], false);
    assert_eq!(options["agent"], "reviewer");
    assert_eq!(options["acp_agent"], "codex");
    assert_eq!(options["acp_force_new_session"], true);
    assert_eq!(options["acp_close_client_after_request"], true);
    assert_eq!(options["acp_history_strategy"], "discard");
    assert_eq!(options["acp_history_recent_count"], 1);
    assert_eq!(options["acp_permission_mode"], "approve-reads");
    assert_eq!(options["acp_non_interactive_permissions"], "deny");
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
    assert_eq!(options.len(), 1);
    assert_eq!(options["cwd"], "/repo/.vw/worktrees/slot-1");
    assert!(!options.contains_key("full_access"));
    Ok(())
}

#[test]
fn gateway_stream_request_preserves_reasoning_effort_without_acp() -> Result<()> {
    let mut request = execution_request(None, None)?;
    request.reasoning_effort = Some(vw_api_chat_identity::ReasoningEffortDto::High);
    let stream = gateway_local_agent_stream_request(&request);
    assert_eq!(stream.reasoning_effort, request.reasoning_effort);
    assert_eq!(stream.acp_agent, None);
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
            agent_id: None,
            run_id: None,
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
    let dir = tempfile::tempdir()?;
    let mut request = workspace_request("task-worktree")?;
    request.project_path = dir.path().to_path_buf();
    let release = release_request("task-worktree")?;
    let recycle = recycle_request("task-worktree", "needs clean checkout")?;
    let project = project_info(dir.path().to_str().unwrap());
    let workspace =
        lease_gateway_local_agent_workspace_for_project(request, &project, &pool).await?;
    release_gateway_local_agent_workspace_for_project(release, &project, &pool).await?;
    recycle_gateway_local_agent_workspace_for_project(recycle, &project, &pool).await?;

    assert_eq!(workspace.slot_id, None);
    assert_eq!(workspace.branch, None);
    assert!(pool.acquire_calls.lock().is_empty());
    assert_eq!(
        pool.release_calls.lock().as_slice(),
        &[(normalize_project_path(dir.path()), "task-worktree".to_string())]
    );
    assert_eq!(
        pool.recycle_calls.lock().as_slice(),
        &[(
            normalize_project_path(dir.path()),
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
    fn release_task_slot<'a>(
        &'a self,
        project: &'a project::Info,
        task_id: &'a vw_api_transport::TaskId,
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
        task_id: &'a vw_api_transport::TaskId,
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

#[tokio::test]
async fn task_pool_workspace_non_git_uses_registered_directory() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let project = project_info(dir.path().to_str().unwrap());
    let pool = FakeWorktreePool::default();
    let mut request = workspace_request("directory-only")?;
    request.project_path = dir.path().to_path_buf();
    let workspace =
        lease_gateway_local_agent_workspace_for_project(request, &project, &pool).await?;
    assert_eq!(Path::new(&workspace.path), dir.path().canonicalize()?);
    assert!(pool.acquire_calls.lock().is_empty());
    assert!(!dir.path().join(".git").exists());
    Ok(())
}

#[tokio::test]
async fn task_pool_workspace_rejects_missing_or_foreign_directory() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let foreign = tempfile::tempdir()?;
    let project = project_info(dir.path().to_str().unwrap());
    let pool = FakeWorktreePool::default();
    for path in [dir.path().join("missing"), foreign.path().to_path_buf()] {
        let mut request = workspace_request("invalid-directory")?;
        request.project_path = path;
        assert!(
            lease_gateway_local_agent_workspace_for_project(request, &project, &pool)
                .await
                .is_err()
        );
    }
    assert!(pool.acquire_calls.lock().is_empty());
    Ok(())
}

#[test]
fn task_pool_agent_does_not_elevate_background_permissions() -> Result<()> {
    for (agent, backend) in [(None, None), (Some("main"), None), (Some("acp-codex"), Some("codex"))]
    {
        let body = gateway_local_agent_stream_request(&execution_request(agent, backend)?);
        assert_ne!(body.options.unwrap()["full_access"], true);
    }
    Ok(())
}

#[test]
fn task_pool_agent_runs_use_distinct_sessions() -> Result<()> {
    let mut task = execution_request(Some("main"), None)?;
    let first = gateway_local_agent_stream_request(&task);
    task.run_id = RunId::new("run-2")?;
    let second = gateway_local_agent_stream_request(&task);
    assert_ne!(first.session_id, second.session_id);
    assert_eq!(first.agent, second.agent);
    Ok(())
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
            slot_id: Some("slot-1".to_string()),
            path: "/repo/.vw/worktrees/slot-1".to_string(),
            branch: Some("codex/task-1".to_string()),
        },
    })
}

fn workspace_request(task_id: &str) -> Result<LocalAgentWorkspaceRequest> {
    Ok(LocalAgentWorkspaceRequest {
        agent: None,
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
            slot_id: Some("slot-1".to_string()),
            path: "/repo/.vw/worktrees/slot-1".to_string(),
            branch: Some("codex/task".to_string()),
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
