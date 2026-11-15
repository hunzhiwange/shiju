//! 项目与 worktree API。
//!
//! 本模块封装网关提供的项目管理能力，包含：
//! - 项目列表查询与项目解析
//! - 单个项目详情读取与更新
//! - 变更记录查询
//! - worktree 的创建、读取、删除与重置

use vw_api_transport::common::OperationAck;
use vw_api_types::project::{
    GetProjectResponse, ListProjectChangeRecordsResponse, ListProjectsRequest,
    ListProjectsResponse, ProjectScopeDto, ResolveProjectRequest, ResolveProjectResponse,
    TenantScopeDto, UpdateProjectRequest,
};
use vw_api_worktree::{
    CreateWorktreeRequest, CreateWorktreeResponse, DeleteWorktreeRequest, GetWorktreeResponse,
    ListWorktreesResponse, ResetWorktreeRequest,
};

use super::GatewayClient;

pub(super) use gateway_project::encode_path_segment;

impl GatewayClient {
    pub async fn project_list(
        &self,
        scope: &TenantScopeDto,
        request: &ListProjectsRequest,
    ) -> Result<ListProjectsResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_list(scope, request).await
    }

    pub async fn project_resolve(
        &self,
        scope: &TenantScopeDto,
        request: &ResolveProjectRequest,
    ) -> Result<ResolveProjectResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_resolve(scope, request).await
    }

    pub async fn project_get(&self, scope: &ProjectScopeDto) -> Result<GetProjectResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_get(scope).await
    }

    pub async fn project_change_records(
        &self,
        scope: &ProjectScopeDto,
        directory: &str,
    ) -> Result<ListProjectChangeRecordsResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_change_records(scope, directory).await
    }

    pub async fn project_update(
        &self,
        scope: &ProjectScopeDto,
        request: &UpdateProjectRequest,
    ) -> Result<GetProjectResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_update(scope, request).await
    }

    pub async fn project_worktrees(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListWorktreesResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_worktrees(scope).await
    }

    pub async fn project_worktree_create(
        &self,
        scope: &ProjectScopeDto,
        request: &CreateWorktreeRequest,
    ) -> Result<CreateWorktreeResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .project_worktree_create(scope, request).await
    }

    pub async fn worktree_get(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
    ) -> Result<GetWorktreeResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_get(scope, worktree_id).await
    }

    pub async fn worktree_delete(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
        request: &DeleteWorktreeRequest,
    ) -> Result<OperationAck, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_delete(scope, worktree_id, request).await
    }

    pub async fn worktree_reset(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
        request: &ResetWorktreeRequest,
    ) -> Result<OperationAck, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_reset(scope, worktree_id, request).await
    }
}
