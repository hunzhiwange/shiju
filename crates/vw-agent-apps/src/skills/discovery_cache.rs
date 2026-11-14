//! One cache instance belongs to one authorized project and execution host.
//! Configuration revisions must come from the repository store, never request order.

use std::{collections::BTreeMap, sync::Mutex, time::{Duration, Instant}};

use super::{discovery::{SkillDiscoveryResult, discover_at}, repos::{SkillRepo, SkillRepos}};

const TTL: Duration = Duration::from_secs(600);
type RepoKey = (String, String, String);

fn key(repo: &SkillRepo) -> RepoKey {
    (repo.owner.to_ascii_lowercase(), repo.name.to_ascii_lowercase(), repo.branch.clone())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    InvalidConfiguration,
    StaleConfiguration,
    ConflictingConfiguration,
    NotInitialized,
    SequenceExhausted,
    Poisoned,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "skill discovery cache: {self:?}")
    }
}
impl std::error::Error for CacheError {}

struct Partition {
    repo: SkillRepo,
    sequence: u64,
    result: Option<(Instant, SkillDiscoveryResult)>,
}

#[derive(Default)]
struct State {
    configuration: Option<(u64, Vec<SkillRepo>)>,
    sequence: u64,
    partitions: BTreeMap<RepoKey, Partition>,
}

#[derive(Default)]
pub struct SkillDiscoveryCache(Mutex<State>);

impl SkillDiscoveryCache {
    /// Apply a committed full repository snapshot. Unchanged partitions retain
    /// their results; removals, branch changes and disabled repositories discard
    /// their partitions, invalidating every outstanding publication token.
    pub fn update_repositories(&self, version: u64, repos: Vec<SkillRepo>) -> Result<(), CacheError> {
        let repos = SkillRepos::from_saved(repos).map_err(|_| CacheError::InvalidConfiguration)?;
        let repos = repos.entries().to_vec();
        let mut state = self.0.lock().map_err(|_| CacheError::Poisoned)?;
        if let Some((current, saved)) = &state.configuration {
            if version < *current { return Err(CacheError::StaleConfiguration); }
            if version == *current {
                return if saved == &repos { Ok(()) } else { Err(CacheError::ConflictingConfiguration) };
            }
        }
        state.partitions.retain(|k, _| repos.iter().any(|repo| repo.enabled && key(repo) == *k));
        for repo in repos.iter().filter(|repo| repo.enabled) {
            state.partitions.entry(key(repo)).or_insert_with(|| Partition {
                repo: repo.clone(), sequence: 0, result: None,
            });
        }
        state.configuration = Some((version, repos));
        Ok(())
    }

    /// Fetch missing/expired partitions or bypass all hits on force refresh.
    /// No lock spans network I/O. Late results are omitted from both cache and
    /// returned snapshot, including when a repository was removed and re-added.
    pub async fn discover(&self, client: &reqwest::Client, force_refresh: bool) -> Result<SkillDiscoveryResult, CacheError> {
        self.discover_at(client, force_refresh, "https://api.github.com", "https://codeload.github.com").await
    }

    pub async fn discover_at(&self, client: &reqwest::Client, force_refresh: bool,
        api: &str, archives: &str,
    ) -> Result<SkillDiscoveryResult, CacheError> {
        let requests = self.begin(force_refresh, Instant::now())?;
        for (repo, sequence) in requests {
            let result = discover_at(client, std::slice::from_ref(&repo), api, archives).await;
            self.finish(&key(&repo), sequence, result, Instant::now())?;
        }
        self.snapshot()
    }

    fn begin(&self, force: bool, now: Instant) -> Result<Vec<(SkillRepo, u64)>, CacheError> {
        let mut state = self.0.lock().map_err(|_| CacheError::Poisoned)?;
        if state.configuration.is_none() { return Err(CacheError::NotInitialized); }
        let keys: Vec<_> = state.partitions.iter().filter(|(_, partition)| {
            force || !partition.result.as_ref().is_some_and(|(time, report)| {
                report.failures.is_empty() && now.saturating_duration_since(*time) < TTL
            })
        }).map(|(key, _)| key.clone()).collect();
        // Preflight overflow before changing any partition.
        let end = state.sequence.checked_add(keys.len() as u64).ok_or(CacheError::SequenceExhausted)?;
        let mut requests = Vec::new();
        for key in keys {
            state.sequence += 1;
            let sequence = state.sequence;
            let partition = state.partitions.get_mut(&key).expect("selected partition");
            partition.sequence = sequence;
            partition.result = None;
            requests.push((partition.repo.clone(), sequence));
        }
        state.sequence = end;
        Ok(requests)
    }

    fn finish(&self, key: &RepoKey, sequence: u64, result: SkillDiscoveryResult, now: Instant) -> Result<(), CacheError> {
        let mut state = self.0.lock().map_err(|_| CacheError::Poisoned)?;
        if let Some(partition) = state.partitions.get_mut(key) {
            if partition.sequence == sequence {
                partition.result = Some((now, result));
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<SkillDiscoveryResult, CacheError> {
        let state = self.0.lock().map_err(|_| CacheError::Poisoned)?;
        if state.configuration.is_none() { return Err(CacheError::NotInitialized); }
        let mut report = SkillDiscoveryResult::default();
        for partition in state.partitions.values() {
            if let Some((_, result)) = &partition.result {
                report.skills.extend(result.skills.clone());
                report.failures.extend(result.failures.clone());
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        report.skills.retain(|skill| seen.insert(skill.key.to_lowercase()));
        report.skills.sort_by_cached_key(|skill| (skill.name.to_lowercase(), skill.key.to_lowercase()));
        Ok(report)
    }
}

#[cfg(test)]
#[path = "discovery_cache_tests.rs"]
mod tests;
