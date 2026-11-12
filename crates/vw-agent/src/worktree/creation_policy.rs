//! Shared preflight for ordinary and fork checkout creation.
use super::{
    Error,
    native::{run_git, worktree_root},
};
use crate::{project, shell::git_tokio_command};
use std::path::{Path, PathBuf};
use vw_config_types::worktree::WorktreeSettings;

pub(super) async fn prepare(
    project: &project::Info,
    primary: &str,
    settings: &WorktreeSettings,
) -> Result<PathBuf, Error> {
    settings.validate().map_err(|error| Error::Invalid(error.into()))?;
    let root = if settings.root.trim().is_empty() {
        worktree_root(project)?
    } else {
        PathBuf::from(settings.root.trim())
    };
    if settings.fetch_before_create {
        fetch_upstream(primary).await?;
    }
    // Resolve the selected root once, then require each checkout's real parent
    // to remain this directory immediately before invoking Git.
    if tokio::fs::symlink_metadata(&root)
        .await
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(Error::Invalid("Worktree root must not be a symbolic link".into()));
    }
    tokio::fs::create_dir_all(&root).await?;
    Ok(tokio::fs::canonicalize(root).await?)
}

pub(super) async fn validate_parent(root: &Path, directory: &Path) -> Result<(), Error> {
    let parent =
        directory.parent().ok_or_else(|| Error::Invalid("Missing worktree parent".into()))?;
    if tokio::fs::canonicalize(parent).await? != root
        || tokio::fs::symlink_metadata(directory).await.is_ok()
    {
        return Err(Error::Invalid(
            "Worktree path is occupied or escapes the selected root".into(),
        ));
    }
    Ok(())
}

async fn fetch_upstream(primary: &str) -> Result<(), Error> {
    let branch = run_git(&["symbolic-ref", "--quiet", "--short", "HEAD"], primary).await?;
    if !branch.success {
        return Err(Error::Invalid(
            "Configure a branch upstream before enabling creation fetch (HEAD is detached)".into(),
        ));
    }
    let branch = branch.stdout.trim();
    let remote = run_git(&["config", "--get", &format!("branch.{branch}.remote")], primary).await?;
    let merge = run_git(&["config", "--get", &format!("branch.{branch}.merge")], primary).await?;
    if !remote.success || !merge.success || merge.stdout.trim().is_empty() {
        return Err(Error::Invalid(
            "Configure the current branch upstream before enabling creation fetch".into(),
        ));
    }
    let remote = remote.stdout.trim();
    let remotes = run_git(&["remote"], primary).await?;
    if !remotes.success || remote == "." || !remotes.stdout.lines().any(|name| name == remote) {
        return Err(Error::Invalid(
            "Configure an existing upstream remote before enabling creation fetch".into(),
        ));
    }
    let mut command = git_tokio_command();
    command
        .current_dir(primary)
        .args(["-c", "credential.interactive=false", "fetch", "--", remote])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "")
        .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(60), command.output())
        .await
        .map_err(|_| {
            Error::Invalid(
                "Creation fetch timed out after 60 seconds; check upstream connectivity".into(),
            )
        })??;
    if !output.status.success() {
        // Git stderr may include credential-bearing remote URLs.
        return Err(Error::Invalid(
            "Creation fetch failed; check the configured upstream remote and credentials".into(),
        ));
    }
    Ok(())
}
