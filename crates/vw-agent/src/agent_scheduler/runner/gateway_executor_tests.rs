//! 测试网关执行器的请求转换、流事件处理、取消与结果归档。
//! 脚本化事件流覆盖完成、失败与取消，确保最终归档不依赖用户可见文本解析。

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use vw_api_transport::common::TimestampMs;

use super::*;
use task_scheduler::gateway_executor_contract_tests::{execution_request, finite_task};
use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;


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
    task_scheduler::gateway_executor_contract_tests::gateway_executor_workspace_methods_fail_before_pool_io_for_unregistered_project::<SchedulerGatewayHost>()
}

#[test]
fn gateway_executor_execute_collects_stream_output_and_appends_observed_logs() -> Result<()> {
    task_scheduler::gateway_executor_contract_tests::gateway_executor_execute_collects_stream_output_and_appends_observed_logs::<SchedulerGatewayHost>()
}

#[test]
fn gateway_executor_execute_propagates_stream_collection_error() -> Result<()> {
    task_scheduler::gateway_executor_contract_tests::gateway_executor_execute_propagates_stream_collection_error::<SchedulerGatewayHost>()
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


#[test]
fn task_pool_recovery_gateway_passes_stop_to_stream() -> Result<()> {
    task_scheduler::gateway_executor_contract_tests::task_pool_recovery_gateway_passes_stop_to_stream::<SchedulerGatewayHost>()
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
