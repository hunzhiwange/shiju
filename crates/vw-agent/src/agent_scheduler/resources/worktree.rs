use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use serde_json::json;
use vw_api_types::id::TaskId;
use vw_api_types::task::TaskPoolWorktreeSlotDto;

use crate::agent_scheduler::domain::ResourceId;
use crate::agent_scheduler::resources::{AcquireResourceRequest, ResourceLease, ResourceProvider};
use crate::agent_scheduler::store::resources::ResourceKind;
use crate::app::agent::project;
use crate::task_worktree_pool::TaskWorktreePoolService;

#[derive(Clone)]
pub struct WorktreeResourceProvider {
    pool: TaskWorktreePoolService,
    project: project::Info,
}

impl WorktreeResourceProvider {
    pub fn new(pool: TaskWorktreePoolService, project: project::Info) -> Self {
        Self { pool, project }
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &TaskWorktreePoolService {
        &self.pool
    }
}

#[async_trait]
impl ResourceProvider for WorktreeResourceProvider {
    async fn acquire(&self, request: AcquireResourceRequest) -> Result<Option<ResourceLease>> {
        let response = self.pool.acquire_and_prepare(&self.project, &request.task_id).await?;
        if !response.leased {
            return Ok(None);
        }
        let slot = response.slot.context("worktree pool reported a lease without a slot")?;
        Ok(Some(lease_from_slot(request, slot)?))
    }

    async fn release(&self, lease: &ResourceLease) -> Result<()> {
        validate_worktree_lease(lease)?;
        let response = self.pool.release_task_slot(&self.project, &lease.task_id).await?;
        if !response.released {
            bail!("worktree lease is not active for task: {}", lease.task_id.as_ref());
        }
        Ok(())
    }

    async fn recycle(&self, lease: &ResourceLease, taint_reason: Option<String>) -> Result<()> {
        validate_worktree_lease(lease)?;
        let response =
            self.pool.recycle_task_slot(&self.project, &lease.task_id, taint_reason).await?;
        if !response.recycled {
            bail!("worktree lease is not active for task: {}", lease.task_id.as_ref());
        }
        Ok(())
    }
}

pub(super) fn lease_from_slot(
    request: AcquireResourceRequest,
    slot: TaskPoolWorktreeSlotDto,
) -> Result<ResourceLease> {
    Ok(ResourceLease {
        resource_kind: ResourceKind::Worktree,
        resource_id: ResourceId::new(slot.id.clone())?,
        holder_id: request.holder_id,
        task_id: leased_task_id(slot.leased_task_id, &request.task_id)?,
        metadata_json: json!({
            "slot_id": slot.id,
            "path": slot.path,
            "branch": slot.branch,
        }),
    })
}

fn leased_task_id(slot_task_id: Option<TaskId>, request_task_id: &TaskId) -> Result<TaskId> {
    match slot_task_id {
        Some(task_id) if task_id == *request_task_id => Ok(task_id),
        Some(task_id) => {
            bail!(
                "worktree pool leased task mismatch: requested {}, got {}",
                request_task_id.as_ref(),
                task_id.as_ref()
            )
        }
        None => bail!("worktree pool returned a slot without a leased task"),
    }
}

fn validate_worktree_lease(lease: &ResourceLease) -> Result<()> {
    if lease.resource_kind != ResourceKind::Worktree {
        bail!("resource lease is not a worktree lease");
    }
    Ok(())
}
