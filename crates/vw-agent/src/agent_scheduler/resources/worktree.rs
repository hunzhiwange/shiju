//! 将逻辑工位池适配为现有调度资源租约；保留历史模块和构造入口。

use anyhow::{Result, bail};
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
/// 将逻辑工位池适配为调度资源提供者。
pub struct WorktreeResourceProvider {
    pool: TaskWorktreePoolService,
    project: project::Info,
}

impl WorktreeResourceProvider {
    /// 绑定工作树池与目标项目；后续租约只能在该项目内创建。
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
        let slot = self
            .pool
            .bind_run(&self.project, &request.task_id, request.holder_id.as_str(), None)
            .await?;
        Ok(Some(lease_from_slot(request, slot)?))
    }

    async fn release(&self, lease: &ResourceLease) -> Result<()> {
        validate_worktree_lease(lease)?;
        if !self
            .pool
            .finish_run(&self.project, &lease.task_id, lease.holder_id.as_str(), None)
            .await?
        {
            bail!("worktree lease is not active for task: {}", lease.task_id.as_ref());
        }
        Ok(())
    }

    async fn recycle(&self, lease: &ResourceLease, taint_reason: Option<String>) -> Result<()> {
        validate_worktree_lease(lease)?;
        if !self
            .pool
            .finish_run(&self.project, &lease.task_id, lease.holder_id.as_str(), taint_reason)
            .await?
        {
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
        resource_kind: ResourceKind::WorkerSlot,
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
    // 释放路径只接受逻辑工位租约，避免误回收 Git 或其它资源。
    if lease.resource_kind != ResourceKind::WorkerSlot {
        bail!("resource lease is not a worktree lease");
    }
    Ok(())
}
