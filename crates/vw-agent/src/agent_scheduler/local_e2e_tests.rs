use anyhow::Result;
use serde_json::json;
use std::path::Path;
use vw_api_types::common::TimestampMs;

use super::domain::{AgentRunStatus, TaskId, WorkerName};
use super::events::AgentEventKind;
use super::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentRunner,
    LocalAgentRunnerRequest, LocalAgentWorkspace, LocalAgentWorkspaceRequest,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::AgentSchedulerStore;
use super::store::tasks::NewAgentTask;

#[test]
fn agent_scheduler_local_e2e_runs_without_matrix() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-local-e2e");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-local-e2e")?;
    let worker_id = WorkerName::new("gateway-local")?;
    let executor = RecordingExecutor::new("local e2e completed without Matrix");
    let runner = LocalAgentRunner::new(&store, &executor);

    store.initialize_project(&project_path)?;
    store.create_task(&project_path, finite_task(&task_id)?)?;

    let outcome = runner
        .run_once(
            &project_path,
            LocalAgentRunnerRequest {
                task_id: task_id.clone(),
                worker_id: worker_id.clone(),
                now_ms: TimestampMs(1_783_050_000_000),
            },
        )?
        .expect("queued task runs locally without Matrix configuration");

    let task = store.load_task(&project_path, &task_id)?.expect("submitted task remains stored");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(outcome.task_id, task_id);
    assert_eq!(outcome.worker_id, worker_id);
    assert_eq!(outcome.run.status, AgentRunStatus::Succeeded);

    let logs = store.list_task_logs(&project_path, &task_id)?;
    assert_eq!(logs.last().and_then(|log| log.to_status), Some(AgentTaskStatus::Submitted));
    assert!(logs.iter().any(|log| log.message == "local execution submitted result"));

    let artifacts = store.list_task_artifacts(&project_path, &task_id)?;
    let artifact = artifacts.first().expect("local execution artifact is readable");
    assert_eq!(artifact.kind, "execution_result");
    assert_eq!(artifact.payload["success"], true);
    assert_eq!(artifact.payload["output"], "local e2e completed without Matrix");
    assert_eq!(artifact.payload["workspace"]["slot_id"], "local-e2e-slot");

    let workspace = store
        .load_run_workspace_context(&project_path, &outcome.run.id)?
        .expect("workspace context is readable");
    assert_eq!(workspace.workspace.slot_id, "local-e2e-slot");
    assert_eq!(workspace.workspace.branch, "codex/task-local-e2e");

    let events = store.list_events_after(&project_path, None, 20)?;
    let event_kinds = events.iter().map(|event| event.kind).collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        vec![
            AgentEventKind::WorkerSelected,
            AgentEventKind::RunStarted,
            AgentEventKind::RunSubmitted,
        ]
    );
    assert!(events.iter().all(|event| event.payload["dispatch_target"] == "local"));

    let outbox = store.claim_outbox_batch(&project_path, TimestampMs(1_783_050_010_000), 10)?;
    assert!(outbox.is_empty(), "local e2e must not enqueue Matrix outbox messages");
    assert_eq!(executor.requests.lock().expect("requests lock").len(), 1);

    Ok(())
}

fn finite_task(task_id: &TaskId) -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: task_id.clone(),
        title: "Run no Matrix local e2e".to_string(),
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
        metadata: json!({"prompt": "Prove local scheduler runs without Matrix"}),
        created_at_ms: TimestampMs(1_783_049_000_000),
    })
}

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
            slot_id: "local-e2e-slot".to_string(),
            path: path.display().to_string(),
            branch: format!("codex/{}", request.task_id.as_str()),
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
