//! Read-only update checks on the authorized execution host (cc-switch 4.0.25, MIT).
use std::{collections::BTreeMap, path::Path, time::Duration};
use serde::{Deserialize, Serialize};
use super::{
    discovery::{download_branch, scan_repo, DiscoveryError, SkillRepoFailure},
    inventory::{compute_dir_hash, local_hash_for_update_check},
    local_install::LocalSkill,
    repo_install::RepoSkillSource,
    repos::{validate_repo_ref, SkillRepo},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillUpdateInfo {
    pub directory: String,
    pub name: String,
    pub current_hash: Option<String>,
    pub remote_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCheckError {
    LocalUnreadable,
    RemoteSkillMissing,
    RemoteUnreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCheckFailure {
    pub directory: String,
    pub error: SkillCheckError,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillUpdateCheckResult {
    pub updates: Vec<SkillUpdateInfo>,
    pub failures: Vec<SkillRepoFailure>,
    pub skill_failures: Vec<SkillCheckFailure>,
    pub without_repository: Vec<String>,
    pub unchanged: Vec<String>,
}

/// Uses persisted provenance and fresh filesystem bytes, never discovery caches.
/// No state is written, including when a repository or individual skill fails.
pub async fn check_updates_report(
    client: &reqwest::Client,
    root: &Path,
    skills: &[LocalSkill],
    sources: &BTreeMap<String, RepoSkillSource>,
) -> SkillUpdateCheckResult {
    check_at(client, root, skills, sources, "https://codeload.github.com").await
}

pub async fn check_at(
    client: &reqwest::Client,
    root: &Path,
    skills: &[LocalSkill],
    sources: &BTreeMap<String, RepoSkillSource>,
    archives: &str,
) -> SkillUpdateCheckResult {
    let mut report = SkillUpdateCheckResult::default();
    let mut groups = BTreeMap::<(String, String, String), Vec<(&LocalSkill, &RepoSkillSource)>>::new();
    for skill in skills {
        match sources.get(&skill.directory) {
            Some(source) => groups.entry((source.owner.to_ascii_lowercase(), source.name.to_ascii_lowercase(), source.branch.clone()))
                .or_default().push((skill, source)),
            None => report.without_repository.push(skill.directory.clone()),
        }
    }
    for ((owner, name, branch), group) in groups {
        let repo = SkillRepo { owner, name, branch, enabled: true };
        // Installation records contain the actual branch. Never silently compare
        // a different branch if the saved branch disappears.
        let downloaded = if validate_repo_ref(&repo.owner, &repo.name, &repo.branch).is_err()
            || repo.branch.is_empty() || repo.branch.eq_ignore_ascii_case("HEAD") {
            Err(DiscoveryError::InvalidRepository)
        } else {
            tokio::time::timeout(Duration::from_secs(60), download_branch(client, &repo, archives))
                .await.unwrap_or(Err(DiscoveryError::Timeout))
        };
        let guard = match downloaded {
            Ok(guard) => guard,
            Err(error) => {
                report.failures.push(SkillRepoFailure { owner: repo.owner, name: repo.name, branch: repo.branch, error });
                continue;
            }
        };
        let remote = match scan_repo(guard.path(), &repo) {
            Ok(remote) => remote,
            Err(error) => {
                report.failures.push(SkillRepoFailure { owner: repo.owner, name: repo.name, branch: repo.branch, error });
                continue;
            }
        };
        for (skill, source) in group {
            let result = (|| {
                let found = remote.iter().find(|entry| entry.directory == source.directory)
                    .ok_or(SkillCheckError::RemoteSkillMissing)?;
                let remote_path = if guard.path().join("SKILL.md").is_file() {
                    guard.path().to_owned()
                } else { guard.path().join(&found.directory) };
                let remote_hash = compute_dir_hash(&remote_path).map_err(|_| SkillCheckError::RemoteUnreadable)?;
                let current_hash = local_hash_for_update_check(root, &skill.directory)
                    .map_err(|_| SkillCheckError::LocalUnreadable)?;
                Ok::<_, SkillCheckError>((current_hash, remote_hash))
            })();
            match result {
                Ok((current_hash, remote_hash)) if current_hash.as_ref() == Some(&remote_hash) => report.unchanged.push(skill.directory.clone()),
                Ok((current_hash, remote_hash)) => report.updates.push(SkillUpdateInfo {
                    directory: skill.directory.clone(), name: skill.name.clone(), current_hash, remote_hash,
                }),
                Err(error) => report.skill_failures.push(SkillCheckFailure { directory: skill.directory.clone(), error }),
            }
        }
    }
    report.updates.sort_by(|a, b| a.directory.cmp(&b.directory));
    report.skill_failures.sort_by(|a, b| a.directory.cmp(&b.directory));
    report.without_repository.sort();
    report.unchanged.sort();
    report
}
