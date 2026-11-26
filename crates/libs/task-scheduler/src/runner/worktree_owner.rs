//! Trusted scheduler ownership, passed through the existing tool context chain.
use crate::{
    domain::{RunId, TaskId},
    store::{AgentSchedulerStore, artifacts::RecordTaskArtifactRequest},
};
use std::path::PathBuf;
use vw_api_transport::common::TimestampMs;

#[derive(Clone)]
pub struct SchedulerWorktreeOwner {
    pub store: AgentSchedulerStore,
    pub project_path: PathBuf,
    pub context: crate::domain::AgentSchedulerContext,
    pub agent_exists: fn(&str) -> anyhow::Result<bool>,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub claims: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl tool_context::run_worktree::RunWorktreePort for SchedulerWorktreeOwner {
    fn move_workflow_step(&self, return_to_previous: bool) -> anyhow::Result<serde_json::Value> {
        self.validate()?;
        let task = crate::task_board::AgentTaskBoardService::new(&self.store).move_running_task(
            &self.context, &self.task_id, &self.run_id, return_to_previous,
            TimestampMs(chrono::Utc::now().timestamp_millis().max(0)), |handoff| {
                use vw_api_task::TaskPoolWorkflowStepHandoffDto;
                match handoff {
                    TaskPoolWorkflowStepHandoffDto::KeepAssignee => {},
                    TaskPoolWorkflowStepHandoffDto::Member { member_id } => anyhow::ensure!(
                        member_id == self.context.scope().project_scope().user_id().as_ref(), "member must be current ShiJu login user"),
                    TaskPoolWorkflowStepHandoffDto::Agent { agent_id, .. } => anyhow::ensure!((self.agent_exists)(agent_id)?, "unknown workflow agent"),
                }
                Ok(())
            })?;
        Ok(serde_json::json!({"task_id": task.id.as_str(), "status_key": crate::task_board::task_board_status_key(&task)}))
    }

    fn validate(&self) -> anyhow::Result<()> {
        let run = self
            .store
            .load_run(&self.context, &self.run_id)?
            .ok_or_else(|| anyhow::anyhow!("worktree owner run not found"))?;
        anyhow::ensure!(run.task_id == self.task_id, "worktree owner task mismatch");
        anyhow::ensure!(
            run.status == crate::domain::AgentRunStatus::Running,
            "worktree owner run is not running"
        );
        Ok(())
    }

    fn claim_directory(&self, directory: &str) -> anyhow::Result<()> {
        use crate::runner::workspace_claim::{WorkspaceClaim, lock_root};
        let path = std::path::Path::new(directory).canonicalize()?;
        // The original directory claim remains held throughout execution.
        if path.starts_with(self.project_path.canonicalize()?) {
            return Ok(());
        }
        let claim = WorkspaceClaim::acquire(&lock_root()?, &path, self.run_id.as_str().into())?
            .ok_or_else(|| anyhow::anyhow!("worktree directory is occupied"))?;
        claim.retain(&path)?;
        self.claims.lock().unwrap_or_else(|e| e.into_inner()).push(path);
        Ok(())
    }

    fn retain_unconfirmed(&self) -> anyhow::Result<()> {
        use crate::runner::workspace_claim::WorkspaceClaim;
        for path in self.claims.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            if let Some(claim) = WorkspaceClaim::take_canonical(path, self.run_id.as_str())? {
                claim.retain_unconfirmed(path)?;
            }
        }
        Ok(())
    }

    fn release_directories(&self) -> anyhow::Result<()> {
        use crate::runner::workspace_claim::WorkspaceClaim;
        for path in self.claims.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
            if let Some(mut claim) = WorkspaceClaim::take_canonical(&path, self.run_id.as_str())? {
                claim.release(self.run_id.as_str())?;
            }
        }
        Ok(())
    }

    fn record(
        &self,
        binding: &tool_context::context::WorktreeBindingState,
        action: &str,
    ) -> anyhow::Result<()> {
        self.store.record_task_artifact(
            &self.context,
            RecordTaskArtifactRequest {
                task_id: self.task_id.clone(),
                run_id: Some(self.run_id.clone()),
                kind: "worktree".into(),
                name: action.into(),
                path: binding.directory.clone(),
                payload: serde_json::json!({"action": action, "binding": binding,
                "retained_until_explicit_exit": action == "entered"}),
                created_at_ms: TimestampMs(chrono::Utc::now().timestamp_millis().max(0)),
            },
        )?;
        Ok(())
    }
}

pub use tool_context::run_worktree::RunWorktreeOwner;
