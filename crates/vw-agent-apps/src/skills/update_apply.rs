//! Staged update of one SSOT and its existing managed projections.
//! The caller holds the settings transaction until finish or explicit rollback.
use std::{collections::BTreeMap, path::Path};
use super::{
    copy_deploy::{binding_fingerprint, CopyBinding, CopyDeploy},
    inventory::{compute_dir_hash, get_all_installed},
    link_deploy::SyncMethod,
    local_install::{copy_skill_tree, validate_directory, LocalInstallError, LocalSkill},
};

type Result<T> = std::result::Result<T, LocalInstallError>;

pub struct SkillUpdate {
    // Retain the SSOT lock until application rollback has finished.
    projections: Vec<(String, CopyDeploy)>,
    ssot: CopyDeploy,
    _source: tempfile::TempDir,
    pub skill: LocalSkill,
    pub content_hash: String,
}

impl SkillUpdate {
    /// Paths and bindings must belong to the authorized host and saved snapshot.
    /// Missing or locally edited trees are conflicts, never implicit reinstalls.
    pub fn prepare(root: &Path, source: &Path, saved: &LocalSkill,
        expected_hash: &str, bindings: &BTreeMap<String, CopyBinding>) -> Result<Self> {
        validate_directory(&saved.directory)?;
        if !root.is_absolute() || !source.is_absolute() { return Err(LocalInstallError::UnsafeSource); }
        let canonical_root = std::fs::canonicalize(root)?;
        let root = canonical_root.as_path();
        let current = root.join(&saved.directory);
        let old = CopyBinding { directory: saved.directory.clone(), destination: current.clone(),
            fingerprint: binding_fingerprint(&current, None)?, link_target: None };
        let stage = tempfile::tempdir()?;
        copy_skill_tree(source, &stage.path().join(&saved.directory))?;
        let ssot = CopyDeploy::prepare_tree(stage.path(), root, &saved.directory, Some(&old), SyncMethod::Copy)?;
        // Checked under the SSOT lock, after publication staging.
        if compute_dir_hash(&current)? != expected_hash { return Err(LocalInstallError::Conflict); }
        let detail = get_all_installed(stage.path(), std::slice::from_ref(saved))?
            .pop().ok_or(LocalInstallError::InvalidSkill)?;
        let mut skill = detail.skill;
        skill.source = saved.source.clone();
        let content_hash = detail.content_hash.ok_or(LocalInstallError::InvalidSkill)?;
        let mut selected: Vec<_> = bindings.iter().filter(|(_, b)| b.directory == saved.directory).collect();
        selected.sort_by(|a, b| a.1.destination.cmp(&b.1.destination));
        // Each physical root is locked once; aliases/overlaps are rejected.
        let mut roots = vec![std::fs::canonicalize(root)?];
        let mut projections = Vec::new();
        for (key, binding) in selected {
            let parent = binding.destination.parent().ok_or(LocalInstallError::UnsafeSource)?;
            let parent = std::fs::canonicalize(parent)?;
            if roots.iter().any(|p| p.starts_with(&parent) || parent.starts_with(p)) {
                return Err(LocalInstallError::UnsafeSource);
            }
            roots.push(parent.clone());
            // Links retain their stable SSOT target and ownership fingerprint.
            // Stage them from the live root, copies from the new source.
            let source_root = if binding.link_target.is_some() { root } else { stage.path() };
            let projection = CopyDeploy::prepare_tree(source_root, &parent, &saved.directory,
                Some(binding), SyncMethod::Auto)?;
            projections.push((key.clone(), projection));
        }
        Ok(Self { projections, ssot, _source: stage, skill, content_hash })
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&str, &CopyBinding)> {
        self.projections.iter().map(|(key, copy)| (key.as_str(), copy.binding()))
    }

    pub fn publish(&mut self) -> Result<()> {
        self.ssot.publish()?;
        for (_, copy) in &mut self.projections { copy.publish()?; }
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        let mut failure = None;
        for (_, copy) in self.projections.iter_mut().rev() {
            if let Err(error) = copy.rollback() { failure = Some(error); }
        }
        if let Err(error) = self.ssot.rollback() { failure = Some(error); }
        match failure { Some(error) => Err(error), None => Ok(()) }
    }

    pub fn finish(self) {
        for (_, copy) in self.projections { copy.finish(); }
        self.ssot.finish();
    }
}

/// Download the exact persisted branch; the temporary tree stays alive until commit.
pub async fn download_update(client: &reqwest::Client, source: &super::repo_install::RepoSkillSource)
    -> std::result::Result<(tempfile::TempDir, std::path::PathBuf), super::repo_install::RepoInstallError> {
    download_update_at(client, source, "https://codeload.github.com").await
}

/// Execution-side archive endpoint; never taken from an API request.
pub async fn download_update_at(client: &reqwest::Client, source: &super::repo_install::RepoSkillSource,
    archives: &str,
) -> std::result::Result<(tempfile::TempDir, std::path::PathBuf), super::repo_install::RepoInstallError> {
    use super::{discovery, repos::{SkillRepo, validate_repo_ref}};
    validate_repo_ref(&source.owner, &source.name, &source.branch)
        .map_err(|_| discovery::DiscoveryError::InvalidRepository)?;
    if source.branch.is_empty() || source.branch.eq_ignore_ascii_case("HEAD") {
        return Err(discovery::DiscoveryError::InvalidRepository.into());
    }
    let repo = SkillRepo { owner: source.owner.clone(), name: source.name.clone(),
        branch: source.branch.clone(), enabled: true };
    let archive = tokio::time::timeout(std::time::Duration::from_secs(60),
        discovery::download_branch(client, &repo, archives))
        .await.map_err(|_| discovery::DiscoveryError::Timeout)??;
    let found = discovery::scan_repo(archive.path(), &repo)?.into_iter()
        .find(|skill| skill.directory == source.directory)
        .ok_or(super::repo_install::RepoInstallError::SkillNotFound)?;
    let tree = if archive.path().join("SKILL.md").is_file() { archive.path().to_owned() }
        else { archive.path().join(found.directory) };
    Ok((archive, tree))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SkillUpdateMetadata {
    pub content_hash: String,
    pub updated_at: u64,
}
