//! 项目与 worktree API。
//!
//! 本模块封装网关提供的项目管理能力，包含：
//! - 项目列表查询与项目解析
//! - 单个项目详情读取与更新
//! - 变更记录查询
//! - worktree 的创建、读取、删除与重置

use vw_api_types::common::OperationAck;
use vw_api_types::project::{
    GetProjectResponse, ListProjectChangeRecordsResponse, ListProjectsRequest,
    ListProjectsResponse, ProjectScopeDto, ResolveProjectRequest, ResolveProjectResponse,
    TenantScopeDto, UpdateProjectRequest,
};
use vw_api_types::worktree::{
    CreateWorktreeRequest, CreateWorktreeResponse, DeleteWorktreeRequest, GetWorktreeResponse,
    ListWorktreesResponse, ResetWorktreeRequest,
};

use super::GatewayClient;

impl GatewayClient {
    /// 分页查询项目列表，并支持按关键字与状态过滤。
    ///
    /// 调用方传入的请求结构会被转换为查询参数；其中状态字段会被序列化为后端接受的字符串形式。
    pub async fn project_list(
        &self,
        scope: &TenantScopeDto,
        request: &ListProjectsRequest,
    ) -> Result<ListProjectsResponse, String> {
        self.ensure_tenant_scope(scope)?;
        let mut query = Vec::new();
        if let Some(cursor) = request.cursor.as_ref() {
            query.push(("cursor".to_string(), cursor.clone()));
        }
        if let Some(limit) = request.limit {
            query.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(search) = request.query.as_ref() {
            query.push(("query".to_string(), search.clone()));
        }
        if let Some(status) = request.status.as_ref() {
            query.push(("status".to_string(), serde_json::to_string(status).unwrap_or_default()));
            if let Some((_, value)) = query.last_mut() {
                *value = value.trim_matches('"').to_string();
            }
        }
        self.get_json("/v1/projects", &query).await
    }

    /// 根据目录等线索解析项目身份。
    ///
    /// 常用于调用方只持有目录路径，但尚未拿到稳定 `project_id` 的场景。
    pub async fn project_resolve(
        &self,
        scope: &TenantScopeDto,
        request: &ResolveProjectRequest,
    ) -> Result<ResolveProjectResponse, String> {
        self.ensure_tenant_scope(scope)?;
        self.post_json("/v1/projects/resolve", &[], request).await
    }

    /// 读取单个项目详情。
    ///
    /// `project_id` 必须是网关已登记项目的稳定标识。
    pub async fn project_get(&self, scope: &ProjectScopeDto) -> Result<GetProjectResponse, String> {
        self.get_json(&self.project_scope_path(scope)?, &[]).await
    }

    /// 获取指定目录关联项目的变更记录列表。
    pub async fn project_change_records(
        &self,
        scope: &ProjectScopeDto,
        directory: &str,
    ) -> Result<ListProjectChangeRecordsResponse, String> {
        self.get_json(
            &format!("{}/change-records", self.project_scope_path(scope)?),
            &[("directory".to_string(), directory.to_string())],
        )
        .await
    }

    /// 更新项目元数据。
    ///
    /// 该接口采用 PATCH 语义，只更新请求体中显式提供的字段。
    pub async fn project_update(
        &self,
        scope: &ProjectScopeDto,
        request: &UpdateProjectRequest,
    ) -> Result<GetProjectResponse, String> {
        self.patch_json(&self.project_scope_path(scope)?, &[], request).await
    }

    /// 列出项目下的全部 worktree。
    pub async fn project_worktrees(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListWorktreesResponse, String> {
        self.get_json(&format!("{}/worktrees", self.project_scope_path(scope)?), &[]).await
    }

    /// 为指定项目创建新的 worktree。
    ///
    /// 返回值中包含新建 worktree 的完整描述信息。
    pub async fn project_worktree_create(
        &self,
        scope: &ProjectScopeDto,
        request: &CreateWorktreeRequest,
    ) -> Result<CreateWorktreeResponse, String> {
        self.post_json(&format!("{}/worktrees", self.project_scope_path(scope)?), &[], request)
            .await
    }

    /// 读取单个 worktree 详情。
    pub async fn worktree_get(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
    ) -> Result<GetWorktreeResponse, String> {
        self.get_json(&self.worktree_path(scope, worktree_id)?, &[]).await
    }

    /// 删除指定 worktree。
    pub async fn worktree_delete(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
        request: &DeleteWorktreeRequest,
    ) -> Result<OperationAck, String> {
        self.delete_json(&self.worktree_path(scope, worktree_id)?, &[], request).await
    }

    /// 将指定 worktree 重置到目标状态。
    ///
    /// 具体重置策略由请求体中的参数决定，例如是否硬重置、是否清理未跟踪文件等。
    pub async fn worktree_reset(
        &self,
        scope: &ProjectScopeDto,
        worktree_id: &str,
        request: &ResetWorktreeRequest,
    ) -> Result<OperationAck, String> {
        self.post_json(&format!("{}/reset", self.worktree_path(scope, worktree_id)?), &[], request)
            .await
    }

    fn ensure_tenant_scope(&self, scope: &TenantScopeDto) -> Result<(), String> {
        if scope != self.endpoint().tenant_scope() {
            return Err("tenant scope does not match gateway endpoint tenant scope".to_string());
        }
        Ok(())
    }

    fn project_scope_path(&self, scope: &ProjectScopeDto) -> Result<String, String> {
        self.ensure_tenant_scope(scope.tenant_scope())?;
        Ok(format!("/v1/projects/{}", encode_path_segment(scope.project_id().as_ref())))
    }

    fn worktree_path(&self, scope: &ProjectScopeDto, worktree_id: &str) -> Result<String, String> {
        Ok(format!(
            "{}/worktrees/{}",
            self.project_scope_path(scope)?,
            encode_path_segment(worktree_id)
        ))
    }
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "project_api_tests.rs"]
mod project_api_tests;
