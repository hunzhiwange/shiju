//! Capture before inserting; creation never selects or applies a profile.
use vw_api_types::{
    app_providers::mode_state::ProviderModeStatus,
    project_profiles::{ProjectProfile, ProjectProfileId, ProjectProfileSkillRef},
};

use super::snapshot::{SnapshotError, SnapshotReadFailure, SnapshotScope, snapshot_current};

#[derive(Debug)]
pub enum CreateError<E> {
    EmptyName,
    Snapshot(SnapshotError),
    Storage(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CreateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => f.write_str("empty project profile name"),
            Self::Snapshot(error) => error.fmt(f),
            // Storage errors may carry backend payloads; do not expose them.
            Self::Storage(_) => f.write_str("project profile storage failed"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CreateError<E> {}

/// The executor supplies authorized readers and an atomic insert-only persistence
/// callback (store.save with expected_version=None). No apply/selection callback
/// is accepted. A failed read never reaches persistence. The callback must not
/// publish a record before its transaction commits.
pub fn create_from_current<P, M, S, R, W, E>(
    name: &str,
    scope: &SnapshotScope<'_>,
    read_provider: P,
    read_mcp: M,
    read_skills: S,
    read_prompt: R,
    insert: W,
) -> Result<ProjectProfile, CreateError<E>>
where
    P: FnOnce(&SnapshotScope<'_>) -> Result<ProviderModeStatus, SnapshotReadFailure>,
    M: FnOnce(&SnapshotScope<'_>) -> Result<Vec<String>, SnapshotReadFailure>,
    S: FnOnce(&SnapshotScope<'_>) -> Result<Vec<ProjectProfileSkillRef>, SnapshotReadFailure>,
    R: FnOnce(&SnapshotScope<'_>) -> Result<Option<String>, SnapshotReadFailure>,
    W: FnOnce(ProjectProfile) -> Result<ProjectProfile, E>,
{
    let name = name.trim();
    if name.is_empty() {
        return Err(CreateError::EmptyName);
    }
    let payload = snapshot_current(scope, read_provider, read_mcp, read_skills, read_prompt)
        .map_err(CreateError::Snapshot)?;
    let profile = ProjectProfile {
        id: ProjectProfileId(uuid::Uuid::new_v4().to_string()),
        name: name.to_owned(),
        version: 1,
        updated_at: None,
        payload,
    };
    insert(profile).map_err(CreateError::Storage)
}
