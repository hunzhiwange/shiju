use super::{CreateInput, Error, Info, RemoveInput, ResetInput};

#[cfg(not(target_arch = "wasm32"))]
use super::event;
#[cfg(not(target_arch = "wasm32"))]
use super::naming::slug;
#[cfg(not(target_arch = "wasm32"))]
use super::native::{
    candidate, canonical, ensure_git_project, find_entry, parse_worktree_list,
    resolve_reset_target, run_git, run_start_scripts,
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

    let (info, lifecycle) =
        add_worktree(&project, input.as_ref().and_then(|input| input.name.as_deref())).await?;

    let scope = vw_api_types::id::ProjectScope::new(
        crate::session::ui_store::current_tenant_scope(),
        vw_api_types::id::ProjectId::from(project_id.as_str()),
    )
    .map_err(|e| Error::Invalid(e.to_string()))?;
    let _ = project::add_sandbox(&scope, &info.directory).await;
    let primary = instance::worktree();

    let extra = input.and_then(|value| value.start_command).unwrap_or_default();
    prepare_checkout(&info, &project_id, &extra, &scope, &primary).await?;
    drop(lifecycle);
    super::cleanup_host::created();

    Ok(info)
}

#[cfg(not(target_arch = "wasm32"))]
async fn add_worktree(
    project: &project::Info,
    name: Option<&str>,
) -> Result<(Info, std::fs::File), Error> {
    let primary = instance::worktree();
    if primary.trim().is_empty() {
        return Err(Error::MissingProject("missing instance worktree".to_string()));
    }
    let lifecycle = super::registry::lifecycle_lock(std::path::Path::new(&primary))?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let settings = crate::config::get_global_for_tenant(&tenant)
        .await
        .map_err(|error| Error::Invalid(error.to_string()))?
        .worktree_settings;
    let root = super::creation_policy::prepare(project, &primary, &settings).await?;
    let base = name.map(slug).filter(|base| !base.trim().is_empty());
    let info = candidate(&root, base.as_deref()).await?;
    super::creation_policy::validate_parent(&root, std::path::Path::new(&info.directory)).await?;

    let created = run_git(
        &["worktree", "add", "--no-checkout", "-b", &info.branch, &info.directory],
        &primary,
    )
    .await?;
    if !created.success {
        return Err(Error::Invalid(created.error_text("Failed to create git worktree")));
    }
    let scope = vw_api_types::id::ProjectScope::new(
        crate::session::ui_store::current_tenant_scope(),
        vw_api_types::id::ProjectId::from(project.id.as_str()),
    )
    .map_err(|e| Error::Invalid(e.to_string()))?;
    if let Err(error) = super::registry::register_locked(
        &scope,
        std::path::Path::new(&primary),
        std::path::Path::new(&info.directory),
    )
    .await
    {
        let _ = run_git(&["worktree", "remove", "--force", &info.directory], &primary).await;
        let _ = run_git(&["branch", "-D", &info.branch], &primary).await;
        return Err(error);
    }
    if let Err(error) = super::environment::pending(&info.directory) {
        rollback_checkout(&scope, &primary, &info.directory).await;
        return Err(error);
    }
    Ok((info, lifecycle))
}

/// Prepare a checked-out worktree before a fork can be published.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn create_fork_checkout(name: Option<&str>) -> Result<Info, Error> {
    let project = ensure_git_project()?;
    let (info, lifecycle) = add_worktree(&project, name).await?;
    let scope = vw_api_types::id::ProjectScope::new(
        crate::session::ui_store::current_tenant_scope(),
        vw_api_types::id::ProjectId::from(project.id.as_str()),
    )
    .map_err(|error| Error::Invalid(error.to_string()))?;
    prepare_checkout(&info, &project.id, "", &scope, &instance::worktree()).await?;
    drop(lifecycle);
    super::cleanup_host::created();
    Ok(info)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn remove_fork_checkout(directory: String) -> Result<(), Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project = ensure_git_project()?;
    let primary = instance::worktree();
    let scope = vw_api_types::id::ProjectScope::new(tenant, project.id.into())
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let _lock = super::registry::lifecycle_lock(std::path::Path::new(&primary))?;
    remove_locked(RemoveInput { directory, force: true }, &scope, std::path::Path::new(&primary))
        .await?;
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
    remove_for_tenant(input, crate::session::ui_store::current_tenant_scope()).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn remove_for_tenant(
    input: RemoveInput,
    tenant: vw_api_types::id::TenantScope,
) -> Result<bool, Error> {
    use vw_api_types::id::{ProjectId, ProjectScope};
    let project = ensure_git_project()?;
    let primary = instance::worktree();
    let scope = ProjectScope::new(tenant, ProjectId::from(project.id.as_str()))
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let _lifecycle = super::registry::lifecycle_lock(std::path::Path::new(&primary))?;
    let (_, _, git_dir, _) = super::registry::checkout(
        std::path::Path::new(&primary),
        std::path::Path::new(&input.directory),
    )?;
    if super::registry::ownership(&git_dir)?.is_some() {
        return Err(Error::Invalid(
            "Managed worktree deletion requires the deletion-preflight and snapshot API".into(),
        ));
    }
    remove_locked(input, &scope, std::path::Path::new(&primary)).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn remove_locked(
    input: RemoveInput,
    scope: &vw_api_types::id::ProjectScope,
    primary: &std::path::Path,
) -> Result<bool, Error> {
    let (directory, _, git_dir, _) =
        super::registry::checkout(primary, std::path::Path::new(&input.directory))?;
    let managed = super::registry::ownership(&git_dir)?;
    if let Some(record) = &managed {
        super::registry::authorize(scope, primary, &record.id).await?;
        super::registry::ensure_not_in_use(record)?;
    } else if super::registry::record_for_path(scope, &directory).await?.is_some() {
        return Err(Error::Invalid("Managed checkout credential missing".into()));
    }
    let primary = primary.to_string_lossy().into_owned();
    let list = run_git(&["worktree", "list", "--porcelain"], &primary).await?;
    if !list.success {
        return Err(Error::Invalid(list.error_text("Failed to read git worktrees")));
    }
    let entries = parse_worktree_list(&list.stdout);

    let entry = find_entry(&entries, &directory).await;
    let entry = entry.ok_or_else(|| Error::Invalid("Worktree not found".into()))?;

    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if input.force {
        args.push("--force");
    }
    args.push(&entry.path);
    let removed = run_git(&args, &primary).await?;
    if !removed.success {
        return Err(Error::Invalid(removed.error_text("Failed to remove git worktree")));
    }

    if let Some(record) = managed {
        super::registry::mark_deleted(scope, &record.id).await?;
    }
    if let Some(branch) =
        entry.branch.as_deref().and_then(|value| value.strip_prefix("refs/heads/"))
    {
        let deleted = run_git(&["branch", "-D", branch], &primary).await?;
        if !deleted.success {
            return Err(Error::Invalid(deleted.error_text("Failed to delete worktree branch")));
        }
    }

    let _ = project::remove_sandbox(scope, &entry.path).await;
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
    let lease = super::registry::use_lease(&directory)?;

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
        let _lease = lease;
        let _ = run_start_scripts(&worktree_dir, &project_id, "").await;
    });

    Ok(true)
}

