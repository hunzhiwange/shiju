//! 测试本地调度从任务入队到运行完成的端到端流程。
//! 端到端夹具保留真实领取与完成路径，同时以临时资源隔离外部环境。

use anyhow::Result;
use serde_json::json;
use std::path::Path;
use vw_api_types::common::TimestampMs;

use super::domain::{AgentRunStatus, AssigneeId, TaskId};
use super::events::AgentEventKind;
use super::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentRunner,
    LocalAgentRunnerRequest, LocalAgentWorkspace, LocalAgentWorkspaceRequest,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::tasks::NewAgentTask;

struct RecordingExecutor {
    output: String,
    requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,
}

impl RecordingExecutor {
    fn new(output: &str) -> Self {
        Self { output: output.to_string(), requests: std::sync::Mutex::new(Vec::new()) }
    }
}

impl LocalAgentExecutor for RecordingExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        let path = request.project_path.join("worktrees/local-e2e-slot");
        prepare_git_workspace(&path, &format!("codex/{}", request.task_id.as_str()))?;
        Ok(LocalAgentWorkspace {
            slot_id: Some("local-e2e-slot".to_string()),
            path: path.display().to_string(),
            branch: Some(format!("codex/{}", request.task_id.as_str())),
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        self.requests.lock().expect("requests lock").push(request);
        Ok(LocalAgentExecutionOutput { raw_output: self.output.clone() })
    }
}

fn prepare_git_workspace(path: &Path, branch: &str) -> Result<()> {
    std::fs::create_dir_all(path)?;
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
        vec!["checkout", "-b", branch],
        vec!["commit", "--allow-empty", "-m", "initial"],
    ] {
        let output =
            crate::app::agent::shell::git_std_command().current_dir(path).args(&args).output()?;
        anyhow::ensure!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
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

struct AcceptanceExecutor {
    fail: std::sync::atomic::AtomicBool,
    requests: std::sync::Mutex<Vec<LocalAgentExecutionRequest>>,
}

impl LocalAgentExecutor for AcceptanceExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            path: request.project_path.display().to_string(),
            slot_id: None,
            branch: None,
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        use super::runner::workspace_claim::{WorkspaceClaim, lock_root};
        assert!(
            WorkspaceClaim::acquire(
                &lock_root()?,
                Path::new(&request.workspace.path),
                "competitor".into()
            )?
            .is_none()
        );
        std::fs::write(Path::new(&request.workspace.path).join("artifact.txt"), "retained")?;
        self.requests.lock().unwrap().push(request);
        if self.fail.swap(false, std::sync::atomic::Ordering::SeqCst) {
            anyhow::bail!("local protocol failure after writing artifact");
        }
        Ok(LocalAgentExecutionOutput { raw_output: "completed".into() })
    }
}

#[test]
fn task_pool_acceptance_failure_and_retry_keep_workspace_ownership() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(directory.path())?;
    let id = TaskId::new("acceptance-retry")?;
    store.create_task(directory.path(), finite_task(&id)?)?;
    let executor = AcceptanceExecutor { fail: true.into(), requests: Default::default() };
    let runner = LocalAgentRunner::new(&store, &executor);
    let first = runner
        .run_once(
            directory.path(),
            LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(1000) },
        )?
        .unwrap();
    assert_eq!(first.run.status, AgentRunStatus::Failed);
    assert_eq!(std::fs::read_to_string(directory.path().join("artifact.txt"))?, "retained");
    store.transition_task_status(
        directory.path(),
        &id,
        AgentTaskStatus::Queued,
        TimestampMs(2000),
    )?;
    let second = runner
        .run_once(
            directory.path(),
            LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(3000) },
        )?
        .unwrap();
    assert_eq!(second.run.status, AgentRunStatus::Succeeded);
    assert_ne!(first.run.id, second.run.id);
    assert_eq!(store.load_task(directory.path(), &id)?.unwrap().status, AgentTaskStatus::Completed);
    assert_eq!(executor.requests.lock().unwrap().len(), 2);
    assert!(!directory.path().join(".git").exists());
    Ok(())
}

#[test]
fn task_pool_acceptance_explicit_actions_and_agent_identity_are_preserved() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(directory.path())?;
    let development = crate::task_lifecycle::built_in_workflow_by_id("dev_lifecycle").unwrap();
    store.save_task_board_workflow(
        directory.path(),
        &serde_json::from_value(serde_json::to_value(development)?)?,
    )?;
    let id = TaskId::new("acceptance-explicit")?;
    store.create_task(directory.path(), finite_task(&id)?)?;
    let executor = AcceptanceExecutor { fail: false.into(), requests: Default::default() };
    let result = LocalAgentRunner::new(&store, &executor)
        .run_once(
            directory.path(),
            LocalAgentRunnerRequest { task_id: id.clone(), now_ms: TimestampMs(1000) },
        )?
        .unwrap();
    assert_eq!(result.run.status, AgentRunStatus::Succeeded);
    let task = store.load_task(directory.path(), &id)?.unwrap();
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    let requests = executor.requests.lock().unwrap();
    assert_eq!(requests[0].acp_agent.as_deref(), Some("codex"));
    assert_eq!(requests[0].model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert!(!directory.path().join(".git").exists());
    Ok(())
}
