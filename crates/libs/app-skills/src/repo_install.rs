//! GitHub archive installation; no Git subprocess, hooks or scripts are executed.
//! Branch fallback follows the preserved cc-switch 4.0.25 implementation (MIT).
use std::{path::Path, time::Duration};
use serde::{Deserialize, Serialize};
use super::{
    discovery::{DiscoveryError},
    local_install::{LocalInstall, LocalInstallError, validate_directory},
    repos::{SkillRepo, validate_repo_ref},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSkillSource {
    pub owner: String,
    pub name: String,
    /// Actual downloaded branch, never the HEAD sentinel.
    pub branch: String,
    pub directory: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RepoInstallError {
    #[error(transparent)]
    Repository(#[from] DiscoveryError),
    #[error(transparent)]
    Local(#[from] LocalInstallError),
    #[error("selected skill was not found in the downloaded repository")]
    SkillNotFound,
}

pub struct RepoInstall {
    pub installation: LocalInstall,
    pub source: RepoSkillSource,
}

/// Call on the authorized execution host, with its configured HTTP client and
/// SSOT root. The selection is repository-relative, never a client filesystem path.
pub async fn prepare_repo_install(
    client: &reqwest::Client,
    repo: &SkillRepo,
    selection: &str,
    ssot: &Path,
    directory: &str,
) -> Result<RepoInstall, RepoInstallError> {
    prepare_at(client, repo, selection, ssot, directory, "https://codeload.github.com").await
}

pub(super) async fn prepare_at(
    client: &reqwest::Client,
    repo: &SkillRepo,
    selection: &str,
    ssot: &Path,
    directory: &str,
    archives: &str,
) -> Result<RepoInstall, RepoInstallError> {
    prepare_checked(client, repo, selection, ssot, directory, archives, &|| Ok(())).await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_checked(
    client: &reqwest::Client, repo: &SkillRepo, selection: &str, ssot: &Path,
    directory: &str, archives: &str, check: &(dyn Fn() -> std::io::Result<()> + Sync),
) -> Result<RepoInstall, RepoInstallError> {
    check().map_err(LocalInstallError::from)?;
    validate_repo_ref(&repo.owner, &repo.name, &repo.branch)
        .map_err(|_| DiscoveryError::InvalidRepository)?;
    validate_directory(directory)?;
    if selection.is_empty() || selection.split('/').any(|part| {
        part.is_empty() || matches!(part, "." | "..") || part.contains(['\\', ':'])
            || part.chars().any(char::is_control)
    }) {
        return Err(RepoInstallError::SkillNotFound);
    }
    let downloaded = tokio::time::timeout(Duration::from_secs(60), async {
        let mut branches = Vec::new();
        if !repo.branch.is_empty() && !repo.branch.eq_ignore_ascii_case("HEAD") {
            branches.push(repo.branch.as_str());
        }
        for branch in ["main", "master"] {
            if !branches.contains(&branch) { branches.push(branch); }
        }
        for branch in branches {
            let mut resolved = repo.clone();
            resolved.branch = branch.to_owned();
            match super::discovery::download_branch_checked(client, &resolved, archives, check).await {
                Ok(guard) => return Ok((guard, resolved)),
                Err(DiscoveryError::NotFound) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(DiscoveryError::NotFound)
    }).await.map_err(|_| DiscoveryError::Timeout)??;
    let (guard, resolved) = downloaded;
    // Discover again from the exact downloaded tree. Ignore client names/readme URLs.
    let found = super::discovery::scan_repo_checked(guard.path(), &resolved, check)?.into_iter()
        .find(|skill| skill.directory == selection)
        .ok_or(RepoInstallError::SkillNotFound)?;
    let source = if guard.path().join("SKILL.md").is_file() {
        guard.path().to_owned()
    } else {
        guard.path().join(&found.directory)
    };
    check().map_err(LocalInstallError::from)?;
    let installation = LocalInstall::prepare_checked(ssot, &source, directory, check)?;
    Ok(RepoInstall {
        installation,
        source: RepoSkillSource {
            owner: resolved.owner, name: resolved.name, branch: resolved.branch,
            directory: found.directory,
        },
    })
}
