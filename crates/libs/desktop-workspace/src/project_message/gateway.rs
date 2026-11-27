use vw_api_transport::id::{ProjectId, WorktreeId};
use vw_api_file::file::{CopyFileRequest, DeleteFileRequest, DuplicateFileRequest, MoveFileRequest};
use vw_api_project::{ProjectDto, ResolveProjectRequest};
use vw_api_worktree::{CreateWorktreeRequest, DeleteWorktreeRequest, ResetMode, ResetWorktreeRequest, WorktreeDto};
use super::paths::relative_to_project;
use super::data::normalize_path;
fn project_scope(tenant: &vw_api_project::TenantScopeDto, project: ProjectId) -> Result<vw_api_project::ProjectScopeDto, String> {
 vw_api_project::ProjectScopeDto::new(tenant.clone(), project).map_err(|error| error.to_string())
}
#[derive(Debug, Clone)]
/// GatewayFileContext 保存该流程中跨函数传递的结构化数据。
///
/// 使用具名字段保留领域含义，避免在消息链路中传递松散的动态数据。
pub struct GatewayFileContext {
    pub project_id: ProjectId,
    pub worktree_id: Option<WorktreeId>,
    pub base_directory: String,
}
/// Resolve a selected checkout without falling back to another directory.
pub async fn resolve_checkout_file_context(
    client: &vw_gateway_client::GatewayClient,
    project_id: ProjectId,
    project_directory: &str,
    directory: &str,
) -> Result<GatewayFileContext, String> {
    let worktree_id = if normalize_path(project_directory) == normalize_path(directory) {
        None
    } else {
        let scope = project_scope(client.endpoint().tenant_scope(), project_id.clone())?;
        let worktree = gateway_project::GatewayProjectClient::new(client.transport().clone())
            .project_worktrees(&scope)
            .await?
            .items
            .into_iter()
            .find(|worktree| normalize_path(&worktree.directory) == normalize_path(directory))
            .ok_or_else(|| "工作区不存在".to_string())?;
        Some(worktree.id)
    };
    Ok(GatewayFileContext { project_id, worktree_id, base_directory: directory.to_string() })
}

pub async fn resolve_gateway_file_context(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
) -> Result<GatewayFileContext, String> {
    let project = resolve_gateway_project(client, tenant_scope, project_path).await?;
    let current_directory = normalize_path(project_path);
    let project_directory = normalize_path(&project.directory);
    if current_directory == project_directory {
        return Ok(GatewayFileContext {
            project_id: project.id,
            worktree_id: None,
            base_directory: project.directory,
        });
    }

    let project_scope = project_scope(tenant_scope, project.id.clone())?;
    let maybe_worktree = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktrees(&project_scope)
        .await?
        .items
        .into_iter()
        .find(|worktree| normalize_path(&worktree.directory) == current_directory);
    if let Some(worktree) = maybe_worktree {
        return Ok(GatewayFileContext {
            project_id: project.id,
            worktree_id: Some(worktree.id),
            base_directory: worktree.directory,
        });
    }

    Ok(GatewayFileContext {
        project_id: project.id,
        worktree_id: None,
        base_directory: project_path.to_string(),
    })
}

pub async fn resolve_gateway_project(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto, project_path: &str) -> Result<ProjectDto, String> {
    resolve_gateway_project_with_client(client, tenant_scope, project_path, true).await
}

async fn resolve_gateway_project_with_client(
    client: &vw_gateway_client::GatewayClient,
    tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    create_if_missing: bool,
) -> Result<ProjectDto, String> {
    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
            tenant_scope,
            &ResolveProjectRequest { directory: project_path.to_string(), create_if_missing },
        )
        .await
        .map(|response| response.project)
}

pub async fn list_gateway_project_sessions(
    client: &vw_gateway_client::GatewayClient,
    project_path: &str,
) -> Result<Vec<session_info::Info>, String> {
    let Ok(project) = resolve_gateway_project_with_client(client, client.endpoint().tenant_scope(), project_path, false).await else {
        return gateway_session::GatewaySessionClient::new(client.transport().clone()).session_list(Some(project_path)).await;
    };
    let mut sessions: Vec<session_info::Info> =
        gateway_session::GatewaySessionClient::new(client.transport().clone()).session_list_for_project(&project.id).await?;
    // Older gateways may ignore the project query parameter.
    sessions.retain(|info| info.project_id == project.id.0);
    Ok(sessions)
}

