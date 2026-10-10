//! 测试本地运行器的任务图约束、工作树租约、执行结果与提交收尾。
//! 可控执行器制造无变更、失败和工作区错误，确认运行记录与资源回收同步。

use anyhow::Result;
use serde_json::json;
use std::path::Path;
use vw_api_types::common::TimestampMs;

use super::domain::{AgentRunStatus, TaskId, WorkerName};
use super::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentRunner,
    LocalAgentRunnerRequest, LocalAgentWorkspace, LocalAgentWorkspaceRecycleRequest,
    LocalAgentWorkspaceReleaseRequest, LocalAgentWorkspaceRequest, add_ms, execution_acp_agent,
    execution_agent, execution_artifact_payload, execution_metadata_string, execution_model,
    execution_prompt, git_has_staged_changes, git_output_detail, normalize_acp_agent,
    object_metadata, run_git_ok, truncate_log_text,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::tasks::NewAgentTask;
use super::task_graph::AgentTaskDependency;

#[test]
fn local_runner_success_leaves_finite_task_submitted_until_merge() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-1")?;
    let executor = RecordingExecutor::succeeding("local execution completed");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(1_000),
            },
        )?
        .expect("queued finite task runs locally");

    let task = store.load_task(&project_path, &task_id)?.expect("task persists after local run");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["selected_worktree_slot_id"], "local-slot");
    assert_eq!(task.metadata["selected_worktree_branch"], "codex/task-local-runner-1");
    assert_eq!(task.metadata["merge_source_branch"], "codex/task-local-runner-1");
    assert!(
        task.metadata["selected_worktree_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("worktrees/local-slot"))
    );
    assert_eq!(outcome.task_id, task_id);
    assert_eq!(outcome.run.status, AgentRunStatus::Succeeded);

    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert_eq!(logs.len(), 7);
    assert_eq!(logs[0].from_status, Some(AgentTaskStatus::Queued));
    assert_eq!(logs[0].to_status, Some(AgentTaskStatus::Assigned));
    assert_eq!(logs[1].to_status, Some(AgentTaskStatus::Dispatched));
    assert_eq!(logs[2].to_status, Some(AgentTaskStatus::Acknowledged));
    assert_eq!(logs[3].to_status, Some(AgentTaskStatus::Running));
    assert!(logs[4].message.contains("local worktree lease recorded"));
    assert!(logs[5].message.contains("local execution committed workspace changes"));
    assert_eq!(logs[6].to_status, Some(AgentTaskStatus::Submitted));
    assert_eq!(logs[6].message, "local execution submitted result");

    let artifacts = store.list_task_artifacts(&project_path, &task_id)?;
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].kind, "execution_result");
    assert_eq!(artifacts[0].run_id, Some(outcome.run.id.clone()));
    assert_eq!(artifacts[0].payload["success"], true);
    assert_eq!(artifacts[0].payload["output"], "local execution completed");

    let workspace = store
        .load_run_workspace_context(&project_path, &outcome.run.id)?
        .expect("workspace context is recorded");
    assert_eq!(workspace.workspace.slot_id, "local-slot");
    assert_eq!(workspace.workspace.branch, "codex/task-local-runner-1");
    let workspace_path = Path::new(&workspace.workspace.path);
    assert_eq!(run_git(workspace_path, &["status", "--short"])?, "");
    assert_eq!(
        run_git(workspace_path, &["log", "-1", "--format=%s"])?,
        "task(task-local-runner-1): submit local execution result"
    );
    let task = store.load_task(&project_path, &task_id)?.expect("task persists after policy");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");

    let requests = executor.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].task_id, task_id);
    assert!(requests[0].prompt.contains("Run local runner through scheduler store"));
    assert!(requests[0].prompt.contains("请输出一句简短最终回复"));
    assert_eq!(requests[0].model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert_eq!(requests[0].agent, None);
    assert_eq!(requests[0].acp_agent.as_deref(), Some("codex"));
    assert_eq!(requests[0].reasoning_effort, Some(vw_api_types::ReasoningEffortDto::High));

    Ok(())
}

#[test]
fn local_runner_forwards_task_reasoning_effort_to_acp_execution() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-reasoning");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-reasoning")?;
    let executor = RecordingExecutor::succeeding("completed");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;
    runner.run_once(
        &project_path,
        LocalAgentRunnerRequest { task_id, now_ms: TimestampMs(1_000) },
    )?;

    let requests = executor.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].acp_agent.as_deref(), Some("codex"));
    assert_eq!(requests[0].reasoning_effort, Some(vw_api_types::ReasoningEffortDto::High));
    Ok(())
}

