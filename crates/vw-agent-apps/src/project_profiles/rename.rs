//! Rename metadata without replacing snapshots or touching selection records.

use anyhow::{Context, Result, ensure};
use vw_api_types::project_profiles::ProjectProfile;

/// The storage owner calls this inside its tenant transaction and supplies time
/// from the executor clock. Validate everything before changing the entity.
pub fn rename(
    profile: &mut ProjectProfile,
    name: &str,
    expected_version: u64,
    updated_at: i64,
) -> Result<()> {
    let name = name.trim();
    ensure!(!name.is_empty(), "empty project profile name");
    ensure!(profile.version == expected_version, "project profile version conflict");
    let version = expected_version.checked_add(1).context("profile version exhausted")?;
    profile.name = name.to_owned();
    profile.updated_at = Some(updated_at);
    profile.version = version;
    Ok(())
}
