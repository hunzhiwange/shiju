//! Singleton prompt application; publication stays with the prompt domain owner.
use std::collections::BTreeSet;
use vw_api_types::project_profiles::{ProjectProfilePayload, PROJECT_PROFILE_NO_ACTIVE_PROMPT};
use super::snapshot::SnapshotScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptApplyFailure {
    Missing,
    Denied,
    Unsupported,
    Conflict,
    Failed,
}

/// Scoped observation; revision and native preimage travel together to the writer.
/// The executor must authorize even a no-op and reject inconsistent native state.
pub struct PromptObservation {
    pub revision: u64,
    pub active_id: Option<String>,
    pub ids: BTreeSet<String>,
    pub live: Option<Vec<u8>>,
}

/// None means an uncaptured slot, with no read or write. Some(Ok(())) also covers
/// an already effective selection. A missing target never clears the active one.
/// The writer must use mutually exclusive activation or checked deactivation,
/// revalidating this observation and physical-slot ownership under domain locks.
pub fn apply_prompt<R, W>(
    scope: &SnapshotScope<'_>,
    payload: &ProjectProfilePayload,
    read: R,
    write: W,
) -> Option<Result<(), PromptApplyFailure>>
where
    R: FnOnce(&SnapshotScope<'_>) -> Result<PromptObservation, PromptApplyFailure>,
    W: FnOnce(&SnapshotScope<'_>, Option<&str>, &PromptObservation) -> Result<(), PromptApplyFailure>,
{
    let id = payload.prompts.get(scope.group).as_deref()?;
    Some((|| {
        if scope.target_host_id.trim().is_empty() {
            return Err(PromptApplyFailure::Denied);
        }
        let before = read(scope)?;
        if before.active_id.as_ref().is_some_and(|active| !before.ids.contains(active)) {
            return Err(PromptApplyFailure::Conflict);
        }
        let desired = if id == PROJECT_PROFILE_NO_ACTIVE_PROMPT {
            None
        } else {
            if !before.ids.contains(id) {
                return Err(PromptApplyFailure::Missing);
            }
            Some(id)
        };
        if before.active_id.as_deref() == desired {
            return Ok(());
        }
        write(scope, desired, &before)
    })())
}
