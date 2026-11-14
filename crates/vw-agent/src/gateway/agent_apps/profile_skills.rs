//! Bridge opaque profile skill references to the existing distribution owner.
use std::{collections::{BTreeMap, BTreeSet}, path::Path};

use vw_agent_apps::{
    project_profiles::{
        apply_skills::{apply_skills, SkillApplyFailure, SkillsApplyResult},
        snapshot::SnapshotScope,
    },
    skills::bulk_sync::SkillSyncTarget,
};
use vw_api_types::project_profiles::ProjectProfilePayload;

use crate::gateway::agent_apps_skills_store::SkillInstallsStore;

/// Resolved on the authorized executor, never deserialized from request paths.
/// A profile reference identifies a skill; the distribution installation ID
/// identifies an application instance. They must not be interchanged.
pub(crate) struct ResolvedProfileSkill {
    pub directory: String,
    pub enabled: bool,
}

/// The caller authorizes project, host, app instance and every skill, observes
/// native truth under the outer profile operation lock, and supplies only this
/// group's resolved references. Missing/ambiguous references are not synthesized.
/// Other domain writes must respect the existing store's version checks.
pub(crate) fn apply_resolved_skills(
    store: &SkillInstallsStore,
    scope: &SnapshotScope<'_>,
    payload: &ProjectProfilePayload,
    ssot: &Path,
    target: &SkillSyncTarget,
    skills: &BTreeMap<String, ResolvedProfileSkill>,
) -> Result<SkillsApplyResult, SkillApplyFailure> {
    if payload.skills.get(scope.group).is_some() {
        if target.path.app_id != scope.group.app_id() {
            return Err(SkillApplyFailure::Denied);
        }
        let mut directories = BTreeSet::new();
        if skills.values().any(|skill| !directories.insert(&skill.directory)) {
            return Err(SkillApplyFailure::Conflict);
        }
        target.identity().validate().map_err(|_| SkillApplyFailure::Unsupported)?;
        target.path.require_distribution().map_err(|_| SkillApplyFailure::Unsupported)?;
    }
    apply_skills(scope, payload,
        |_| Ok(skills.iter().map(|(id, skill)| (id.clone(), skill.enabled)).collect()),
        |scope, id, enabled| {
            let skill = skills.get(id).ok_or(SkillApplyFailure::Missing)?;
            // Refresh after each commit/failure; never reuse a previous item's version.
            let state = store.get(scope.project, scope.target_host_id, ssot)
                .map_err(|_| SkillApplyFailure::Failed)?;
            let attempt = if enabled {
                store.enable_with_method(scope.project, scope.target_host_id, state.version,
                    ssot, &target.installation_id, &target.path, &skill.directory, state.storage_mode)
            } else {
                store.disable_deploy(scope.project, scope.target_host_id, state.version,
                    ssot, &target.installation_id, &target.path, &skill.directory)
            };
            attempt.map(|_| ()).map_err(|_| SkillApplyFailure::Failed)
        })
}