#[test]
fn local_runner_cannot_bypass_task_graph() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-task-graph");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-blocked")?;
    let predecessor_id = TaskId::new("task-local-runner-prerequisite")?;
    let executor = RecordingExecutor::succeeding("unexpected local execution");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    let mut predecessor = finite_task(&predecessor_id)?;
    predecessor.status = AgentTaskStatus::Submitted;
    store.create_task(&project_path, predecessor)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;
    store.replace_task_dependencies(
        &project_path,
        vec![AgentTaskDependency::new(predecessor_id, task_id.clone())],
    )?;

    let outcome = runner.run_once(
        &project_path,
        LocalAgentRunnerRequest {
            task_id: task_id.clone(),
            now_ms: TimestampMs(1_500),
        },
    )?;

    assert_eq!(outcome, None);
    assert_eq!(
        store.load_task(&project_path, &task_id)?.expect("blocked task persists").status,
        AgentTaskStatus::Queued
    );
    assert!(executor.requests.lock().expect("execution requests lock").is_empty());

    store.transition_task_status(
        &project_path,
        &TaskId::new("task-local-runner-prerequisite")?,
        AgentTaskStatus::Completed,
        TimestampMs(1_501),
    )?;
    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(1_502),
            },
        )?
        .expect("task runs after the prerequisite completes");
    assert_eq!(outcome.task_id, task_id);
    Ok(())
}


#[test]
fn local_runner_logs_worktree_workspace_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-worktree-log");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-worktree-log")?;
    let executor = RecordingExecutor::succeeding("local execution completed");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(3_000),
            },
        )?
        .expect("queued finite task runs locally");

    let logs = store.list_task_logs(&project_path, &task_id)?;
    let workspace_log = logs
        .iter()
        .find(|log| log.message.contains("local worktree lease"))
        .expect("worktree lease log is recorded");
    assert_eq!(workspace_log.run_id.as_ref(), Some(&outcome.run.id));
    assert!(workspace_log.message.contains("slot=local-slot"));
    assert!(workspace_log.message.contains("branch=codex/task-local-runner-worktree-log"));
    assert!(workspace_log.message.contains("worktrees/local-slot"));

    Ok(())
}

#[test]
fn local_agent_runner_executes_with_leased_workspace() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-leased-workspace");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-leased-workspace")?;
    let executor = RecordingExecutor::succeeding("local execution completed");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(4_000),
            },
        )?
        .expect("queued finite task runs locally");

    let workspace_requests = executor.workspace_requests.lock().expect("workspace requests lock");
    assert_eq!(workspace_requests.len(), 1);
    assert_eq!(workspace_requests[0].project_path, project_path);
    assert_eq!(workspace_requests[0].task_id, task_id);

    let requests = executor.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].workspace.path.ends_with("worktrees/local-slot"));
    assert_ne!(Path::new(&requests[0].workspace.path), project_path.as_path());
    assert_eq!(requests[0].workspace.branch, "codex/task-local-runner-leased-workspace");

    Ok(())
}

#[test]
fn local_runner_success_logs_when_workspace_has_no_committable_changes() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-no-changes");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-no-changes")?;
    let executor = RecordingExecutor::succeeding_without_changes("local execution completed");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(5_000),
            },
        )?
        .expect("queued finite task runs locally");

    let task = store.load_task(&project_path, &task_id)?.expect("task persists after local run");
    assert_eq!(task.status, AgentTaskStatus::Submitted);

    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert!(
        logs.iter().any(|log| {
            log.message == "local execution produced no committable workspace changes"
        })
    );
    assert!(logs.iter().any(|log| log.message == "local execution submitted result"));

    let workspace = store
        .load_run_workspace_context(&project_path, &outcome.run.id)?
        .expect("workspace context is recorded");
    let workspace_path = Path::new(&workspace.workspace.path);
    assert_eq!(run_git(workspace_path, &["status", "--short"])?, "");
    assert_eq!(run_git(workspace_path, &["log", "-1", "--format=%s"])?, "initial");

    Ok(())
}

