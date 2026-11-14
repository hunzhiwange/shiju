//! Repository references and mutations, independent of storage and execution hosts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRepo {
    pub owner: String,
    pub name: String,
    /// Empty or HEAD selects the default branch; consumers must resolve this sentinel.
    pub branch: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoError {
    InvalidOwner,
    InvalidName,
    InvalidBranch,
    DuplicateRepository,
    RepositoryNotFound,
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidOwner => "invalid skill repository owner",
            Self::InvalidName => "invalid skill repository name",
            Self::InvalidBranch => "invalid skill repository branch",
            Self::DuplicateRepository => "duplicate saved skill repository",
            Self::RepositoryNotFound => "skill repository not found",
        })
    }
}

impl std::error::Error for RepoError {}

/// Validate on both ingress and persisted reads. Network consumers must also
/// validate the final URL and resolve default-branch sentinels before downloading.
pub fn validate_repo_ref(owner: &str, name: &str, branch: &str) -> Result<(), RepoError> {
    validate_identity(owner, name)?;
    if branch.is_empty() || branch.eq_ignore_ascii_case("HEAD") {
        return Ok(());
    }
    if branch.len() > 255
        || branch == "@"
        || branch.contains("..")
        || branch.contains("@{")
        || branch.chars().any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\#%".contains(c))
        || branch.split('/').any(|segment| {
            segment.is_empty()
                || segment.starts_with('.')
                || segment.ends_with('.')
                || segment.ends_with(".lock")
        })
    {
        return Err(RepoError::InvalidBranch);
    }
    Ok(())
}

fn validate_identity(owner: &str, name: &str) -> Result<(), RepoError> {
    if owner.is_empty()
        || owner.len() > 39
        || !owner.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(RepoError::InvalidOwner);
    }
    if name.is_empty()
        || name.len() > 100
        || matches!(name, "." | "..")
        || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
    {
        return Err(RepoError::InvalidName);
    }
    Ok(())
}

/// An ordered set keyed by GitHub owner/name, whose identity is case insensitive.
/// No installed skill state is owned or changed by this collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillRepos(Vec<SkillRepo>);

impl SkillRepos {
    pub fn from_saved(repos: Vec<SkillRepo>) -> Result<Self, RepoError> {
        let mut result = Self::default();
        for repo in repos {
            validate_repo_ref(&repo.owner, &repo.name, &repo.branch)?;
            if result.position(&repo.owner, &repo.name).is_some() {
                return Err(RepoError::DuplicateRepository);
            }
            result.0.push(repo);
        }
        result.sort();
        Ok(result)
    }

    pub fn entries(&self) -> &[SkillRepo] {
        &self.0
    }

    /// Add or replace configuration for the same repository, including enabled state.
    pub fn save(&mut self, repo: SkillRepo) -> Result<(), RepoError> {
        validate_repo_ref(&repo.owner, &repo.name, &repo.branch)?;
        if let Some(index) = self.position(&repo.owner, &repo.name) {
            self.0[index] = repo;
        } else {
            self.0.push(repo);
        }
        self.sort();
        Ok(())
    }

    pub fn set_enabled(&mut self, owner: &str, name: &str, enabled: bool) -> Result<(), RepoError> {
        validate_identity(owner, name)?;
        let index = self.position(owner, name).ok_or(RepoError::RepositoryNotFound)?;
        self.0[index].enabled = enabled;
        Ok(())
    }

    pub fn remove(&mut self, owner: &str, name: &str) -> Result<bool, RepoError> {
        validate_identity(owner, name)?;
        match self.position(owner, name) {
            Some(index) => {
                self.0.remove(index);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn position(&self, owner: &str, name: &str) -> Option<usize> {
        self.0.iter().position(|repo| {
            repo.owner.eq_ignore_ascii_case(owner) && repo.name.eq_ignore_ascii_case(name)
        })
    }

    fn sort(&mut self) {
        self.0.sort_by_cached_key(|repo| {
            (repo.owner.to_ascii_lowercase(), repo.name.to_ascii_lowercase())
        });
    }
}
