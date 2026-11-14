//! Read one group without publishing partial or fabricated snapshots.
use vw_api_types::{
    app_providers::mode_state::ProviderModeStatus,
    id::ProjectScope,
    project_profiles::{
        PROJECT_PROFILE_NO_ACTIVE_PROMPT, ProjectProfileCategory, ProjectProfilePayload,
        ProjectProfileScope, ProjectProfileSkillRef,
    },
};

/// Executor-owned context, assembled after host/project and domain authorization.
/// Passing a host name is not authorization and never permits local fallback.
pub struct SnapshotScope<'a> {
    pub project: &'a ProjectScope,
    pub target_host_id: &'a str,
    pub group: ProjectProfileScope,
}

/// Stable errors deliberately exclude native payloads and credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotReadFailure {
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    InvalidReference,
    NoActiveProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotError {
    pub category: ProjectProfileCategory,
    pub reason: SnapshotReadFailure,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "project profile snapshot {:?}: {:?}", self.category, self.reason)
    }
}

impl std::error::Error for SnapshotError {}

/// Invoke each authorized domain reader exactly once for the same host/group.
/// Readers must inspect effective state: provider mode observation, native MCP
/// bindings, deployed skill installations and the scoped active prompt reference.
/// They must return errors for unreadable/unmanaged state, unsupported capabilities,
/// stale projections or remote hosts without an executor. Empty collections and
/// None prompt are successful observations only, never error fallbacks.
///
/// Readers are lazy so a failed category stops the operation. Nothing is persisted
/// here; callers may save only the complete returned payload on the execution host.
/// Native observations are sequential, not an atomic transaction across domains.
pub fn snapshot_current<P, M, S, R>(
    scope: &SnapshotScope<'_>,
    read_provider: P,
    read_mcp: M,
    read_skills: S,
    read_prompt: R,
) -> Result<ProjectProfilePayload, SnapshotError>
where
    P: FnOnce(&SnapshotScope<'_>) -> Result<ProviderModeStatus, SnapshotReadFailure>,
    M: FnOnce(&SnapshotScope<'_>) -> Result<Vec<String>, SnapshotReadFailure>,
    S: FnOnce(&SnapshotScope<'_>) -> Result<Vec<ProjectProfileSkillRef>, SnapshotReadFailure>,
    R: FnOnce(&SnapshotScope<'_>) -> Result<Option<String>, SnapshotReadFailure>,
{
    use ProjectProfileCategory::{Mcp, Prompt, Provider, Skill};
    let error = |category, reason| SnapshotError { category, reason };
    if scope.target_host_id.trim().is_empty() {
        return Err(error(Provider, SnapshotReadFailure::Denied));
    }
    let provider = read_provider(scope).map_err(|reason| error(Provider, reason))?;
    // Direct selection is not the effective route while proxy mode is selected.
    let provider = provider.in_use_provider
        .ok_or_else(|| error(Provider, SnapshotReadFailure::NoActiveProvider))?;
    if provider.trim().is_empty() {
        return Err(error(Provider, SnapshotReadFailure::InvalidReference));
    }
    let mut mcp = read_mcp(scope).map_err(|reason| error(Mcp, reason))?;
    if mcp.iter().any(|id| id.trim().is_empty()) {
        return Err(error(Mcp, SnapshotReadFailure::InvalidReference));
    }
    mcp.sort();
    mcp.dedup();
    let mut skills = read_skills(scope).map_err(|reason| error(Skill, reason))?;
    if skills.iter().any(|skill| skill.installation_id.trim().is_empty()) {
        return Err(error(Skill, SnapshotReadFailure::InvalidReference));
    }
    skills.sort_by(|a, b| a.installation_id.cmp(&b.installation_id));
    skills.dedup_by(|a, b| a.installation_id == b.installation_id);
    let prompt = read_prompt(scope).map_err(|reason| error(Prompt, reason))?;
    if prompt.as_ref().is_some_and(|id| id.trim().is_empty()) {
        return Err(error(Prompt, SnapshotReadFailure::InvalidReference));
    }
    let mut payload = ProjectProfilePayload::default();
    *payload.providers.get_mut(scope.group) = Some(provider);
    *payload.mcp.get_mut(scope.group) = Some(mcp);
    *payload.skills.get_mut(scope.group) = Some(skills);
    *payload.prompts.get_mut(scope.group) =
        Some(prompt.unwrap_or_else(|| PROJECT_PROFILE_NO_ACTIVE_PROMPT.to_owned()));
    Ok(payload)
}