#[test]
fn local_runner_records_failure_when_executor_does_not_lease_workspace() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-missing-workspace");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-missing-workspace")?;
    let executor = MissingWorkspaceExecutor;
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(6_000),
            },
        )?
        .expect("workspace lease failure still records a run");

    assert_eq!(outcome.run.status, AgentRunStatus::Failed);
    assert!(
        outcome
            .run
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("did not provide a task worktree lease"))
    );
    let task = store.load_task(&project_path, &task_id)?.expect("failed task remains stored");
    assert_eq!(task.status, AgentTaskStatus::Failed);
    let artifact = store
        .list_task_artifacts(&project_path, &task_id)?
        .into_iter()
        .next()
        .expect("failure artifact is recorded");
    assert_eq!(artifact.payload["success"], false);
    assert_eq!(artifact.payload["workspace"], serde_json::Value::Null);

    Ok(())
}

#[test]
fn local_runner_start_once_handles_missing_non_finite_and_non_queued_tasks() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-start-boundaries");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let executor = RecordingExecutor::succeeding("completed");
    let runner = LocalAgentRunner::new(&store, &executor);
    store.initialize_project(&project_path)?;

    assert!(
        runner
            .run_once(
                &project_path,
                LocalAgentRunnerRequest {
                    task_id: TaskId::new("missing-run-once-task")?,
                    now_ms: TimestampMs(7_000),
                },
            )?
            .is_none()
    );
    assert!(
        runner
            .start_once(
                &project_path,
                LocalAgentRunnerRequest {
                    task_id: TaskId::new("missing-task")?,
                    now_ms: TimestampMs(7_001),
                },
            )?
            .is_none()
    );

    let non_finite_id = TaskId::new("task-local-runner-infinite")?;
    let mut non_finite = finite_task(&non_finite_id)?;
    non_finite.kind = AgentTaskKind::Infinite;
    store.create_task(&project_path, non_finite)?;
    let error = runner
        .start_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: non_finite_id,
                now_ms: TimestampMs(7_002),
            },
        )
        .expect_err("infinite task rejected")
        .to_string();
    assert!(error.contains("only supports finite tasks"));

    let assigned_id = TaskId::new("task-local-runner-assigned")?;
    let mut assigned = finite_task(&assigned_id)?;
    assigned.status = AgentTaskStatus::Assigned;
    store.create_task(&project_path, assigned)?;
    assert!(
        runner
            .start_once(
                &project_path,
                LocalAgentRunnerRequest {
                    task_id: assigned_id,
                    now_ms: TimestampMs(7_003),
                },
            )?
            .is_none()
    );

    Ok(())
}

#[test]
fn local_runner_finish_started_run_handles_missing_non_finite_and_non_running_tasks() -> Result<()>
{
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-finish-boundaries");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let executor = RecordingExecutor::succeeding("completed");
    let runner = LocalAgentRunner::new(&store, &executor);
    let run_id = super::domain::RunId::new("local-run-boundary")?;
    store.initialize_project(&project_path)?;

    assert!(
        runner
            .finish_started_run(
                &project_path,
                &TaskId::new("missing-task")?,
                &run_id,
                LocalAgentRunnerRequest {
                    task_id: TaskId::new("missing-task")?,
                    now_ms: TimestampMs(8_000),
                },
            )?
            .is_none()
    );

    let non_finite_id = TaskId::new("task-local-runner-finish-infinite")?;
    let mut non_finite = finite_task(&non_finite_id)?;
    non_finite.kind = AgentTaskKind::Infinite;
    non_finite.status = AgentTaskStatus::Running;
    store.create_task(&project_path, non_finite)?;
    let error = runner
        .finish_started_run(
            &project_path,
            &non_finite_id,
            &run_id,
            LocalAgentRunnerRequest {
                task_id: non_finite_id.clone(),
                now_ms: TimestampMs(8_001),
            },
        )
        .expect_err("infinite task rejected")
        .to_string();
    assert!(error.contains("only supports finite tasks"));

    let submitted_id = TaskId::new("task-local-runner-finish-submitted")?;
    let mut submitted = finite_task(&submitted_id)?;
    submitted.status = AgentTaskStatus::Submitted;
    store.create_task(&project_path, submitted)?;
    assert!(
        runner
            .finish_started_run(
                &project_path,
                &submitted_id,
                &run_id,
                LocalAgentRunnerRequest {
                    task_id: submitted_id.clone(),
                    now_ms: TimestampMs(8_002),
                },
            )?
            .is_none()
    );

    Ok(())
}

