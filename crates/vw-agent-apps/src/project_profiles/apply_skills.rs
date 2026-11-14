//! Skill enablement orchestration over executor-authorized observations.
use std::collections::{BTreeMap, BTreeSet};

use vw_api_types::project_profiles::{ProjectProfilePayload, ProjectProfileScope};

use super::snapshot::SnapshotScope;

/// Stable diagnostics contain neither filesystem paths nor native error payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillApplyFailure {
    Missing,
    Denied,
    Unsupported,
    Conflict,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillApplyItem {
    pub installation_id: String,
    pub enabled: bool,
    pub result: Result<(), SkillApplyFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsApplyResult {
    pub group: ProjectProfileScope,
    pub captured: bool,
    pub items: Vec<SkillApplyItem>,
}

/// Read actual enablement in this project/host/group, then dispatch only changes.
/// Readers must reject unavailable or inconsistent native state, not return an
/// empty map. Keys are opaque profile skill references resolved by the executor.
/// The toggle must recheck authorization and optimistic versions for every item.
/// No downloads, installs, snapshot writes or selection pointer updates occur here.
pub fn apply_skills<R, T>(
    scope: &SnapshotScope<'_>,
    payload: &ProjectProfilePayload,
    read: R,
    mut toggle: T,
) -> Result<SkillsApplyResult, SkillApplyFailure>
where
    R: FnOnce(&SnapshotScope<'_>) -> Result<BTreeMap<String, bool>, SkillApplyFailure>,
    T: FnMut(&SnapshotScope<'_>, &str, bool) -> Result<(), SkillApplyFailure>,
{
    let mut result = SkillsApplyResult {
        group: scope.group,
        captured: false,
        items: Vec::new(),
    };
    let Some(references) = payload.skills.get(scope.group) else {
        return Ok(result);
    };
    result.captured = true;
    if scope.target_host_id.trim().is_empty() {
        return Err(SkillApplyFailure::Denied);
    }
    let desired: BTreeSet<&str> = references.iter()
        .map(|reference| reference.installation_id.as_str()).collect();
    if desired.iter().any(|id| id.trim().is_empty()) {
        return Err(SkillApplyFailure::Missing);
    }
    let current = read(scope)?;
    let all: BTreeSet<&str> = current.keys().map(String::as_str)
        .chain(desired.iter().copied()).collect();
    for id in all {
        let enabled = desired.contains(id);
        let outcome = match current.get(id) {
            None => Err(SkillApplyFailure::Missing),
            Some(actual) if *actual == enabled => continue,
            Some(_) => toggle(scope, id, enabled),
        };
        result.items.push(SkillApplyItem {
            installation_id: id.to_owned(), enabled, result: outcome,
        });
    }
    Ok(result)
}
