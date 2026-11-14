use std::{sync::{Arc, Barrier}, time::{Duration, Instant}};

use super::super::{discovery::{DiscoverableSkill, DiscoveryError, SkillDiscoveryResult, SkillRepoFailure}, repos::SkillRepo};
use super::{CacheError, SkillDiscoveryCache, key};

fn repo(name: &str) -> SkillRepo {
    SkillRepo { owner: "org".into(), name: name.into(), branch: "main".into(), enabled: true }
}

fn result(name: &str) -> SkillDiscoveryResult {
    SkillDiscoveryResult { skills: vec![DiscoverableSkill {
        key: "org/repo:skill".into(), name: name.into(), description: String::new(),
        directory: "skill".into(), readme_url: None, repo_owner: "org".into(),
        repo_name: "repo".into(), repo_branch: "main".into(),
    }], failures: vec![] }
}

#[test]
fn cp12_discovery_cache_success() {
    let cache = SkillDiscoveryCache::default();
    cache.update_repositories(1, vec![repo("repo")]).unwrap();
    let now = Instant::now();
    let first = cache.begin(false, now).unwrap();
    cache.finish(&key(&first[0].0), first[0].1, result("first"), now).unwrap();
    assert!(cache.begin(false, now + Duration::from_secs(599)).unwrap().is_empty());
    let forced = cache.begin(true, now).unwrap();
    cache.finish(&key(&forced[0].0), forced[0].1, result("fresh"), now).unwrap();
    cache.finish(&key(&first[0].0), first[0].1, result("stale"), now).unwrap();
    assert_eq!(cache.snapshot().unwrap(), result("fresh"));
    assert_eq!(cache.begin(false, now + Duration::from_secs(600)).unwrap().len(), 1);
}

#[test]
fn cp12_discovery_cache_rejects_without_side_effects() {
    let cache = SkillDiscoveryCache::default();
    assert_eq!(cache.snapshot(), Err(CacheError::NotInitialized));
    assert_eq!(cache.begin(false, Instant::now()), Err(CacheError::NotInitialized));
    cache.update_repositories(5, vec![repo("repo")]).unwrap();
    let now = Instant::now();
    let request = cache.begin(false, now).unwrap().remove(0);
    cache.finish(&key(&request.0), request.1, result("saved"), now).unwrap();
    assert_eq!(cache.update_repositories(4, vec![]), Err(CacheError::StaleConfiguration));
    assert_eq!(cache.update_repositories(5, vec![]), Err(CacheError::ConflictingConfiguration));
    assert_eq!(cache.update_repositories(6, vec![repo("../invalid")]), Err(CacheError::InvalidConfiguration));
    assert_eq!(cache.snapshot().unwrap(), result("saved"));
}

#[test]
fn cp12_discovery_cache_removed_repository_cannot_be_resurrected() {
    let cache = Arc::new(SkillDiscoveryCache::default());
    cache.update_repositories(1, vec![repo("repo"), repo("other")]).unwrap();
    let now = Instant::now();
    let requests = cache.begin(false, now).unwrap();
    let request = requests.into_iter().find(|(repo, _)| repo.name == "repo").unwrap();
    let started = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker = {
        let cache = cache.clone();
        let started = started.clone();
        let release = release.clone();
        std::thread::spawn(move || {
            started.wait();
            release.wait();
            cache.finish(&key(&request.0), request.1, result("deleted"), now).unwrap();
        })
    };
    started.wait();
    cache.update_repositories(2, vec![repo("other")]).unwrap();
    cache.update_repositories(3, vec![repo("repo"), repo("other")]).unwrap();
    release.wait();
    worker.join().unwrap();
    assert!(cache.snapshot().unwrap().skills.is_empty());
}

#[test]
fn cp12_discovery_cache_configuration_preserves_only_unchanged_partitions() {
    let cache = SkillDiscoveryCache::default();
    cache.update_repositories(1, vec![repo("repo")]).unwrap();
    let now = Instant::now();
    let request = cache.begin(false, now).unwrap().remove(0);
    cache.finish(&key(&request.0), request.1, result("saved"), now).unwrap();
    cache.update_repositories(2, vec![repo("repo"), repo("new")]).unwrap();
    assert_eq!(cache.snapshot().unwrap(), result("saved"));
    let requests = cache.begin(false, now).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0.name, "new");
    let mut changed = repo("repo");
    changed.branch = "release".into();
    cache.update_repositories(3, vec![changed]).unwrap();
    cache.finish(&key(&request.0), request.1, result("old branch"), now).unwrap();
    assert!(cache.snapshot().unwrap().skills.is_empty());
}

#[test]
fn cp12_discovery_cache_new_failure_prevents_old_success_publication() {
    let cache = SkillDiscoveryCache::default();
    cache.update_repositories(1, vec![repo("repo")]).unwrap();
    let now = Instant::now();
    let old = cache.begin(false, now).unwrap().remove(0);
    let new = cache.begin(true, now).unwrap().remove(0);
    let failure = SkillDiscoveryResult { skills: vec![], failures: vec![SkillRepoFailure {
        owner: "org".into(), name: "repo".into(), branch: "main".into(), error: DiscoveryError::Transport,
    }] };
    cache.finish(&key(&new.0), new.1, failure.clone(), now).unwrap();
    cache.finish(&key(&old.0), old.1, result("stale"), now).unwrap();
    assert_eq!(cache.snapshot().unwrap(), failure);
    assert_eq!(cache.begin(false, now).unwrap().len(), 1);
}
