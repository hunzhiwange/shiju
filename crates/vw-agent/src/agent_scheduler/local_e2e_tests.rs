//! 测试本地调度从任务入队到运行完成的端到端流程。
//! 端到端夹具保留真实领取与完成路径，同时以临时资源隔离外部环境。

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