#[cfg(not(target_arch = "wasm32"))]
async fn rollback_checkout(scope: &vw_api_types::id::ProjectScope, primary: &str, directory: &str) {
    if let Err(error) = remove_locked(
        RemoveInput { directory: directory.into(), force: true },
        scope,
        std::path::Path::new(primary),
    )
    .await
    {
        tracing::warn!(%error, "failed to roll back a managed checkout");
    }
}

/// Checkout and bootstrap failures are rolled back; failed initialization is retained for retry.
#[cfg(not(target_arch = "wasm32"))]
async fn prepare_checkout(
    info: &Info,
    project_id: &str,
    extra: &str,
    scope: &vw_api_types::id::ProjectScope,
    primary: &str,
) -> Result<(), Error> {
    let checkout = async {
        let populated = run_git(&["reset", "--hard"], &info.directory).await?;
        if !populated.success {
            return Err(Error::Invalid(populated.error_text("Failed to populate worktree")));
        }
        // Use the caller's verified tenant/project rather than default-tenant discovery.
        let context = instance::Context {
            directory: info.directory.clone(),
            worktree: info.directory.clone(),
            project: ensure_git_project()?,
        };
        instance::provide_context(
            context,
            project::instance_bootstrap(PathBuf::from(&info.directory)),
        )
        .await;
        Ok(())
    }
    .await;
    if let Err(error) = checkout {
        rollback_checkout(scope, primary, &info.directory).await;
        publish_preparation(info, project_id, &Err(Error::Invalid(error.to_string())));
        return Err(error);
    }
    let result = super::environment::prepare(&info.directory, project_id, extra).await;
    publish_preparation(info, project_id, &result);
    result
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_preparation(info: &Info, project_id: &str, result: &Result<(), Error>) {
    let (definition, payload) = match result {
        Ok(()) => (
            event::READY,
            serde_json::json!({"projectId": project_id, "name": info.name, "branch": info.branch}),
        ),
        Err(error) => (
            event::FAILED,
            serde_json::json!({"projectId": project_id, "message": error.to_string()}),
        ),
    };
    let _ = bus::publish(definition, payload, Some(info.directory.clone()));
}

/// Retry only failed setup, preserving checkout contents and successful command stages.
#[cfg(not(target_arch = "wasm32"))]
pub async fn retry_environment(directory: String) -> Result<(), Error> {
    let project = ensure_git_project()?;
    let primary = instance::worktree();
    let _lifecycle = super::registry::lifecycle_lock(std::path::Path::new(&primary))?;
    let scope = vw_api_types::id::ProjectScope::new(
        crate::session::ui_store::current_tenant_scope(),
        vw_api_types::id::ProjectId::from(project.id.as_str()),
    )
    .map_err(|error| Error::Invalid(error.to_string()))?;
    let directory = std::fs::canonicalize(directory)?;
    let record = super::registry::record_for_path(&scope, &directory)
        .await?
        .ok_or_else(|| Error::Invalid("Managed checkout not found".into()))?;
    let record =
        super::registry::authorize(&scope, std::path::Path::new(&primary), &record.id).await?;
    let branch = git2::Repository::open(&record.directory)
        .and_then(|repo| repo.head().and_then(|head| head.shorthand().map(str::to_owned)))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let info = Info {
        directory: record.directory.to_string_lossy().into_owned(),
        name: record.directory.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        branch,
    };
    if super::environment::ensure_ready(&info.directory).is_ok() {
        return Ok(());
    }
    let result = super::environment::retry(&info.directory, &project.id).await;
    publish_preparation(&info, &project.id, &result);
    result
}