pub async fn gateway_session_project_path(
    client: &vw_gateway_client::GatewayClient,
    info: &session_info::Info,
) -> Result<String, String> {
    let scope = vw_api_transport::ProjectScope::new(
        client.endpoint().tenant_scope().clone(),
        info.project_id.as_str().into(),
    )
    .map_err(|error| error.to_string())?;
    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_get(&scope).await.map(|response| response.project.directory)
}

pub async fn load_project_worktree_picker_options(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
) -> Result<Vec<(String, String)>, String> {
    let project = resolve_gateway_project(client, tenant_scope, project_path).await?;
    let project_scope = project_scope(tenant_scope, project.id.clone())?;
    let mut worktrees = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktrees(&project_scope).await?.items;
    worktrees.sort_by(|left, right| {
        left.name.cmp(&right.name).then(left.directory.cmp(&right.directory))
    });

    let mut options = vec![(project.directory.clone(), "主工作区".to_string())];
    let mut seen = std::collections::HashSet::new();
    seen.insert(normalize_path(&project.directory));

    for worktree in worktrees {
        let key = normalize_path(&worktree.directory);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let label = if worktree.name.trim().is_empty() {
            let name = std::path::Path::new(&worktree.directory)
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_default();
            name.to_owned()
        } else {
            worktree.name
        };
        options.push((worktree.directory, label));
    }

    options.push(("__create_worktree__".to_string(), "创建新的独立工作区".to_string()));
    Ok(options)
}

pub async fn create_gateway_session_worktree(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: String,
    requested_name: String,
) -> Result<String, String> {
    let project = resolve_gateway_project(client, tenant_scope, &project_path).await?;
    let project_scope = project_scope(tenant_scope, project.id.clone())?;
    let worktree = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktree_create(
            &project_scope,
            &CreateWorktreeRequest {
                name: requested_name.clone(),
                branch: format!("shiju/{requested_name}"),
                from_ref: None,
                checkout: true,
            },
        )
        .await?
        .worktree;
    Ok(worktree.directory)
}

async fn find_gateway_worktree(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto, project_path: &str, directory: &str) -> Result<WorktreeDto, String> {
    let project = resolve_gateway_project(client, tenant_scope, project_path).await?;
    let project_scope = project_scope(tenant_scope, project.id.clone())?;
    let target = normalize_path(directory);
    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktrees(&project_scope)
        .await?
        .items
        .into_iter()
        .find(|worktree| normalize_path(&worktree.directory) == target)
        .ok_or_else(|| "工作区不存在".to_string())
}

pub async fn delete_gateway_worktree(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    directory: &str,
    force: bool,
) -> Result<(), String> {
    let worktree = find_gateway_worktree(client, tenant_scope, project_path, directory).await?;
    let project_scope =
        project_scope(tenant_scope, worktree.project_id.clone())?;
    let ack = gateway_project::GatewayProjectClient::new(client.transport().clone()).worktree_delete(&project_scope, &worktree.id.0, &DeleteWorktreeRequest { force })
        .await?;
    if ack.ok {
        Ok(())
    } else {
        Err(ack.message.unwrap_or_else(|| "删除工作区失败".to_string()))
    }
}

pub async fn reset_gateway_worktree(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    directory: &str,
) -> Result<(), String> {
    let worktree = find_gateway_worktree(client, tenant_scope, project_path, directory).await?;
    let project_scope =
        project_scope(tenant_scope, worktree.project_id.clone())?;
    let ack = gateway_project::GatewayProjectClient::new(client.transport().clone()).worktree_reset(
            &project_scope,
            &worktree.id.0,
            &ResetWorktreeRequest { mode: ResetMode::Hard, target_ref: None },
        )
        .await?;
    if ack.ok {
        Ok(())
    } else {
        Err(ack.message.unwrap_or_else(|| "重置工作区失败".to_string()))
    }
}

pub async fn gateway_import_local_paths(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    target_directory: &str,
    source_paths: Vec<String>,
) -> Result<vw_api_file::file::LocalImportResponse, String> {
    if !client.endpoint().is_loopback() {
        return Err("本机导入不支持远程网关".into());
    }
    let context = resolve_gateway_file_context(client, tenant_scope, project_path).await?;
    let directory = relative_to_project(&context.base_directory, target_directory)
        .ok_or_else(|| "无法定位目标目录".to_string())?;
    gateway_files::GatewayFilesClient::new(client.transport().clone())
        .file_import_local(&vw_api_file::file::LocalImportRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            target_directory: if directory == "." { String::new() } else { directory },
            source_paths,
        })
        .await
}