#[test]
fn local_runner_fails_and_recycles_workspace_when_commit_cannot_read_workspace() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-commit-failure");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-commit-failure")?;
    let executor = MissingCommittedWorkspaceExecutor::new(project_path.join("missing-worktree"));
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                now_ms: TimestampMs(9_000),
            },
        )?
        .expect("commit failure still records run");

    assert_eq!(outcome.run.status, AgentRunStatus::Failed);
    assert!(
        outcome
            .run
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("workspace path does not exist"))
    );
    let recycled = executor.recycled.lock().expect("recycled lock");
    assert_eq!(recycled.len(), 1);
    assert!(recycled[0].taint_reason.contains("workspace path does not exist"));
    assert_eq!(recycled[0].workspace.slot_id, "missing-slot");

    Ok(())
}

#[test]
fn local_executor_default_release_and_recycle_succeed() -> Result<()> {
    let executor = MissingWorkspaceExecutor;
    let workspace = LocalAgentWorkspace {
        slot_id: "slot".to_string(),
        path: "/tmp/workspace".to_string(),
        branch: "branch".to_string(),
    };
    let project_path = Path::new("/tmp/project").to_path_buf();
    let task_id = TaskId::new("task-local-defaults")?;
    let run_id = super::domain::RunId::new("run-local-defaults")?;

    LocalAgentExecutor::release_workspace(
        &executor,
        LocalAgentWorkspaceReleaseRequest {
            project_path: project_path.clone(),
            task_id: task_id.clone(),
            run_id: run_id.clone(),
            workspace: workspace.clone(),
        },
    )?;
    LocalAgentExecutor::recycle_workspace(
        &executor,
        LocalAgentWorkspaceRecycleRequest {
            project_path,
            task_id,
            run_id,
            workspace,
            taint_reason: "not needed".to_string(),
        },
    )?;

    Ok(())
}

#[test]
fn local_runner_helpers_cover_metadata_truncation_time_and_git_output_edges() -> Result<()> {
    assert_eq!(add_ms(TimestampMs(10), 5)?, TimestampMs(15));
    assert!(add_ms(TimestampMs(i64::MAX), 1).is_err());
    assert_eq!(truncate_log_text("  short path  ", 20), "short path");
    assert_eq!(truncate_log_text("abcdef", 5), "ab...");
    assert_eq!(normalize_acp_agent("default".to_string()), None);
    assert_eq!(normalize_acp_agent("acp".to_string()), None);
    assert_eq!(normalize_acp_agent("internal".to_string()), None);
    assert_eq!(normalize_acp_agent("codex".to_string()), Some("codex".to_string()));
    assert!(object_metadata(serde_json::Value::Null)?.is_empty());
    assert!(object_metadata(json!("not-object")).is_err());

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-runner-helper-task");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-runner-helper")?;
    store.initialize_project(&project_path)?;
    let mut task = finite_task(&task_id)?;
    task.title = "Fallback title".to_string();
    task.metadata = json!({
        "prompt": "  ",
        "model": "AUTO",
        "agent": "main",
        "acp_agent": "default"
    });
    store.create_task(&project_path, task)?;
    let stored = store.load_task(&project_path, &task_id)?.expect("stored task");
    assert!(execution_prompt(&stored).starts_with("Fallback title\n\n"));
    assert_eq!(execution_model(&stored), None);
    assert_eq!(execution_acp_agent(&stored), None);
    assert_eq!(execution_agent(&stored, Some("codex")), None);
    assert_eq!(execution_agent(&stored, None), Some("main".to_string()));
    assert_eq!(execution_metadata_string(&stored, "missing"), None);

    let payload = execution_artifact_payload(
        false,
        Some("out"),
        Some("err"),
        Some(&LocalAgentWorkspace {
            slot_id: "slot".to_string(),
            path: "/tmp/workspace".to_string(),
            branch: "branch".to_string(),
        }),
    );
    assert_eq!(payload["success"], false);
    assert_eq!(payload["output"], "out");
    assert_eq!(payload["error"], "err");
    assert_eq!(payload["workspace"]["slot_id"], "slot");

    let stdout_only = std::process::Command::new("sh").arg("-c").arg("printf stdout").output()?;
    assert_eq!(git_output_detail(&stdout_only), "stdout");
    let stderr_only =
        std::process::Command::new("sh").arg("-c").arg("printf stderr >&2").output()?;
    assert_eq!(git_output_detail(&stderr_only), "stderr");
    let both = std::process::Command::new("sh")
        .arg("-c")
        .arg("printf stdout; printf stderr >&2")
        .output()?;
    assert_eq!(git_output_detail(&both), "stdout\nstderr");
    let empty = std::process::Command::new("sh").arg("-c").arg("exit 7").output()?;
    assert!(git_output_detail(&empty).contains("exit status"));
    let non_repo = tempfile::tempdir()?;
    assert!(git_has_staged_changes(non_repo.path()).is_err());
    assert!(run_git_ok(non_repo.path(), &["not-a-real-subcommand"]).is_err());

    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Run local runner through scheduler store".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
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
        metadata: json!({
            "prompt": "Run local runner through scheduler store",
            "model": "deepseek/deepseek-v4-flash",
            "agent": "main",
            "acp_agent": "codex",
            "reasoning_effort": "high"
        }),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

