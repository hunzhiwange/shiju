//! Prepare a switch before applying any target configuration.
use anyhow::{Result, ensure};
use vw_api_types::project_profiles::{ProjectProfile, ProjectProfileId, ProjectProfileScope};

/// Warning codes contain no backend errors, native payloads or credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutosaveWarning {
    PreviousProfileSaveFailed,
}

#[derive(Debug)]
pub struct PreparedProfileSwitch {
    pub target: ProjectProfile,
    pub warning: Option<AutosaveWarning>,
}

/// The storage owner must load/parse the target before calling this function.
/// Null categories and an uncaptured group are valid (leave configuration alone).
/// The callback snapshots and version-checks only the previous profile's group.
/// Same-profile and first selections never invoke it. Authorization/current-slot
/// lookup failures belong to the caller and must not be downgraded to warnings.
pub fn prepare_switch<E>(
    target: ProjectProfile,
    current: Option<&ProjectProfileId>,
    group: ProjectProfileScope,
    save_previous: impl FnOnce(&ProjectProfileId) -> Result<(), E>,
) -> Result<PreparedProfileSwitch> {
    ensure!(!target.id.0.trim().is_empty(), "invalid target profile id");
    let payload = &target.payload;
    ensure!(
        payload.providers.get(group).as_ref().is_none_or(|id| !id.trim().is_empty())
            && payload.mcp.get(group).as_ref().is_none_or(|ids|
                ids.iter().all(|id| !id.trim().is_empty()))
            && payload.skills.get(group).as_ref().is_none_or(|skills|
                skills.iter().all(|skill| !skill.installation_id.trim().is_empty()))
            && payload.prompts.get(group).as_ref().is_none_or(|id|
                id.is_empty() || !id.trim().is_empty()),
        "invalid target profile reference"
    );
    let warning = match current {
        Some(previous) if previous != &target.id => save_previous(previous)
            .err().map(|_| AutosaveWarning::PreviousProfileSaveFailed),
        _ => None,
    };
    Ok(PreparedProfileSwitch { target, warning })
}
