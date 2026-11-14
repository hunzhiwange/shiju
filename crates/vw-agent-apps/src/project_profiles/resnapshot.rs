//! Replace one complete group without changing shared metadata or other groups.
use anyhow::{Context, Result, ensure};
use vw_api_types::project_profiles::{ProjectProfile, ProjectProfilePayload, ProjectProfileScope};

/// Called by the storage owner inside its version-checked transaction, with a
/// complete snapshot from `snapshot_current` and executor-owned time. The scope
/// is mandatory; no default group or empty-payload recovery is supported.
pub fn merge_snapshot(
    profile: &mut ProjectProfile,
    snapshot: &ProjectProfilePayload,
    scope: ProjectProfileScope,
    expected_version: u64,
    updated_at: i64,
) -> Result<()> {
    ensure!(profile.version == expected_version, "project profile version conflict");
    let version = expected_version.checked_add(1).context("profile version exhausted")?;
    let (Some(provider), Some(mcp), Some(skills), Some(_prompt)) = (
        snapshot.providers.get(scope),
        snapshot.mcp.get(scope),
        snapshot.skills.get(scope),
        snapshot.prompts.get(scope),
    ) else {
        anyhow::bail!("incomplete project profile resnapshot");
    };
    ensure!(
        !provider.trim().is_empty()
            && !mcp.iter().any(|id| id.trim().is_empty())
            && !skills.iter().any(|skill| skill.installation_id.trim().is_empty()),
        "invalid project profile resnapshot reference"
    );
    profile.payload.merge_scope_from(snapshot, scope);
    profile.updated_at = Some(updated_at);
    profile.version = version;
    Ok(())
}
