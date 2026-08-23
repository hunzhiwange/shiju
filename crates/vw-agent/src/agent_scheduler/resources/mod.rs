//! 定义调度资源的申请与租约契约，具体资源类型通过 `ResourceProvider` 接入。

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use vw_api_types::id::TaskId;

use crate::agent_scheduler::domain::{HolderId, ResourceId};
use crate::agent_scheduler::store::resources::ResourceKind;

pub mod worktree;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 资源提供者接收的任务、资源种类与租期请求。
pub struct AcquireResourceRequest {
    pub holder_id: HolderId,
    pub task_id: TaskId,
}

#[derive(Debug, Clone, PartialEq)]
/// 提供者返回的资源租约和实现专属元数据。
pub struct ResourceLease {
    pub resource_kind: ResourceKind,
    pub resource_id: ResourceId,
    pub holder_id: HolderId,
    pub task_id: TaskId,
    pub metadata_json: Value,
}

#[async_trait]
/// 调度器申请和释放外部资源的同步抽象。
pub trait ResourceProvider {
    async fn acquire(&self, request: AcquireResourceRequest) -> Result<Option<ResourceLease>>;

    async fn release(&self, lease: &ResourceLease) -> Result<()>;

    async fn recycle(&self, lease: &ResourceLease, taint_reason: Option<String>) -> Result<()>;
}

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod worktree_tests;
