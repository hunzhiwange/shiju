//! Read-only discovery in the public skills.sh catalog.

use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://skills.sh/api/search";
const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Carries no response body, request URL, query, or transport error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketSearchError {
    Forbidden,
    NotFound,
    RateLimited,
    Timeout,
    Transport,
    InvalidResponse,
    HttpStatus { status: u16 },
}

impl fmt::Display for MarketSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden => f.write_str("market search access denied"),
            Self::NotFound => f.write_str("market search endpoint not found"),
            Self::RateLimited => f.write_str("market search rate limited"),
            Self::Timeout => f.write_str("market search timed out"),
            Self::Transport => f.write_str("market search network failure"),
            Self::InvalidResponse => f.write_str("market search response invalid"),
            Self::HttpStatus { status } => write!(f, "market search HTTP status {status}"),
        }
    }
}

impl std::error::Error for MarketSearchError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShSearchResult {
    pub skills: Vec<SkillsShDiscoverableSkill>,
    /// Upstream count before filtering unsupported repository sources.
    pub total_count: usize,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShDiscoverableSkill {
    pub key: String,
    pub name: String,
    /// Market skill identifier, not a verified path inside the repository.
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub installs: u64,
    pub readme_url: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    query: String,
    skills: Vec<ApiSkill>,
    count: usize,
}

#[derive(Deserialize)]
struct ApiSkill {
    id: String,
    #[serde(rename = "skillId")]
    skill_id: String,
    name: String,
    installs: u64,
    source: String,
}

/// The caller supplies its configured HTTP client (including proxy policy).
/// Search never installs files, writes settings, or executes repository scripts.
pub async fn search_skills_sh(
    client: &reqwest::Client,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, MarketSearchError> {
    search_at(client, ENDPOINT, query, limit, offset, TIMEOUT).await
}

pub(super) async fn search_at(
    client: &reqwest::Client,
    endpoint: &str,
    query: &str,
    limit: usize,
    offset: usize,
    timeout: Duration,
) -> Result<SkillsShSearchResult, MarketSearchError> {
    let mut response = client
        .get(endpoint)
        .query(&[("q", query), ("limit", &limit.to_string()), ("offset", &offset.to_string())])
        .timeout(timeout)
        .send()
        .await
        .map_err(transport_error)?;
    match response.status().as_u16() {
        200..=299 => {}
        403 => return Err(MarketSearchError::Forbidden),
        404 => return Err(MarketSearchError::NotFound),
        429 => return Err(MarketSearchError::RateLimited),
        status => return Err(MarketSearchError::HttpStatus { status }),
    }
    if response.content_length().is_some_and(|size| size > MAX_RESPONSE_BYTES as u64) {
        return Err(MarketSearchError::InvalidResponse);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if chunk.len() > MAX_RESPONSE_BYTES - body.len() {
            return Err(MarketSearchError::InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    parse_response(&body)
}

fn transport_error(error: reqwest::Error) -> MarketSearchError {
    if error.is_timeout() {
        MarketSearchError::Timeout
    } else if error.is_decode() || error.is_body() {
        MarketSearchError::InvalidResponse
    } else {
        MarketSearchError::Transport
    }
}

fn parse_response(body: &[u8]) -> Result<SkillsShSearchResult, MarketSearchError> {
    let response: ApiResponse =
        serde_json::from_slice(body).map_err(|_| MarketSearchError::InvalidResponse)?;
    // Keep upstream ranking. Unsupported/non-GitHub sources are omitted, as in
    // the source service; total_count remains the market's unfiltered count.
    let skills = response.skills.into_iter().filter_map(|skill| {
        let (owner, repo) = skill.source.split_once('/')?;
        if !valid_repository(owner, repo) || !valid_directory(&skill.skill_id) {
            return None;
        }
        Some(SkillsShDiscoverableSkill {
            key: skill.id,
            name: skill.name,
            directory: skill.skill_id,
            repo_owner: owner.to_owned(),
            repo_name: repo.to_owned(),
            repo_branch: "main".to_owned(),
            installs: skill.installs,
            readme_url: Some(format!("https://github.com/{owner}/{repo}")),
        })
    }).collect();
    Ok(SkillsShSearchResult { skills, total_count: response.count, query: response.query })
}

// Matches the upstream GitHub coordinate allowlist, including dotted repos.
fn valid_repository(owner: &str, repo: &str) -> bool {
    !owner.is_empty()
        && owner.len() <= 39
        && owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !repo.is_empty()
        && repo.len() <= 100
        && !matches!(repo, "." | "..")
        && repo.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn valid_directory(directory: &str) -> bool {
    !directory.is_empty()
        && !matches!(directory, "." | "..")
        && directory.trim() == directory
        && !directory.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
}
