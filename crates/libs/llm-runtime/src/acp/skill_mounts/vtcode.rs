//! Expose the request-owned snapshot only to the reviewed read-only loader.
use super::super::{
    Error, acp_option_error, host::SkillLease, model_source::profile::AcpLaunchProfile,
    to_api_error,
};
use agent_config::schema::AcpAgentConfig;
use std::path::{Path, PathBuf};

const ENABLED: &str = "SHIJU_VTCODE_READ_ONLY_SKILLS";
const SELECTED: &str = "SHIJU_VTCODE_SELECTED_SKILLS";
const SANDBOX: &str = "SHIJU_VTCODE_SANDBOX";
const SNAPSHOT: &str = "vtcode-selected-skills";

pub(in crate::acp) fn configure(
    profile: &AcpLaunchProfile,
    config: &mut AcpAgentConfig,
    enabled: bool,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    if !enabled {
        return Ok(());
    }
    if let Err(error) = acp_agent_registry::verify_vtcode_skill_executable(&config.command) {
        return if view.is_some() { Err(acp_option_error(error)) } else { Ok(()) };
    }
    if let Some(root) = publish_snapshot(profile.directory(), view)? {
        let policy =
            std::fs::read_to_string(profile.directory().join("vtcode.sb")).map_err(to_api_error)?;
        let policy = with_snapshot_policy(&policy, &root).map_err(to_api_error)?;
        let policy =
            profile.write_config("vtcode-skills.sb", policy.as_bytes()).map_err(to_api_error)?;
        config.env.insert(SANDBOX.into(), policy.to_string_lossy().into_owned());
        config.env.insert(SELECTED.into(), root.to_string_lossy().into_owned());
    }
    config.env.insert(ENABLED.into(), "1".into());
    Ok(())
}

fn publish_snapshot(
    directory: &Path,
    view: Option<&impl SkillLease>,
) -> Result<Option<PathBuf>, Error> {
    let Some(view) = view else {
        return Ok(None);
    };
    let root = directory.canonicalize().map_err(to_api_error)?.join(SNAPSHOT);
    view.snapshot(&root)?;
    Ok(Some(root))
}

fn with_snapshot_policy(policy: &str, root: &Path) -> anyhow::Result<String> {
    let path = root.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 VT Code skill path"))?;
    anyhow::ensure!(!path.chars().any(char::is_control), "Invalid VT Code skill path");
    let path = serde_json::to_string(path)?;
    Ok(format!(
        "{policy}\n(allow file-read-data (subpath {path}))\n(deny file-write* (subpath {path}))\n"
    ))
}

#[cfg(test)]
#[path = "vtcode_tests.rs"]
mod tests;
