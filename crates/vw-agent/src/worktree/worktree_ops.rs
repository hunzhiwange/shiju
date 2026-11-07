use super::{CreateInput, Error, Info, RemoveInput, ResetInput};

#[cfg(not(target_arch = "wasm32"))]
use super::event;
#[cfg(not(target_arch = "wasm32"))]
use super::naming::slug;
#[cfg(not(target_arch = "wasm32"))]
use super::native::{
    candidate, canonical, ensure_git_project, find_entry, parse_worktree_list, path_exists,
    resolve_reset_target, run_git, run_start_scripts, worktree_root,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::bus;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::project;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::project::instance;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

/// 创建新的 worktree（WASM 平台实现）
///
/// Web 版本不支持 worktree 功能
#[cfg(target_arch = "wasm32")]
pub async fn create(_input: Option<CreateInput>) -> Result<Info, Error> {
    Err(Error::Invalid("worktree 在 Web 版本不可用".to_string()))
}

/// 创建新的 worktree（原生平台实现）
#[cfg(not(target_arch = "wasm32"))]
pub async fn create(input: Option<CreateInput>) -> Result<Info, Error> {
    let project = ensure_git_project()?;
    let project_id = project.id.clone();
    if project_id.trim().is_empty() {
        return Err(Error::MissingProject("missing project id".to_string()));
    }

    let info =
        add_worktree(&project, input.as_ref().and_then(|input| input.name.as_deref())).await?;

    if let Ok(scope) = project::project_scope_for_default_tenant(project_id.as_str()) {
        let _ = project::add_sandbox(&scope, &info.directory).await;
    }

    let extra = input.and_then(|value| value.start_command).unwrap_or_default();
    let ready_info = info.clone();
    tokio::spawn(async move {
        let populated = run_git(&["reset", "--hard"], &ready_info.directory).await;
        if populated.as_ref().is_ok_and(|result| result.success) {
            let booted = instance::provide(
                &ready_info.directory,
                Some(Box::new({
                    let directory = ready_info.directory.clone();
                    move || {
                        Box::pin(async move {
                            project::instance_bootstrap(PathBuf::from(directory)).await;
                        })
                    }
                })),
                || Box::pin(async {}),
            )
            .await;

            if let Err(err) = booted {
                let _ = bus::publish(
                    event::FAILED,
                    serde_json::json!({ "message": err.to_string() }),
                    Some(ready_info.directory.clone()),
                );
                return;
            }

            let _ = bus::publish(
                event::READY,
                serde_json::json!({ "name": ready_info.name, "branch": ready_info.branch }),
                Some(ready_info.directory.clone()),
            );

            let _ = run_start_scripts(&ready_info.directory, &project_id, &extra).await;
        } else {
            let message = populated
                .err()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "Failed to populate worktree".to_string());
            let _ = bus::publish(
                event::FAILED,
                serde_json::json!({ "message": message }),
                Some(ready_info.directory.clone()),
            );
        }
    });

    Ok(info)
}

#[cfg(not(target_arch = "wasm32"))]
async fn add_worktree(project: &project::Info, name: Option<&str>) -> Result<Info, Error> {
    let root = worktree_root(project)?;
    tokio::fs::create_dir_all(&root).await?;
    let base = name.map(slug).filter(|base| !base.trim().is_empty());
    let info = candidate(&root, base.as_deref()).await?;

    let primary = instance::worktree();
    if primary.trim().is_empty() {
        return Err(Error::MissingProject("missing instance worktree".to_string()));
    }
    let created = run_git(
        &["worktree", "add", "--no-checkout", "-b", &info.branch, &info.directory],
        &primary,
    )
    .await?;
    if !created.success {
        return Err(Error::Invalid(created.error_text("Failed to create git worktree")));
    }
    Ok(info)
}

