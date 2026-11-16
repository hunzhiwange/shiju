//! 测试网关执行器的请求转换、流事件处理、取消与结果归档。
//! 脚本化事件流覆盖完成、失败与取消，确保最终归档不依赖用户可见文本解析。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use parking_lot::Mutex;
use serde_json::{Value, json};
use vw_api_transport::common::TimestampMs;
use std::collections::VecDeque;

use super::*;
use crate::agent_scheduler::domain::{AssigneeId, RunId, TaskId};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;
use crate::agent_scheduler::store::tasks::NewAgentTask;
use crate::app::agent::gateway::api::handlers::session::stream::GatewayChatStreamCollected;


#[cfg(unix)]
#[tokio::test]
#[ignore = "requires python3 for the ACP protocol fixture"]
async fn acp_background_approval_scheduler_writes_only_inside_worktree() -> Result<()> {
    use crate::gateway::api::handlers::session::stream::collect_background_gateway_chat_stream;
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?.join("worktree");
    std::fs::create_dir(&workspace)?;
    let outside = temp.path().join("outside.txt");
    let peer = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_acp_approval.py");
    crate::config::update(json!({
        "agents":{"acp-codex":{"enabled":true,"runtime":{"kind":"acp","backend":"codex"}}},
        "acp":{"codex":{"command":"python3", "args":[peer, "--peer", workspace, outside]}}
    }))
    .await?;

    let mut task = execution_request(Some("acp-codex"), Some("codex"))?;
    task.workspace.path = workspace.to_string_lossy().into_owned();
    let mut request = gateway_local_agent_stream_request(&task);
    // The controlled peer exercises host callbacks without a supplier model or credentials.
    request.acp_model_source = AcpModelSource::CliLocal;
    request.model = None;

    let mut completed = false;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        collect_background_gateway_chat_stream(task.workspace.path, request, false, |event| {
            completed |= event["type"] == "chat.done";
        }),
    )
    .await??;
    assert!(completed);
    assert!(
        result
            .output
            .contains(r#"ACP_APPROVAL_FIXTURE_DONE [["deny", "deny", "deny"], false, "new"]"#)
    );
    assert_eq!(std::fs::read_to_string(workspace.join("note.txt"))?, "second edit");
    assert!(!outside.exists());
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
fn gateway_real_chat_client_is_disabled_in_unit_tests() -> Result<()> {
    let chat = RealGatewayChatStreamClient;
    let chat_error = run_gateway_future(
        "real chat stub",
        async move { chat.collect_with_observer(
            "/repo".to_string(),
            gateway_local_agent_stream_request(&execution_request(None, None)?),
            tokio_util::sync::CancellationToken::new(),
            Box::new(|_| {}),
        ).await },
    )
    .expect_err("real chat stream is disabled");
    assert!(chat_error.to_string().contains("disabled in tests"));
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
        _cancellation: tokio_util::sync::CancellationToken,
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

#[test]
fn task_pool_recovery_gateway_passes_stop_to_stream() -> Result<()> {
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
    let executor = GatewayLocalAgentExecutor::new_with_chat_stream(store, Arc::new(CancelStream));
    assert!(
        executor.execute(request).unwrap_err().to_string().contains("acknowledged cancellation")
    );
    Ok(())
}

async fn exercise_task_pool_worktree(git: bool, fail_run: bool) -> Result<()> {
    use crate::tools::{Tool, ToolUseContext, scope_tool_use_context};
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir()?;
    if git {
        let repo = git2::Repository::init(dir.path())?;
        std::fs::write(dir.path().join("seed.txt"), "seed")?;
        let mut index = repo.index()?;
        index.add_path(Path::new("seed.txt"))?;
        index.write()?;
        let tree = repo.find_tree(index.write_tree()?)?;
        let signature = git2::Signature::now("Test", "test@example.com")?;
        repo.commit(Some("HEAD"), &signature, &signature, "seed", &tree, &[])?;
    }
    let store = AgentSchedulerStore::new();
    store.initialize_project(dir.path())?;
    let task_id = TaskId::new("explicit-worktree-task")?;
    let run_id = RunId::new("explicit-worktree-run")?;
    store.create_task(dir.path(), finite_task(&task_id)?)?;
    store
        .claim_task_run(
            dir.path(),
            ClaimTaskRunRequest {
                task_id: task_id.clone(),
                run_id: run_id.clone(),
                lease_acquired_at_ms: TimestampMs(100),
                lease_expires_at_ms: None,
            },
        )?
        .expect("claim");
    store
        .heartbeat_run(dir.path(), &run_id, TimestampMs(101), TimestampMs(10_000))?
        .expect("running heartbeat");
    let path = dir.path().to_path_buf();
    let owned_path = path.clone();
    let owned_store = store.clone();
    let owned_task = task_id.clone();
    let owned_run = run_id.clone();
    crate::project::instance::provide(&path, None, move || {
        Box::pin(async move {
            let context = Arc::new(
                ToolUseContext::new(
                    "explicit-worktree-run",
                    Some(owned_path.to_string_lossy().into_owned()),
                )
                .with_run_worktree_owner(crate::tools::RunWorktreeOwner::new(
                    crate::tools::context::run_worktree::SchedulerWorktreeOwner {
                        claims: Default::default(),
                        store: owned_store.clone(),
                        project_path: owned_path.clone(),
                        task_id: owned_task.clone(),
                        run_id: owned_run.clone(),
                    },
                )),
            );
            assert!(context.worktree_binding_state().directory.is_none());
            assert!(owned_store.list_task_artifacts(&owned_path, &owned_task)?.is_empty());
            if git {
                assert_eq!(git2::Repository::open(&owned_path)?.worktrees()?.len(), 0);
            }
            let result = scope_tool_use_context(
                context.clone(),
                crate::tools::enter_worktree::EnterWorktreeTool::new()
                    .call(json!({"name": "explicit-task"})),
            )
            .await;
            if !git {
                assert!(result.is_err());
                assert!(context.worktree_binding_state().directory.is_none());
                assert!(owned_store.list_task_artifacts(&owned_path, &owned_task)?.is_empty());
                assert!(!owned_path.join(".git").exists());
                return Ok::<_, anyhow::Error>(());
            }
            result?;
            let binding = context.worktree_binding_state();
            let workspace = context.workspace_root().expect("effective directory");
            assert_ne!(workspace, owned_path);
            std::fs::write(workspace.join("draft.txt"), "recoverable")?;
            assert!(!owned_path.join("draft.txt").exists());
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .current_dir(&workspace)
                .output()?;
            assert!(output.status.success());
            assert_eq!(
                Path::new(String::from_utf8(output.stdout)?.trim()).canonicalize()?,
                workspace.canonicalize()?
            );
            let artifacts = owned_store.list_task_artifacts(&owned_path, &owned_task)?;
            assert_eq!(artifacts.len(), 1);
            assert_eq!(artifacts[0].run_id.as_ref(), Some(&owned_run));
            assert_eq!(artifacts[0].path, binding.directory);
            assert_eq!(artifacts[0].payload["binding"]["branch"], binding.branch.unwrap());
            if fail_run {
                owned_store
                    .finish_run(
                        &owned_path,
                        &owned_run,
                        crate::agent_scheduler::store::runs::AgentRunFinishStatus::Failed,
                        TimestampMs(200),
                        Some("model failed".into()),
                    )?
                    .expect("finished run");
                // Shutdown drops all in-memory ownership; recovery keeps the checkout and artifact.
                context.run_worktree_owner().unwrap().release_directories()?;

                assert_eq!(std::fs::read_to_string(workspace.join("draft.txt"))?, "recoverable");
                assert!(git2::Repository::open(&workspace).is_ok());
            } else {
                let exit = scope_tool_use_context(
                    context.clone(),
                    crate::tools::exit_worktree::ExitWorktreeTool::new().call(json!({})),
                )
                .await?;
                assert_eq!(exit.data["dirty"], true);
                assert!(workspace.exists());
                assert!(
                    scope_tool_use_context(
                        context.clone(),
                        crate::tools::exit_worktree::ExitWorktreeTool::new()
                            .call(json!({"force": true}))
                    )
                    .await
                    .is_err()
                );
                std::fs::remove_file(workspace.join("draft.txt"))?;
                let error = scope_tool_use_context(
                    context.clone(),
                    crate::tools::ExitWorktreeTool::new().call(json!({})),
                )
                .await
                .expect_err("managed deletion requires existing preflight and snapshot API");
                assert!(error.to_string().contains("deletion-preflight"));
                assert!(workspace.exists());
                assert!(context.worktree_binding_state().directory.is_some());
            }
            context.run_worktree_owner().unwrap().release_directories()?;
            Ok(())
        })
    })
    .await??;
    Ok(())
}

#[tokio::test]
async fn task_pool_worktree_created_only_by_explicit_tool() -> Result<()> {
    exercise_task_pool_worktree(true, false).await
}

#[tokio::test]
async fn task_pool_worktree_non_git_rejected_without_side_effects() -> Result<()> {
    exercise_task_pool_worktree(false, false).await
}

#[tokio::test]
async fn task_pool_worktree_failed_run_preserves_changes() -> Result<()> {
    exercise_task_pool_worktree(true, true).await
}
