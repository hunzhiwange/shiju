//! AutoHand's patched ACP runtime receives a private, validated skill snapshot.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use std::{collections::HashMap, path::Path};

pub(in crate::acp) const MANAGED: &str = "SHIJU_AUTOHAND_MANAGED";
pub(in crate::acp) const WORKSPACE: &str = "SHIJU_AUTOHAND_WORKSPACE";
pub(in crate::acp) const ROOT: &str = "SHIJU_AUTOHAND_SKILL_ROOT";

pub(in crate::acp) fn matches_managed_entry(command: &str) -> bool {
    let Ok(Some((entry, _))) = acp_agent_registry::reviewed_agent_command("autohand") else {
        return false;
    };
    Path::new(command)
        .canonicalize()
        .is_ok_and(|command| Path::new(&entry).canonicalize().is_ok_and(|entry| command == entry))
}

pub(in crate::acp) fn configure(
    profile: &mut AcpLaunchProfile,
    workspace: &Path,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let path =
        workspace.to_str().ok_or_else(|| to_api_error("AutoHand workspace path is not UTF-8"))?;
    let mut env = HashMap::from([
        (MANAGED.into(), "1".into()),
        (WORKSPACE.into(), path.into()),
        (ROOT.into(), String::new()),
    ]);
    if let Some(view) = view {
        let snapshot = profile.directory().join("shiju-session-skills-autohand");
        view.snapshot(&snapshot)?;
        let snapshot = snapshot.canonicalize().map_err(to_api_error)?;
        let path =
            snapshot.to_str().ok_or_else(|| to_api_error("AutoHand skill path is not UTF-8"))?;
        env.insert(ROOT.into(), path.into());
    }
    profile.overlay_environment(env).map_err(to_api_error)
}

#[cfg(test)]
#[path = "autohand_tests.rs"]
mod tests;