struct RecordingExecutor {
    output: Result<LocalAgentExecutionOutput, String>,
    write_output_file: bool,
    requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,
    workspace_requests: std::sync::Mutex<Vec<LocalAgentWorkspaceRequest>>,
}

struct MissingCommittedWorkspaceExecutor {
    workspace_path: std::path::PathBuf,
    recycled: std::sync::Mutex<Vec<LocalAgentWorkspaceRecycleRequest>>,
}

struct MissingWorkspaceExecutor;

impl LocalAgentExecutor for MissingWorkspaceExecutor {
    fn execute(&self, _request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        anyhow::bail!("execute must not run without a workspace")
    }
}

impl RecordingExecutor {
    fn succeeding(output: &str) -> Self {
        Self {
            output: Ok(LocalAgentExecutionOutput { raw_output: output.to_string() }),
            write_output_file: true,
            requests: std::sync::Mutex::new(Vec::new()),
            workspace_requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn succeeding_without_changes(output: &str) -> Self {
        Self {
            output: Ok(LocalAgentExecutionOutput { raw_output: output.to_string() }),
            write_output_file: false,
            requests: std::sync::Mutex::new(Vec::new()),
            workspace_requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            output: Err(message.to_string()),
            write_output_file: false,
            requests: std::sync::Mutex::new(Vec::new()),
            workspace_requests: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl MissingCommittedWorkspaceExecutor {
    fn new(workspace_path: std::path::PathBuf) -> Self {
        Self { workspace_path, recycled: std::sync::Mutex::new(Vec::new()) }
    }
}

impl LocalAgentExecutor for MissingCommittedWorkspaceExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            slot_id: "missing-slot".to_string(),
            path: self.workspace_path.to_string_lossy().to_string(),
            branch: format!("codex/{}", request.task_id.as_str()),
        })
    }

    fn execute(&self, _request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        Ok(LocalAgentExecutionOutput { raw_output: "completed before commit".to_string() })
    }

    fn recycle_workspace(&self, request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
        self.recycled.lock().expect("recycled lock").push(request);
        Ok(())
    }
}

impl LocalAgentExecutor for RecordingExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        let path = request.project_path.join("worktrees/local-slot");
        prepare_git_workspace(&path, &format!("codex/{}", request.task_id.as_str()))?;
        self.workspace_requests.lock().expect("workspace requests lock").push(request.clone());
        Ok(LocalAgentWorkspace {
            slot_id: "local-slot".to_string(),
            path: path.to_string_lossy().to_string(),
            branch: format!("codex/{}", request.task_id.as_str()),
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        if self.write_output_file
            && let Ok(output) = &self.output
        {
            std::fs::write(
                Path::new(&request.workspace.path).join("agent-output.txt"),
                &output.raw_output,
            )?;
        }
        self.requests.lock().expect("requests lock").push(request);
        self.output.clone().map_err(anyhow::Error::msg)
    }
}

fn prepare_git_workspace(path: &Path, branch: &str) -> Result<()> {
    std::fs::create_dir_all(path)?;
    if !path.join(".git").exists() {
        run_git(path, &["init"])?;
        run_git(path, &["config", "user.email", "test@example.com"])?;
        run_git(path, &["config", "user.name", "Test User"])?;
        run_git(path, &["checkout", "-b", branch])?;
        run_git(path, &["commit", "--allow-empty", "-m", "initial"])?;
    }
    Ok(())
}

fn run_git(path: &Path, args: &[&str]) -> Result<String> {
    let output =
        crate::app::agent::shell::git_std_command().current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim())
}
