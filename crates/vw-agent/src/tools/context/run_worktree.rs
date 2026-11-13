//! Trusted scheduler ownership, passed through the existing tool context chain.
use crate::agent_scheduler::{
    domain::{RunId, TaskId},
    store::{AgentSchedulerStore, artifacts::RecordTaskArtifactRequest},
};
use std::path::PathBuf;
use vw_api_types::common::TimestampMs;

#[derive(Clone)]
pub(crate) struct RunWorktreeOwner {
    pub store: AgentSchedulerStore,
    pub project_path: PathBuf,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub claims: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl RunWorktreeOwner {
    pub fn validate(&self) -> anyhow::Result<()> {
        let run = self
            .store
            .load_run(&self.project_path, &self.run_id)?
            .ok_or_else(|| anyhow::anyhow!("worktree owner run not found"))?;
        anyhow::ensure!(run.task_id == self.task_id, "worktree owner task mismatch");
        anyhow::ensure!(
            run.status == crate::agent_scheduler::domain::AgentRunStatus::Running,
            "worktree owner run is not running"
        );
        Ok(())
    }

    pub fn claim_directory(&self, directory: &str) -> anyhow::Result<()> {
        use crate::agent_scheduler::runner::workspace_claim::{WorkspaceClaim, lock_root};
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

    pub fn retain_unconfirmed(&self) -> anyhow::Result<()> {
        use crate::agent_scheduler::runner::workspace_claim::WorkspaceClaim;
        for path in self.claims.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            if let Some(claim) = WorkspaceClaim::take_canonical(path, self.run_id.as_str())? {
                claim.retain_unconfirmed(path)?;
            }
        }
        Ok(())
    }

    pub fn release_directories(&self) -> anyhow::Result<()> {
        use crate::agent_scheduler::runner::workspace_claim::WorkspaceClaim;
        for path in self.claims.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
            if let Some(mut claim) = WorkspaceClaim::take_canonical(&path, self.run_id.as_str())? {
                claim.release(self.run_id.as_str())?;
            }
        }
        Ok(())
    }

    pub fn record(
        &self,
        binding: &super::WorktreeBindingState,
        action: &str,
    ) -> anyhow::Result<()> {
        self.store.record_task_artifact(
            &self.project_path,
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
