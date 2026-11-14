//! Read-only GitHub skill discovery, adapted from cc-switch 4.0.25 (MIT).
//! Downloads live only in temporary storage; one repository failure never removes
//! another repository's results. No installed state or application paths are used.

use std::{collections::BTreeSet, fs, io::{Read, Write}, path::{Path, PathBuf}, time::Duration};

use serde::{Deserialize, Serialize};

use super::{archive::MAX_ARCHIVE_DOWNLOAD_BYTES, repos::{SkillRepo, validate_repo_ref}};

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_METADATA_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverableSkill {
    pub key: String,
    pub name: String,
    pub description: String,
    /// Repository-relative source directory; the root skill uses the repository name.
    pub directory: String,
    pub readme_url: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
}

/// Carries no upstream body, transport URL, credential, or local filesystem path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscoveryError {
    InvalidRepository,
    Forbidden,
    NotFound,
    RateLimited,
    Timeout,
    Transport,
    InvalidResponse,
    InvalidArchive,
    UnreadableSkill,
    HttpStatus { status: u16 },
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRepository => f.write_str("invalid skill repository"),
            Self::Forbidden => f.write_str("skill repository access denied"),
            Self::NotFound => f.write_str("skill repository not found"),
            Self::RateLimited => f.write_str("skill repository rate limited"),
            Self::Timeout => f.write_str("skill repository request timed out"),
            Self::Transport => f.write_str("skill repository network failure"),
            Self::InvalidResponse => f.write_str("invalid skill repository response"),
            Self::InvalidArchive => f.write_str("invalid skill repository archive"),
            Self::UnreadableSkill => f.write_str("cannot read repository skill metadata"),
            Self::HttpStatus { status } => write!(f, "skill repository HTTP status {status}"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRepoFailure {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub error: DiscoveryError,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiscoveryResult {
    pub skills: Vec<DiscoverableSkill>,
    pub failures: Vec<SkillRepoFailure>,
}

/// Uses the execution host's configured client, including its proxy policy.
/// Disabled repositories are skipped before validation or any HTTP request.
pub async fn discover_available_report(
    client: &reqwest::Client,
    repos: &[SkillRepo],
) -> SkillDiscoveryResult {
    discover_at(client, repos, "https://api.github.com", "https://codeload.github.com").await
}

pub async fn discover_repo(client: &reqwest::Client, repo: &SkillRepo) -> SkillDiscoveryResult {
    discover_available_report(client, std::slice::from_ref(repo)).await
}

pub async fn discover_at(
    client: &reqwest::Client,
    repos: &[SkillRepo],
    api: &str,
    archives: &str,
) -> SkillDiscoveryResult {
    let mut report = SkillDiscoveryResult::default();
    for repo in repos.iter().filter(|repo| repo.enabled) {
        match fetch_repo(client, repo, api, archives).await {
            Ok(skills) => report.skills.extend(skills),
            Err(error) => report.failures.push(SkillRepoFailure {
                owner: repo.owner.clone(), name: repo.name.clone(), branch: repo.branch.clone(), error,
            }),
        }
    }
    let mut seen = BTreeSet::new();
    report.skills.retain(|skill| seen.insert(skill.key.to_lowercase()));
    report.skills.sort_by_cached_key(|skill| (skill.name.to_lowercase(), skill.key.to_lowercase()));
    report
}

pub(super) async fn request(client: &reqwest::Client, url: reqwest::Url) -> Result<reqwest::Response, DiscoveryError> {
    let response = client.get(url).header("User-Agent", "ShiJu")
        .timeout(TIMEOUT).send().await.map_err(transport_error)?;
    match response.status().as_u16() {
        200..=299 => Ok(response),
        403 => Err(DiscoveryError::Forbidden),
        404 => Err(DiscoveryError::NotFound),
        429 => Err(DiscoveryError::RateLimited),
        status => Err(DiscoveryError::HttpStatus { status }),
    }
}

pub(super) fn transport_error(error: reqwest::Error) -> DiscoveryError {
    if error.is_timeout() { DiscoveryError::Timeout } else { DiscoveryError::Transport }
}

// Push path segments rather than interpolating untrusted branch/path text into URLs.
pub(super) fn url(base: &str, segments: &[&str]) -> Result<reqwest::Url, DiscoveryError> {
    let mut url = reqwest::Url::parse(base).map_err(|_| DiscoveryError::InvalidResponse)?;
    url.path_segments_mut().map_err(|_| DiscoveryError::InvalidResponse)?.pop_if_empty().extend(segments.iter().copied());
    Ok(url)
}

async fn fetch_repo(
    client: &reqwest::Client,
    repo: &SkillRepo,
    api: &str,
    archives: &str,
) -> Result<Vec<DiscoverableSkill>, DiscoveryError> {
    validate_repo_ref(&repo.owner, &repo.name, &repo.branch).map_err(|_| DiscoveryError::InvalidRepository)?;
    let mut resolved = repo.clone();
    if repo.branch.is_empty() || repo.branch.eq_ignore_ascii_case("HEAD") {
        let mut response = request(client, url(api, &["repos", &repo.owner, &repo.name])?).await?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if chunk.len() > MAX_METADATA_BYTES as usize - body.len() {
                return Err(DiscoveryError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        #[derive(Deserialize)]
        struct Repository { default_branch: String }
        resolved.branch = serde_json::from_slice::<Repository>(&body)
            .map_err(|_| DiscoveryError::InvalidResponse)?.default_branch;
        if resolved.branch.is_empty() || resolved.branch.eq_ignore_ascii_case("HEAD") {
            return Err(DiscoveryError::InvalidResponse);
        }
        validate_repo_ref(&resolved.owner, &resolved.name, &resolved.branch)
            .map_err(|_| DiscoveryError::InvalidResponse)?;
    }
    let guard = download_branch(client, &resolved, archives).await?;
    scan_repo(guard.path(), &resolved)
}

pub(super) async fn download_branch(
    client: &reqwest::Client,
    repo: &SkillRepo,
    archives: &str,
) -> Result<tempfile::TempDir, DiscoveryError> {
    download_branch_checked(client, repo, archives, &|| Ok(())).await
}

pub(super) async fn download_branch_checked(
    client: &reqwest::Client, repo: &SkillRepo, archives: &str,
    check: &(dyn Fn() -> std::io::Result<()> + Sync),
) -> Result<tempfile::TempDir, DiscoveryError> {
    check().map_err(|_| DiscoveryError::InvalidArchive)?;
    let mut response = request(client, url(archives, &[&repo.owner, &repo.name, "zip", "refs", "heads", &repo.branch])?).await?;
    if response.content_length().is_some_and(|length| length > MAX_ARCHIVE_DOWNLOAD_BYTES) {
        return Err(DiscoveryError::InvalidArchive);
    }
    let mut file = tempfile::tempfile().map_err(|_| DiscoveryError::InvalidArchive)?;
    let mut size = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        check().map_err(|_| DiscoveryError::InvalidArchive)?;
        size = size.checked_add(chunk.len() as u64).filter(|size| *size <= MAX_ARCHIVE_DOWNLOAD_BYTES)
            .ok_or(DiscoveryError::InvalidArchive)?;
        file.write_all(&chunk).map_err(|_| DiscoveryError::InvalidArchive)?;
    }
    let guard = super::archive::extract_repo_archive_checked(file, check).map_err(|_| DiscoveryError::InvalidArchive)?;
    Ok(guard)
}

#[derive(Default, Deserialize)]
struct Metadata { name: Option<String>, description: Option<String> }

fn metadata(path: &Path) -> Result<Metadata, DiscoveryError> {
    let mut content = String::new();
    fs::File::open(path).map_err(|_| DiscoveryError::UnreadableSkill)?
        .take(MAX_METADATA_BYTES + 1).read_to_string(&mut content)
        .map_err(|_| DiscoveryError::UnreadableSkill)?;
    if content.len() as u64 > MAX_METADATA_BYTES { return Err(DiscoveryError::UnreadableSkill); }
    let content = content.trim_start_matches('\u{feff}');
    let mut lines = content.lines();
    if lines.next() != Some("---") { return Ok(Metadata::default()); }
    let mut front = String::new();
    for line in lines {
        if line == "---" { return Ok(serde_yaml::from_str(&front).unwrap_or_default()); }
        front.push_str(line);
        front.push('\n');
    }
    Ok(Metadata::default())
}

// Only called on the private, bounded extraction directory, never a client path.
pub(super) fn scan_repo(root: &Path, repo: &SkillRepo) -> Result<Vec<DiscoverableSkill>, DiscoveryError> {
    scan_repo_checked(root, repo, &|| Ok(()))
}

pub(super) fn scan_repo_checked(root: &Path, repo: &SkillRepo,
    check: &(dyn Fn() -> std::io::Result<()> + Sync)) -> Result<Vec<DiscoverableSkill>, DiscoveryError> {
    let mut pending = vec![PathBuf::new()];
    let mut skills = Vec::new();
    while let Some(relative) = pending.pop() {
        check().map_err(|_| DiscoveryError::UnreadableSkill)?;
        let mut directories = Vec::new();
        let mut document = None;
        for entry in fs::read_dir(root.join(&relative)).map_err(|_| DiscoveryError::UnreadableSkill)? {
            let entry = entry.map_err(|_| DiscoveryError::UnreadableSkill)?;
            let kind = entry.file_type().map_err(|_| DiscoveryError::UnreadableSkill)?;
            if kind.is_symlink() { return Err(DiscoveryError::InvalidArchive); }
            let name = entry.file_name().into_string().map_err(|_| DiscoveryError::InvalidArchive)?;
            if kind.is_file() && name == "SKILL.md" { document = Some(relative.join(name)); }
            else if kind.is_dir() { directories.push(relative.join(name)); }
        }
        if let Some(document) = document {
            let meta = metadata(&root.join(&document))?;
            let directory = if relative.as_os_str().is_empty() { repo.name.clone() }
                else { relative.to_str().ok_or(DiscoveryError::InvalidArchive)?.to_owned() };
            let mut segments = vec![repo.owner.as_str(), repo.name.as_str(), "blob", repo.branch.as_str()];
            let doc = document.to_str().ok_or(DiscoveryError::InvalidArchive)?;
            segments.extend(doc.split('/'));
            skills.push(DiscoverableSkill {
                key: format!("{}/{}:{directory}", repo.owner, repo.name),
                name: meta.name.unwrap_or_else(|| directory.clone()),
                description: meta.description.unwrap_or_default(), directory,
                readme_url: Some(url("https://github.com", &segments)?.into()),
                repo_owner: repo.owner.clone(), repo_name: repo.name.clone(), repo_branch: repo.branch.clone(),
            });
            // A skill owns its subtree; nested examples are not separate skills.
        } else {
            directories.sort();
            pending.extend(directories.into_iter().rev());
        }
    }
    Ok(skills)
}