/// Prepare a checked-out worktree before a fork can be published.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn create_fork_checkout(name: Option<&str>) -> Result<Info, Error> {
    let project = ensure_git_project()?;
    let info = add_worktree(&project, name).await?;
    let populated = run_git(&["reset", "--hard"], &info.directory).await.and_then(|result| {
        if result.success {
            Ok(())
        } else {
            Err(Error::Invalid(result.error_text("Failed to populate worktree")))
        }
    });
    if let Err(error) = populated {
        if let Err(cleanup) = remove_fork_checkout(info.directory).await {
            tracing::warn!(%cleanup, "failed to clean up a fork checkout");
        }
        return Err(error);
    }
    Ok(info)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn remove_fork_checkout(directory: String) -> Result<(), Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    remove_for_tenant(RemoveInput { directory, force: true }, tenant).await?;
    Ok(())
}

/// 删除 worktree（WASM 平台实现）
///
/// Web 版本不支持 worktree 功能
#[cfg(target_arch = "wasm32")]
pub async fn remove(_input: RemoveInput) -> Result<bool, Error> {
    Err(Error::Invalid("worktree 在 Web 版本不可用".to_string()))
}

/// 删除 worktree（原生平台实现）
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove(input: RemoveInput) -> Result<bool, Error> {
    remove_for_tenant(input, project::default_tenant_scope()).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn remove_for_tenant(
    input: RemoveInput,
    tenant: vw_api_types::id::TenantScope,
) -> Result<bool, Error> {
    use vw_api_types::id::{ProjectId, ProjectScope};
    let project = ensure_git_project()?;
    let project_id = project.id.clone();
    let primary = instance::worktree();

    let directory = canonical(&input.directory).await;
    let list = run_git(&["worktree", "list", "--porcelain"], &primary).await?;
    if !list.success {
        return Err(Error::Invalid(list.error_text("Failed to read git worktrees")));
    }
    let entries = parse_worktree_list(&list.stdout);

    let entry = find_entry(&entries, &directory).await;
    let Some(entry) = entry else {
        if path_exists(&directory).await {
            let _ = tokio::fs::remove_dir_all(&directory).await;
        }
        return Ok(true);
    };

    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if input.force {
        args.push("--force");
    }
    args.push(&entry.path);
    let removed = run_git(&args, &primary).await?;
    if !removed.success {
        return Err(Error::Invalid(removed.error_text("Failed to remove git worktree")));
    }

    if let Some(branch) =
        entry.branch.as_deref().and_then(|value| value.strip_prefix("refs/heads/"))
    {
        let deleted = run_git(&["branch", "-D", branch], &primary).await?;
        if !deleted.success {
            return Err(Error::Invalid(deleted.error_text("Failed to delete worktree branch")));
        }
    }

    if let Ok(scope) = ProjectScope::new(tenant, ProjectId::from(project_id.as_str())) {
        let _ = project::remove_sandbox(&scope, &entry.path).await;
    }
    Ok(true)
}

/// 列出所有 worktree 目录（WASM 平台实现）
///
/// Web 版本不支持 worktree 功能
#[cfg(target_arch = "wasm32")]
pub async fn list_directories() -> Result<Vec<String>, Error> {
    Err(Error::Invalid("worktree 在 Web 版本不可用".to_string()))
}

/// 列出所有 worktree 目录（原生平台实现）
#[cfg(not(target_arch = "wasm32"))]
pub async fn list_directories() -> Result<Vec<String>, Error> {
    let _project = ensure_git_project()?;
    let primary = instance::worktree();
    if primary.trim().is_empty() {
        return Err(Error::MissingProject("missing instance worktree".to_string()));
    }
    // Git API 的 worktree 校验同样使用内置库，避免间接依赖系统 Git。
    tokio::task::spawn_blocking(move || -> Result<Vec<String>, Error> {
        let read = || -> Result<Vec<String>, git2::Error> {
            let repository = git2::Repository::open(&primary)?;
            let common = git2::Repository::open(repository.commondir())?;
            let root = common
                .workdir()
                .ok_or_else(|| git2::Error::from_str("worktree requires a non-bare repository"))?;
            let normalize = |path: &std::path::Path| {
                std::fs::canonicalize(path)
                    .unwrap_or_else(|_| path.to_path_buf())
                    .to_string_lossy()
                    .into_owned()
            };
            let mut directories = vec![normalize(root)];
            for name in common.worktrees()?.iter() {
                if let Some(name) = name? {
                    directories.push(normalize(common.find_worktree(name)?.path()));
                }
            }
            Ok(directories)
        };
        read().map_err(|error| Error::Invalid(error.to_string()))
    })
    .await?
}

/// 重置 worktree 到指定状态（WASM 平台实现）
///
/// Web 版本不支持 worktree 功能
#[cfg(target_arch = "wasm32")]
pub async fn reset(_input: ResetInput) -> Result<bool, Error> {
    Err(Error::Invalid("worktree 在 Web 版本不可用".to_string()))
}

/// 重置 worktree 到指定状态（原生平台实现）
#[cfg(not(target_arch = "wasm32"))]
pub async fn reset(input: ResetInput) -> Result<bool, Error> {
    let project = ensure_git_project()?;
    let project_id = project.id.clone();
    let primary = canonical(&instance::worktree()).await;
    let directory = canonical(&input.directory).await;

    if directory == primary {
        return Err(Error::Invalid("Cannot reset the primary workspace".to_string()));
    }

    let primary_s = primary.to_string_lossy().to_string();
    let list = run_git(&["worktree", "list", "--porcelain"], &primary_s).await?;
    if !list.success {
        return Err(Error::Invalid(list.error_text("Failed to read git worktrees")));
    }
    let entries = parse_worktree_list(&list.stdout);
    let entry = find_entry(&entries, &directory)
        .await
        .ok_or_else(|| Error::Invalid("Worktree not found".to_string()))?;

    let target = resolve_reset_target(&primary, input.base_ref.as_deref()).await?;

    if let Some((remote, branch)) = target.remote.as_deref().zip(target.remote_branch.as_deref()) {
        let fetched = run_git(&["fetch", remote, branch], &primary_s).await?;
        if !fetched.success {
            return Err(Error::Invalid(
                fetched.error_text(&format!("Failed to fetch {}/{}", remote, branch)),
            ));
        }
    }

    let reset = run_git(&["reset", "--hard", &target.target], &entry.path).await?;
    if !reset.success {
        return Err(Error::Invalid(reset.error_text("Failed to reset worktree to target")));
    }

    let clean = run_git(&["clean", "-fd"], &entry.path).await?;
    if !clean.success {
        return Err(Error::Invalid(clean.error_text("Failed to clean worktree")));
    }

    let update =
        run_git(&["submodule", "update", "--init", "--recursive", "--force"], &entry.path).await?;
    if !update.success {
        return Err(Error::Invalid(update.error_text("Failed to update submodules")));
    }

    let sub_reset =
        run_git(&["submodule", "foreach", "--recursive", "git", "reset", "--hard"], &entry.path)
            .await?;
    if !sub_reset.success {
        return Err(Error::Invalid(sub_reset.error_text("Failed to reset submodules")));
    }

    let sub_clean =
        run_git(&["submodule", "foreach", "--recursive", "git", "clean", "-fdx"], &entry.path)
            .await?;
    if !sub_clean.success {
        return Err(Error::Invalid(sub_clean.error_text("Failed to clean submodules")));
    }

    let status = run_git(&["status", "--porcelain=v1"], &entry.path).await?;
    if !status.success {
        return Err(Error::Invalid(status.error_text("Failed to read git status")));
    }
    if !status.stdout.trim().is_empty() {
        return Err(Error::Invalid(format!(
            "Worktree reset left local changes:\n{}",
            status.stdout.trim()
        )));
    }

    let worktree_dir = entry.path.clone();
    tokio::spawn(async move {
        let _ = run_start_scripts(&worktree_dir, &project_id, "").await;
    });

    Ok(true)
}
