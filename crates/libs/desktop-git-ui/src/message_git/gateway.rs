//! Git 网关操作；客户端与租户范围由宿主注入。
use super::commit::SelectedCommitRequest;
use crate::git_panel::types::GitWorktreeOption;

pub async fn execute_selected_commit_via_gateway(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: String,
    request: SelectedCommitRequest,
) -> Result<(), String> {
    let context =
        desktop_workspace::project_message::gateway::resolve_gateway_file_context(client, tenant_scope, &project_path).await?;
    let response = gateway_project::GatewayProjectClient::new(client.transport().clone())
        .git_commit(&vw_api_git::GitCommitRequest {
            project_id: context.project_id,
            worktree_id: context.worktree_id,
            message: request.message,
            stage_all: request.stage_all,
            selected_files: request.selected_files,
            selected_hunks: request
                .selected_hunks
                .into_iter()
                .map(|(path, index)| vw_api_git::GitHunkSelectionDto {
                    path,
                    index,
                })
                .collect(),
            selected_lines: request
                .selected_lines
                .into_iter()
                .map(|(path, line)| vw_api_git::GitLineSelectionDto {
                    path,
                    line,
                })
                .collect(),
            selected_old_lines: request
                .selected_old_lines
                .into_iter()
                .map(|(path, line)| vw_api_git::GitLineSelectionDto {
                    path,
                    line,
                })
                .collect(),
        })
        .await?;
    if !response.ok {
        return Err("网关未确认提交成功".to_string());
    }
    if response.commit.sha.trim().is_empty() {
        return Err("网关返回的提交结果缺少 SHA".to_string());
    }
    Ok(())
}

pub async fn execute_stage_all_via_gateway(
    client: &vw_gateway_client::GatewayClient,
    _tenant_scope: &vw_api_project::TenantScopeDto,
    directory: String,
) -> Result<(), String> {
    let response = gateway_project::GatewayProjectClient::new(client.transport().clone())
        .git_command(&vw_api_git::GitCommandRequest {
            directory,
            args: vec!["add".to_string(), "--all".to_string()],
            timeout_secs: Some(30),
        })
        .await?;
    if response.success {
        Ok(())
    } else {
        let error = response.stderr.trim();
        Err(if error.is_empty() {
            "网关未确认 Git 暂存成功".to_string()
        } else {
            error.to_string()
        })
    }
}

pub async fn load_git_worktree_options(client: &vw_gateway_client::GatewayClient, tenant_scope: &vw_api_project::TenantScopeDto,
    project_path: String,
) -> Result<Vec<GitWorktreeOption>, String> {
    let project =
        desktop_workspace::project_message::gateway::resolve_gateway_project(client, tenant_scope, &project_path).await?;
    if !project.git.is_repo {
        return Ok(Vec::new());
    }
    let mut options = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let project_directory = super::shared::normalize_path(&project.directory);
    let main_branch = project.current_branch.clone().filter(|value| !value.trim().is_empty());
    let main_label = main_branch
        .as_ref()
        .map(|branch| format!("主工作区 · {branch}"))
        .unwrap_or_else(|| "主工作区".to_string());
    seen.insert(project_directory);
    options.push(GitWorktreeOption {
        directory: project.directory.clone(),
        label: main_label,
        branch: main_branch,
    });

    let project_scope = vw_api_project::ProjectScopeDto::new(tenant_scope.clone(), project.id.clone()).map_err(|error| error.to_string())?;
    let mut worktrees = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_worktrees(&project_scope).await?.items;
    worktrees.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.branch.cmp(&right.branch))
            .then(left.directory.cmp(&right.directory))
    });
    for worktree in worktrees {
        let key = super::shared::normalize_path(&worktree.directory);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let branch = (!worktree.branch.trim().is_empty()).then_some(worktree.branch.clone());
        options.push(GitWorktreeOption {
            directory: worktree.directory.clone(),
            label: super::shared::worktree_label(&worktree.name, &worktree.branch, &worktree.directory),
            branch,
        });
    }

    Ok(options)
}