pub async fn gateway_move_path(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    resolve_gateway_file_context(client, tenant_scope, project_path)
        .await?
        .move_path(client, from_path, to_path)
        .await
}

pub async fn gateway_copy_path(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    from_path: &str,
    to_path: &str,
) -> Result<(), String> {
    resolve_gateway_file_context(client, tenant_scope, project_path)
        .await?
        .copy_path(client, from_path, to_path)
        .await
}

pub async fn gateway_duplicate_path(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto, project_path: &str, path: &str) -> Result<(), String> {
    resolve_gateway_file_context(client, tenant_scope, project_path)
        .await?
        .duplicate_path(client, path)
        .await
}

pub async fn gateway_delete_path(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: &str,
    path: &str,
    recursive: bool,
) -> Result<(), String> {
    resolve_gateway_file_context(client, tenant_scope, project_path)
        .await?
        .delete_path(client, path, recursive)
        .await
}

impl GatewayFileContext {
    pub async fn move_path(
        &self,
        client: &vw_gateway_client::GatewayClient,
        from_path: &str,
        to_path: &str,
    ) -> Result<(), String> {
        let from_relative = relative_to_project(&self.base_directory, from_path)
            .ok_or_else(|| "无法定位源路径".to_string())?;
        let to_relative = relative_to_project(&self.base_directory, to_path)
            .ok_or_else(|| "无法定位目标路径".to_string())?;
        let result = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_move(&MoveFileRequest {
                project_id: self.project_id.clone(),
                worktree_id: self.worktree_id.clone(),
                from_path: from_relative,
                to_path: to_relative,
                overwrite: false,
            })
            .await?;
        if result.ok {
            Ok(())
        } else {
            Err(result.message.unwrap_or_else(|| "移动失败".to_string()))
        }
    }

    pub async fn copy_path(
        &self,
        client: &vw_gateway_client::GatewayClient,
        from_path: &str,
        to_path: &str,
    ) -> Result<(), String> {
        let from_relative = relative_to_project(&self.base_directory, from_path)
            .ok_or_else(|| "无法定位源路径".to_string())?;
        let to_relative = relative_to_project(&self.base_directory, to_path)
            .ok_or_else(|| "无法定位目标路径".to_string())?;
        let result = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_copy(&CopyFileRequest {
                project_id: self.project_id.clone(),
                worktree_id: self.worktree_id.clone(),
                from_path: from_relative,
                to_path: to_relative,
                overwrite: false,
            })
            .await?;
        if result.ok {
            Ok(())
        } else {
            Err(result.message.unwrap_or_else(|| "复制失败".to_string()))
        }
    }

    pub async fn duplicate_path(
        &self,
        client: &vw_gateway_client::GatewayClient,
        path: &str,
    ) -> Result<(), String> {
        let path = relative_to_project(&self.base_directory, path)
            .ok_or_else(|| "无法定位路径".to_string())?;
        let result = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_duplicate(&DuplicateFileRequest {
                project_id: self.project_id.clone(),
                worktree_id: self.worktree_id.clone(),
                path,
            })
            .await?;
        if result.ok {
            Ok(())
        } else {
            Err(result.message.unwrap_or_else(|| "复制失败".to_string()))
        }
    }

    pub async fn delete_path(
        &self,
        client: &vw_gateway_client::GatewayClient,
        path: &str,
        recursive: bool,
    ) -> Result<(), String> {
        let relative = relative_to_project(&self.base_directory, path)
            .ok_or_else(|| "无法定位路径".to_string())?;
        let result = gateway_files::GatewayFilesClient::new(client.transport().clone())
            .file_delete(&DeleteFileRequest {
                project_id: self.project_id.clone(),
                worktree_id: self.worktree_id.clone(),
                path: relative,
                recursive,
            })
            .await?;
        if result.ok {
            Ok(())
        } else {
            Err(result.message.unwrap_or_else(|| "删除失败".to_string()))
        }
    }
}

pub async fn load_recent_projects(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto) -> Result<Vec<ProjectDto>, String> {
    let client = gateway_project::GatewayProjectClient::new(client.transport().clone());
    client.project_list_all(tenant_scope).await
}
